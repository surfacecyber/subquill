use std::time::{Duration, Instant};

use crate::bilibili::SubtitleSegment;
use crate::llm::{LlmClient, LlmError};

use super::chunk::{ends_complete_sentence, join_segments_for_analysis, plan_chunks};
use super::error::{NoteError, NoteResult};
use super::markdown::{build_display_title, escape_heading_text};
use super::merge::{fallback_merge_summaries, merge_chunk_outputs, parse_summary_merge_output};
use super::parse_md::parse_chunk_output;
use super::prompt::{chunk_messages, summary_merge_fix_messages, summary_merge_messages};
use super::types::{
    ChunkLlmOutput, NoteData, NoteLocale, NoteProgress, NoteProgressStage, NoteSection,
    SubtitleChunk, VideoMetadata,
};

const DRAFT_EMIT_INTERVAL: Duration = Duration::from_millis(80);
const COVER_SLACK_MS: u64 = 90_000;
const MAX_CONTINUATIONS: usize = 8;

pub fn generate_note_data<T: crate::llm::HttpTransport>(
    client: &LlmClient<T>,
    metadata: VideoMetadata,
    segments: Vec<SubtitleSegment>,
    locale: NoteLocale,
    mut cancellation_check: impl FnMut() -> bool,
    mut progress: impl FnMut(NoteProgress),
) -> NoteResult<NoteData> {
    if cancellation_check() {
        return Err(NoteError::Cancelled);
    }

    let duration_ms = metadata.duration_ms;
    let chunks = plan_chunks(&join_segments_for_analysis(&segments));
    let chunk_count = chunks.len().max(1);
    let full_pass = chunks.len() == 1;

    progress(NoteProgress::new(
        NoteProgressStage::Chunking,
        None,
        chunk_count,
    ));

    if chunks.is_empty() {
        return Err(NoteError::Llm(LlmError::invalid_response(
            "no subtitle segments to analyze",
        )));
    }

    let mut chunk_outputs = Vec::with_capacity(chunks.len());
    let mut completed_drafts: Vec<String> = Vec::new();
    let mut last_draft = String::new();

    for chunk in &chunks {
        if cancellation_check() {
            return Err(NoteError::Cancelled);
        }

        progress(NoteProgress::new(
            NoteProgressStage::AnalyzingChunk,
            Some(chunk.index),
            chunk_count,
        ));

        let mut pump = DraftPump::new();
        let output = analyze_chunk(
            client,
            &metadata,
            chunk,
            locale,
            duration_ms,
            full_pass,
            &mut cancellation_check,
            &mut |content, thinking| {
                last_draft = compose_draft(
                    &metadata,
                    locale,
                    &completed_drafts,
                    chunk.index,
                    chunk_count,
                    content,
                );
                pump.emit(
                    &mut progress,
                    NoteProgressStage::AnalyzingChunk,
                    Some(chunk.index),
                    chunk_count,
                    &last_draft,
                    thinking,
                    false,
                );
            },
        )?;
        pump.emit(
            &mut progress,
            NoteProgressStage::AnalyzingChunk,
            Some(chunk.index),
            chunk_count,
            &last_draft,
            false,
            true,
        );
        if cancellation_check() {
            return Err(NoteError::Cancelled);
        }
        completed_drafts.push(output.raw_markdown.clone());
        chunk_outputs.push(output.parsed);
    }

    let (sections, partial_summaries) = merge_chunk_outputs(&chunk_outputs);

    let summary = if chunk_outputs.len() <= 1 {
        chunk_outputs
            .first()
            .map(|o| o.summary.clone())
            .unwrap_or_default()
    } else {
        if cancellation_check() {
            return Err(NoteError::Cancelled);
        }

        progress(
            NoteProgress::new(NoteProgressStage::MergingSummaries, None, chunk_count)
                .with_draft(last_draft.clone(), false),
        );

        let merged = merge_summaries(client, &partial_summaries, locale)?;
        if cancellation_check() {
            return Err(NoteError::Cancelled);
        }
        merged
    };

    if cancellation_check() {
        return Err(NoteError::Cancelled);
    }

    progress(
        NoteProgress::new(NoteProgressStage::Complete, None, chunk_count)
            .with_draft(last_draft, false),
    );

    Ok(NoteData {
        metadata,
        summary,
        sections,
        segments,
    })
}

