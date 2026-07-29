use crate::bilibili::SubtitleSegment;
use crate::llm::{LlmClient, LlmError};

use super::chunk::{plan_chunks, segments_prompt_char_count};
use super::error::{NoteError, NoteResult};
use super::merge::{fallback_merge_summaries, merge_chunk_outputs, parse_summary_merge_output};
use super::prompt::{
    chunk_fix_messages, chunk_messages, summary_merge_fix_messages, summary_merge_messages,
};
use super::types::{
    NoteData, NoteLocale, NoteProgress, NoteProgressStage, SubtitleChunk, VideoMetadata,
};
use super::validate::parse_and_validate_chunk_output;

const MAX_TOKENS_MIN: u32 = 4_096;
const MAX_TOKENS_MAX: u32 = 16_384;
const SUMMARY_MERGE_MAX_TOKENS: u32 = 1_024;

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
    let chunks = plan_chunks(&segments);
    let chunk_count = chunks.len().max(1);
    let full_pass = chunks.len() == 1;

    progress(NoteProgress {
        stage: NoteProgressStage::Chunking,
        chunk_index: None,
        chunk_count,
    });

    if chunks.is_empty() {
        return Err(NoteError::Llm(LlmError::invalid_response(
            "no subtitle segments to analyze",
        )));
    }

    let mut chunk_outputs = Vec::with_capacity(chunks.len());

    for chunk in &chunks {
        if cancellation_check() {
            return Err(NoteError::Cancelled);
        }

        progress(NoteProgress {
            stage: NoteProgressStage::AnalyzingChunk,
            chunk_index: Some(chunk.index),
            chunk_count,
        });

        let output = analyze_chunk(client, &metadata, chunk, locale, duration_ms, full_pass)?;
        if cancellation_check() {
            return Err(NoteError::Cancelled);
        }
        chunk_outputs.push(output);
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

        progress(NoteProgress {
            stage: NoteProgressStage::MergingSummaries,
            chunk_index: None,
            chunk_count,
        });

        let merged = merge_summaries(client, &partial_summaries, locale)?;
        if cancellation_check() {
            return Err(NoteError::Cancelled);
        }
        merged
    };

    if cancellation_check() {
        return Err(NoteError::Cancelled);
    }

    progress(NoteProgress {
        stage: NoteProgressStage::Complete,
        chunk_index: None,
        chunk_count,
    });

    Ok(NoteData {
        metadata,
        summary,
        sections,
        segments,
    })
}

fn max_tokens_for_chunk(chunk: &SubtitleChunk) -> u32 {
    let prompt_chars = segments_prompt_char_count(&chunk.segments);
    // Longer transcripts need more room for section JSON; clamp to a safe range.
    let estimated = (prompt_chars / 4) as u32;
    estimated.clamp(MAX_TOKENS_MIN, MAX_TOKENS_MAX)
}

fn analyze_chunk<T: crate::llm::HttpTransport>(
    client: &LlmClient<T>,
    metadata: &VideoMetadata,
    chunk: &SubtitleChunk,
    locale: NoteLocale,
    video_duration_ms: u64,
    full_pass: bool,
) -> NoteResult<super::types::ChunkLlmOutput> {
    let max_tokens = max_tokens_for_chunk(chunk);
    let messages = chunk_messages(metadata, chunk, locale, full_pass);
    let content = client
        .chat(messages, Some(max_tokens))
        .map_err(NoteError::Llm)?;

    match parse_and_validate_chunk_output(&content, chunk, video_duration_ms) {
        Ok(output) => Ok(output),
        Err(first_err) => {
            let reason = first_err.to_string();
            let fix_messages =
                chunk_fix_messages(metadata, chunk, locale, full_pass, &content, &reason);
            let fixed_content = client
                .chat(fix_messages, Some(max_tokens))
                .map_err(NoteError::Llm)?;
            parse_and_validate_chunk_output(&fixed_content, chunk, video_duration_ms)
                .map_err(NoteError::Llm)
        }
    }
}