struct ChunkAnalysis {
    parsed: super::types::ChunkLlmOutput,
    raw_markdown: String,
}

fn analyze_chunk<T: crate::llm::HttpTransport>(
    client: &LlmClient<T>,
    metadata: &VideoMetadata,
    chunk: &SubtitleChunk,
    locale: NoteLocale,
    video_duration_ms: u64,
    full_pass: bool,
    cancellation_check: &mut impl FnMut() -> bool,
    on_draft: &mut impl FnMut(&str, bool),
) -> NoteResult<ChunkAnalysis> {
    let mut work = chunk.clone();
    let mut sections: Vec<NoteSection> = Vec::new();
    let mut summaries: Vec<String> = Vec::new();
    let mut raw_parts: Vec<String> = Vec::new();
    let mut from_ms = chunk.start_ms;

    for pass in 0..=MAX_CONTINUATIONS {
        if cancellation_check() {
            return Err(NoteError::Cancelled);
        }
        let once = analyze_once(
            client,
            metadata,
            &work,
            locale,
            video_duration_ms,
            full_pass && pass == 0,
            cancellation_check,
            on_draft,
        )?;
        raw_parts.push(once.raw_markdown);
        if !once.parsed.summary.trim().is_empty() {
            summaries.push(once.parsed.summary);
        }

        let mut produced = once.parsed.sections;
        let last = produced.last().cloned();
        let incomplete = last
            .as_ref()
            .is_some_and(|section| !ends_complete_sentence(&section.explanation));
        let redo_cut = incomplete && produced.len() > 1;
        if redo_cut {
            produced.pop();
        }
        sections.extend(produced);

        let next_from = match last {
            Some(cut) if redo_cut => cut.start_ms,
            Some(cut) if incomplete => cut.end_ms,
            Some(cut) => cut.end_ms,
            None => from_ms,
        };

        if chunk.end_ms.saturating_sub(next_from) <= COVER_SLACK_MS {
            break;
        }
        if next_from <= from_ms {
            break;
        }
        let Some(rest) = slice_chunk(chunk, next_from) else {
            break;
        };
        from_ms = next_from;
        work = rest;
    }

    let summary = if summaries.len() <= 1 {
        summaries.into_iter().next().unwrap_or_default()
    } else {
        merge_summaries(client, &summaries, locale)?
    };

    Ok(ChunkAnalysis {
        parsed: ChunkLlmOutput {
            summary,
            sections,
        },
        raw_markdown: raw_parts.join("\n\n"),
    })
}

fn analyze_once<T: crate::llm::HttpTransport>(
    client: &LlmClient<T>,
    metadata: &VideoMetadata,
    chunk: &SubtitleChunk,
    locale: NoteLocale,
    video_duration_ms: u64,
    full_pass: bool,
    cancellation_check: &mut impl FnMut() -> bool,
    on_draft: &mut impl FnMut(&str, bool),
) -> NoteResult<ChunkAnalysis> {
    let messages = chunk_messages(metadata, chunk, locale, full_pass);
    let content = client
        .chat_stream(messages, |event| on_draft(&event.content, event.thinking), || {
            cancellation_check()
        })
        .map_err(map_llm_error)?;

    let parsed = parse_chunk_output(&content, chunk, video_duration_ms).map_err(NoteError::Llm)?;
    Ok(ChunkAnalysis {
        parsed,
        raw_markdown: content,
    })
}

fn slice_chunk(chunk: &SubtitleChunk, from_ms: u64) -> Option<SubtitleChunk> {
    let segments: Vec<SubtitleSegment> = chunk
        .segments
        .iter()
        .filter(|segment| segment.end_ms > from_ms)
        .cloned()
        .collect();
    if segments.is_empty() {
        return None;
    }
    Some(SubtitleChunk {
        index: chunk.index,
        start_ms: segments[0].start_ms.max(from_ms),
        end_ms: chunk.end_ms,
        segments,
    })
}

fn merge_summaries<T: crate::llm::HttpTransport>(
    client: &LlmClient<T>,
    partials: &[String],
    locale: NoteLocale,
) -> NoteResult<String> {
    let messages = summary_merge_messages(partials, locale);
    let content = client.chat(messages).map_err(map_llm_error)?;

    match parse_summary_merge_output(&content) {
        Ok(summary) => Ok(summary),
        Err(first_err) if is_invalid_response(&first_err) => {
            let reason = first_err.to_string();
            let fix_messages = summary_merge_fix_messages(partials, locale, &content, &reason);
            let fixed_content = match client.chat(fix_messages) {
                Ok(text) => text,
                Err(retry_err) if is_invalid_response(&retry_err) => {
                    return Ok(fallback_merge_summaries(partials, locale));
                }
                Err(retry_err) => return Err(map_llm_error(retry_err)),
            };

            parse_summary_merge_output(&fixed_content)
                .or_else(|_| Ok(fallback_merge_summaries(partials, locale)))
        }
        Err(err) => Err(NoteError::Llm(err)),
    }
}

fn map_llm_error(err: LlmError) -> NoteError {
    if err.code() == "JOB_CANCELLED" {
        NoteError::Cancelled
    } else {
        NoteError::Llm(err)
    }
}

fn is_invalid_response(err: &LlmError) -> bool {
    err.code() == "LLM_INVALID_RESPONSE"
}

fn compose_draft(
    metadata: &VideoMetadata,
    locale: NoteLocale,
    completed: &[String],
    current_index: usize,
    chunk_count: usize,
    current: &str,
) -> String {
    let mut out = format!(
        "# {}\n\n",
        escape_heading_text(&build_display_title(metadata))
    );

    if chunk_count > 1 {
        for (index, markdown) in completed.iter().enumerate() {
            out.push_str(&chunk_part_heading(locale, index, chunk_count));
            out.push_str(markdown.trim());
            out.push_str("\n\n");
        }
        out.push_str(&chunk_part_heading(locale, current_index, chunk_count));
    }

    out.push_str(current);
    out
}

fn chunk_part_heading(locale: NoteLocale, index: usize, chunk_count: usize) -> String {
    match locale {
        NoteLocale::Zh => format!("## 第 {} / {} 块\n\n", index + 1, chunk_count),
        NoteLocale::En => format!("## Part {} / {}\n\n", index + 1, chunk_count),
    }
}

struct DraftPump {
    last_emit: Instant,
    last_thinking: bool,
}

impl DraftPump {
    fn new() -> Self {
        Self {
            last_emit: Instant::now() - DRAFT_EMIT_INTERVAL,
            last_thinking: false,
        }
    }