fn merge_summaries<T: crate::llm::HttpTransport>(
    client: &LlmClient<T>,
    partials: &[String],
    locale: NoteLocale,
) -> NoteResult<String> {
    let messages = summary_merge_messages(partials, locale);
    let content = client
        .chat(messages, Some(SUMMARY_MERGE_MAX_TOKENS))
        .map_err(NoteError::Llm)?;

    match parse_summary_merge_output(&content) {
        Ok(summary) => Ok(summary),
        Err(first_err) if is_invalid_response(&first_err) => {
            let reason = first_err.to_string();
            let fix_messages = summary_merge_fix_messages(partials, locale, &content, &reason);
            let fixed_content = match client.chat(fix_messages, Some(SUMMARY_MERGE_MAX_TOKENS)) {
                Ok(text) => text,
                Err(retry_err) if is_invalid_response(&retry_err) => {
                    return Ok(fallback_merge_summaries(partials, locale));
                }
                Err(retry_err) => return Err(NoteError::Llm(retry_err)),
            };

            parse_summary_merge_output(&fixed_content)
                .or_else(|_| Ok(fallback_merge_summaries(partials, locale)))
        }
        Err(err) => Err(NoteError::Llm(err)),
    }
}

fn is_invalid_response(err: &LlmError) -> bool {
    err.code() == "LLM_INVALID_RESPONSE"
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

    #[test]
    fn e2e_single_chunk_generation() {
        let chunk_json = r#"{"summary":"overall","sections":[{"start_ms":1000,"end_ms":2000,"title":"Intro","explanation":"Says hi"}]}"#;
        let client =
            client_from_responses(vec![MockResponse::success(success_chat_body(chunk_json))]);

        let segments = vec![segment(1000, 2000, "hello world")];
        let meta = metadata(3000);
        let mut analyze_count = 0usize;

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
                }
            },
        )
        .expect("generate");

        assert_eq!(analyze_count, 1);
        assert_eq!(data.summary, "overall");
        assert_eq!(data.segments, segments);
        assert_eq!(data.sections.len(), 1);

        let md = render_markdown(&data, NoteLocale::En);
        assert!(md.contains("hello world"));
        assert!(md.contains("overall"));
    }

    #[test]
    fn max_tokens_scales_with_chunk_size() {
        let small = SubtitleChunk {
            index: 0,
            start_ms: 0,
            end_ms: 1000,
            segments: vec![segment(0, 1000, "hi")],
        };
        assert_eq!(max_tokens_for_chunk(&small), MAX_TOKENS_MIN);

        let large = SubtitleChunk {
            index: 0,
            start_ms: 0,
            end_ms: 1000,
            segments: vec![segment(0, 1000, &"x".repeat(80_000))],
        };
        assert_eq!(max_tokens_for_chunk(&large), MAX_TOKENS_MAX);
    }

    #[test]
    fn e2e_multi_chunk_with_summary_merge() {
        let chunk1 = r#"{"summary":"part one","sections":[{"start_ms":1000,"end_ms":2000,"title":"A","explanation":"first"}]}"#;
        let chunk2 = r#"{"summary":"part two","sections":[{"start_ms":5000,"end_ms":6000,"title":"B","explanation":"second"}]}"#;
        let merge = r#"{"summary":"merged overall"}"#;

        let client = client_from_responses(vec![
            MockResponse::success(success_chat_body(chunk1)),
            MockResponse::success(success_chat_body(chunk2)),
            MockResponse::success(success_chat_body(merge)),
        ]);

        let big_a = segment(
            1000,
            2000,
            &"x".repeat(super::super::chunk::LARGE_CHUNK_CHAR_BUDGET + 50),
        );
        let big_b = segment(
            5000,
            6000,
            &"y".repeat(super::super::chunk::LARGE_CHUNK_CHAR_BUDGET + 50),
        );
        let segments = vec![big_a, big_b];

        let meta = metadata(7000);
        let data = generate_note_data(&client, meta, segments, NoteLocale::En, || false, |_| {})
            .expect("generate");

        assert_eq!(data.summary, "merged overall");
        assert_eq!(data.sections.len(), 2);
        assert_eq!(data.sections[0].start_ms, 1000);
        assert_eq!(data.sections[1].start_ms, 5000);
    }

    #[test]
    fn invalid_chunk_retries_once_then_fails() {
        let bad = "not json";
        let still_bad = r#"{"summary":"","sections":[]}"#;
        let client = client_from_responses(vec![
            MockResponse::success(success_chat_body(bad)),
            MockResponse::success(success_chat_body(still_bad)),
        ]);

        let segments = vec![segment(1000, 2000, "x")];
        let err = generate_note_data(
            &client,
            metadata(3000),
            segments,
            NoteLocale::En,
            || false,
            |_| {},
        )
        .unwrap_err();

        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }

    #[test]
    fn summary_merge_falls_back_when_json_invalid() {
        let chunk1 = r#"{"summary":"part one","sections":[{"start_ms":1000,"end_ms":2000,"title":"A","explanation":"first"}]}"#;
        let chunk2 = r#"{"summary":"part two","sections":[{"start_ms":5000,"end_ms":6000,"title":"B","explanation":"second"}]}"#;

        let client = client_from_responses(vec![
            MockResponse::success(success_chat_body(chunk1)),
            MockResponse::success(success_chat_body(chunk2)),
            MockResponse::success(success_chat_body("not json")),
            MockResponse::success(success_chat_body(r#"{"summary":""}"#)),
        ]);

        let big_a = segment(
            1000,
            2000,
            &"x".repeat(super::super::chunk::LARGE_CHUNK_CHAR_BUDGET + 50),
        );
        let big_b = segment(
            5000,
            6000,
            &"y".repeat(super::super::chunk::LARGE_CHUNK_CHAR_BUDGET + 50),
        );

        let data = generate_note_data(
            &client,
            metadata(7000),
            vec![big_a, big_b],
            NoteLocale::En,
            || false,
            |_| {},
        )
        .expect("generate");

        assert_eq!(data.summary, "part one\n\npart two");
    }

    #[test]
    fn summary_merge_propagates_auth_errors() {
        let chunk1 = r#"{"summary":"part one","sections":[{"start_ms":1000,"end_ms":2000,"title":"A","explanation":"first"}]}"#;
        let chunk2 = r#"{"summary":"part two","sections":[{"start_ms":5000,"end_ms":6000,"title":"B","explanation":"second"}]}"#;

        let client = client_from_responses(vec![
            MockResponse::success(success_chat_body(chunk1)),
            MockResponse::success(success_chat_body(chunk2)),
            MockResponse::success(r#"{"error":{"message":"bad key"}}"#).with_status(401),
        ]);

        let big_a = segment(
            1000,
            2000,
            &"x".repeat(super::super::chunk::LARGE_CHUNK_CHAR_BUDGET + 50),
        );
        let big_b = segment(
            5000,
            6000,
            &"y".repeat(super::super::chunk::LARGE_CHUNK_CHAR_BUDGET + 50),
        );

        let err = generate_note_data(
            &client,
            metadata(7000),
            vec![big_a, big_b],
            NoteLocale::En,
            || false,
            |_| {},
        )
        .unwrap_err();

        assert_eq!(err.code(), "LLM_AUTH_ERROR");
    }

    #[test]
    fn cancellation_after_chunk_llm_call() {
        let chunk_json = r#"{"summary":"overall","sections":[{"start_ms":1000,"end_ms":2000,"title":"Intro","explanation":"Says hi"}]}"#;
        let client =
            client_from_responses(vec![MockResponse::success(success_chat_body(chunk_json))]);

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