    fn emit(
        &mut self,
        progress: &mut impl FnMut(NoteProgress),
        stage: NoteProgressStage,
        chunk_index: Option<usize>,
        chunk_count: usize,
        draft: &str,
        thinking: bool,
        force: bool,
    ) {
        let thinking_changed = thinking != self.last_thinking;
        if !force && !thinking_changed && self.last_emit.elapsed() < DRAFT_EMIT_INTERVAL {
            return;
        }
        progress(
            NoteProgress::new(stage, chunk_index, chunk_count).with_draft(draft, thinking),
        );
        self.last_emit = Instant::now();
        self.last_thinking = thinking;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::SubtitleSegment;
    use crate::llm::{LlmClientConfig, MockResponse, MockTransport};
    use crate::note::markdown::render_markdown;

    fn success_chat_body(content: &str) -> String {
        serde_json::json!({
            "choices": [{ "message": { "content": content } }]
        })
        .to_string()
    }

    fn segment(start: u64, end: u64, text: &str) -> SubtitleSegment {
        SubtitleSegment {
            start_ms: start,
            end_ms: end,
            text: text.to_string(),
        }
    }

    fn metadata(duration_ms: u64) -> VideoMetadata {
        VideoMetadata {
            title: "Test Video".to_string(),
            part_title: None,
            bvid: "BVTEST".to_string(),
            duration_ms,
        }
    }

    fn client_from_responses(responses: Vec<MockResponse>) -> LlmClient<MockTransport> {
        let (transport, _) = MockTransport::new(responses);
        LlmClient::new(
            LlmClientConfig {
                base_url: "https://api.example.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                model: "test-model".to_string(),
            },
            transport,
        )
        .expect("client")
    }

    fn en_note_md(summary: &str, title: &str, start: &str, end: &str, explanation: &str) -> String {
        format!(
            "## Summary\n\n{summary}\n\n## Section breakdown\n\n### {title} ({start} - {end})\n\n{explanation}\n"
        )
    }

    #[test]
    fn e2e_single_chunk_generation() {
        let chunk_md = en_note_md("overall", "Intro", "00:00:01", "00:00:02", "Says hi");
        let client =
            client_from_responses(vec![MockResponse::success(success_chat_body(&chunk_md))]);

        let segments = vec![segment(1000, 2000, "hello world")];
        let meta = metadata(3000);
        let mut analyze_count = 0usize;
        let mut saw_draft = false;

        let data = generate_note_data(
            &client,
            meta,
            segments.clone(),
            NoteLocale::En,
            || false,
            |p| {
                if p.stage == NoteProgressStage::AnalyzingChunk {
                    analyze_count += 1;
                    assert_eq!(p.chunk_count, 1);
                    if p.draft_markdown.as_ref().is_some_and(|md| md.contains("overall"))
                    {
                        saw_draft = true;
                    }
                }
            },
        )
        .expect("generate");

        assert!(analyze_count >= 1);
        assert!(saw_draft);
        assert_eq!(data.summary, "overall");
        assert_eq!(data.segments, segments);
        assert_eq!(data.sections.len(), 1);

        let md = render_markdown(&data, NoteLocale::En);
        assert!(md.contains("hello world"));
        assert!(md.contains("overall"));
    }

    #[test]
    fn e2e_json_fallback_still_works() {
        let chunk_json = r#"{"summary":"overall","sections":[{"start_ms":1000,"end_ms":2000,"title":"Intro","explanation":"Says hi"}]}"#;
        let client =
            client_from_responses(vec![MockResponse::success(success_chat_body(chunk_json))]);

        let data = generate_note_data(
            &client,
            metadata(3000),
            vec![segment(1000, 2000, "hello world")],
            NoteLocale::En,
            || false,
            |_| {},
        )
        .expect("generate");

        assert_eq!(data.summary, "overall");
        assert_eq!(data.sections[0].title, "Intro");
    }

    #[test]
    fn e2e_multi_chunk_with_summary_merge() {
        let chunk1 = en_note_md("part one", "A", "00:00:01", "00:00:02", "first.");
        let chunk2 = en_note_md("part two", "B", "00:00:05", "00:03:00", "second.");
        let merge = "merged overall";

        let client = client_from_responses(vec![
            MockResponse::success(success_chat_body(&chunk1)),
            MockResponse::success(success_chat_body(&chunk2)),
            MockResponse::success(success_chat_body(merge)),
        ]);

        let segments = vec![
            segment(1000, 2000, "hello"),
            segment(5000, 180_000, "later"),
        ];

        let meta = metadata(180_000);
        let data = generate_note_data(&client, meta, segments, NoteLocale::En, || false, |_| {})
            .expect("generate");

        assert_eq!(data.summary, "merged overall");
        assert_eq!(data.sections.len(), 2);
        assert_eq!(data.sections[0].start_ms, 1000);
        assert_eq!(data.sections[1].start_ms, 5000);
    }

    #[test]
    fn unstructured_prose_still_generates() {
        let client = client_from_responses(vec![MockResponse::success(success_chat_body(
            "just some notes about the clip",
        ))]);

        let data = generate_note_data(
            &client,
            metadata(3000),
            vec![segment(1000, 2000, "x")],
            NoteLocale::En,
            || false,
            |_| {},
        )
        .expect("generate");

        assert!(data.summary.contains("just some notes"));
        assert_eq!(data.sections.len(), 1);
    }

    #[test]
    fn summary_merge_falls_back_when_json_invalid() {
        let chunk1 = r#"{"summary":"part one","sections":[{"start_ms":1000,"end_ms":2000,"title":"A","explanation":"first."}]}"#;
        let chunk2 = r#"{"summary":"part two","sections":[{"start_ms":5000,"end_ms":180000,"title":"B","explanation":"second."}]}"#;

        let client = client_from_responses(vec![
            MockResponse::success(success_chat_body(chunk1)),
            MockResponse::success(success_chat_body(chunk2)),
            MockResponse::success(success_chat_body(r#"{"summary":""}"#)),
            MockResponse::success(success_chat_body(r#"{"summary":""}"#)),
        ]);

        let data = generate_note_data(
            &client,
            metadata(180_000),
            vec![segment(1000, 2000, "hello"), segment(5000, 180_000, "later")],
            NoteLocale::En,
            || false,
            |_| {},
        )
        .expect("generate");

        assert_eq!(data.summary, "part one\n\npart two");
    }

    #[test]
    fn summary_merge_propagates_auth_errors() {
        let chunk1 = r#"{"summary":"part one","sections":[{"start_ms":1000,"end_ms":2000,"title":"A","explanation":"first."}]}"#;
        let chunk2 = r#"{"summary":"part two","sections":[{"start_ms":5000,"end_ms":180000,"title":"B","explanation":"second."}]}"#;

        let client = client_from_responses(vec![
            MockResponse::success(success_chat_body(chunk1)),
            MockResponse::success(success_chat_body(chunk2)),
            MockResponse::success(r#"{"error":{"message":"bad key"}}"#).with_status(401),
        ]);

        let err = generate_note_data(
            &client,
            metadata(180_000),
            vec![segment(1000, 2000, "hello"), segment(5000, 180_000, "later")],
            NoteLocale::En,
            || false,
            |_| {},
        )
        .unwrap_err();

        assert_eq!(err.code(), "LLM_AUTH_ERROR");
    }

    #[test]
    fn cancellation_after_chunk_llm_call() {
        let chunk_md = en_note_md("overall", "Intro", "00:00:01", "00:00:02", "Says hi");
        let client =
            client_from_responses(vec![MockResponse::success(success_chat_body(&chunk_md))]);

        let segments = vec![segment(1000, 2000, "hello world")];
        let meta = metadata(3000);
        let mut cancelled = false;

        let err = generate_note_data(
            &client,
            meta,
            segments,
            NoteLocale::En,
            || {
                if cancelled {
                    return true;
                }
                cancelled = true;
                false
            },
            |_| {},
        )
        .unwrap_err();

        assert_eq!(err.code(), "JOB_CANCELLED");
    }

    #[test]
    fn cancellation_stops_generation() {
        let client = client_from_responses(vec![]);
        let err = generate_note_data(
            &client,
            metadata(1000),
            vec![segment(0, 1000, "a")],
            NoteLocale::En,
            || true,
            |_| {},
        )
        .unwrap_err();
        assert_eq!(err.code(), "JOB_CANCELLED");
    }
}
