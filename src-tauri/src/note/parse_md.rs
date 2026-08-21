use crate::llm::{LlmError, LlmResult};

use super::types::{ChunkLlmOutput, NoteSection, SubtitleChunk};
use super::validate::parse_and_validate_chunk_output;

/// Parse model output into structured note data.
///
/// Prefers a valid JSON payload when the model still emits one, then falls
/// back to the Markdown study-note schema. Any non-empty prose degrades into
/// a single section so the job can still save a readable draft.
pub fn parse_chunk_output(
    content: &str,
    chunk: &SubtitleChunk,
    video_duration_ms: u64,
) -> LlmResult<ChunkLlmOutput> {
    let text = unwrap_markdown_fence(content);
    if text.trim().is_empty() {
        return Err(LlmError::invalid_response("model content is empty"));
    }

    if text.trim_start().starts_with('{') {
        if let Ok(output) = parse_and_validate_chunk_output(text, chunk, video_duration_ms) {
            return Ok(output);
        }
    }

    Ok(parse_markdown_note(text, chunk, video_duration_ms))
}

pub fn unwrap_markdown_fence(content: &str) -> &str {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed;
    }

    let rest = &trimmed[3..];
    let rest = rest
        .strip_prefix("markdown")
        .or_else(|| rest.strip_prefix("json"))
        .or_else(|| rest.strip_prefix("md"))
        .unwrap_or(rest);
    let rest = rest.trim_start_matches(['\r', '\n', ' ']);

    match rest.rfind("```") {
        Some(idx) => rest[..idx].trim(),
        None => rest.trim(),
    }
}

fn parse_markdown_note(
    content: &str,
    chunk: &SubtitleChunk,
    video_duration_ms: u64,
) -> ChunkLlmOutput {
    let without_transcript = strip_transcript_section(content);
    let summary = extract_summary(&without_transcript);
    let mut sections = extract_sections(&without_transcript, chunk, video_duration_ms);

    if sections.is_empty() {
        let explanation = body_after_summary(&without_transcript, &summary);
        let explanation = if explanation.trim().is_empty() {
            summary.clone()
        } else {
            explanation
        };
        sections.push(NoteSection {
            start_ms: chunk.start_ms,
            end_ms: chunk.end_ms.max(chunk.start_ms.saturating_add(1)),
            title: fallback_section_title(),
            explanation,
        });
    }

    sections.sort_by_key(|section| section.start_ms);

    let summary = if summary.trim().is_empty() {
        sections
            .first()
            .map(|section| section.explanation.clone())
            .unwrap_or_else(|| without_transcript.trim().to_string())
    } else {
        summary
    };

    ChunkLlmOutput { summary, sections }
}

fn fallback_section_title() -> String {
    "Notes".to_string()
}

fn strip_transcript_section(content: &str) -> String {
    let mut out = Vec::new();
    for line in content.lines() {
        if is_h2(line, &["完整字幕", "full transcript"]) {
            break;
        }
        out.push(line);
    }
    out.join("\n")
}

fn extract_summary(content: &str) -> String {
    if let Some(body) = section_body_after_h2(content, &["总结", "summary"]) {
        return body;
    }

    let mut lines = Vec::new();
    for line in content.lines() {
        if is_h2(line, &["分段讲解", "section breakdown"]) || line.starts_with("### ") {
            break;
        }
        if line.starts_with('#') {
            continue;
        }
        lines.push(line);
    }

    collapse_paragraphs(&lines.join("\n"))
}

fn body_after_summary(content: &str, summary: &str) -> String {
    let remainder = content.replacen(summary, "", 1);
    let mut lines = Vec::new();
    for line in remainder.lines() {
        if is_h2(
            line,
            &["总结", "summary", "分段讲解", "section breakdown"],
        ) {
            continue;
        }
        lines.push(line);
    }
    collapse_paragraphs(&lines.join("\n"))
}

fn extract_sections(
    content: &str,
    chunk: &SubtitleChunk,
    video_duration_ms: u64,
) -> Vec<NoteSection> {
    let mut sections = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_start = chunk.start_ms;
    let mut current_end = chunk.end_ms;
    let mut body_lines: Vec<&str> = Vec::new();
    let mut in_breakdown = false;

    for line in content.lines() {
        if is_h2(line, &["分段讲解", "section breakdown"]) {
            in_breakdown = true;
            continue;
        }
        if is_h2(line, &["总结", "summary"]) {
            in_breakdown = false;
            continue;
        }
        if line.starts_with("## ") {
            in_breakdown = false;
            continue;
        }

        if let Some(stripped) = line.strip_prefix("### ") {
            flush_section(
                &mut sections,
                current_title.take(),
                current_start,
                current_end,
                &mut body_lines,
                chunk,
                video_duration_ms,
            );
            let (title, start, end) = parse_heading(stripped, chunk);
            current_title = Some(title);
            current_start = start;
            current_end = end;
            in_breakdown = true;
            continue;
        }

        if in_breakdown || current_title.is_some() {
            body_lines.push(line);
        }
    }

    flush_section(
        &mut sections,
        current_title,
        current_start,
        current_end,
        &mut body_lines,
        chunk,
        video_duration_ms,
    );

    sections
}

fn flush_section(
    sections: &mut Vec<NoteSection>,
    title: Option<String>,
    start_ms: u64,
    end_ms: u64,
    body_lines: &mut Vec<&str>,
    chunk: &SubtitleChunk,
    video_duration_ms: u64,
) {
    let Some(title) = title else {
        body_lines.clear();
        return;
    };
    let explanation = collapse_paragraphs(&body_lines.join("\n"));
    body_lines.clear();
    if title.trim().is_empty() && explanation.trim().is_empty() {
        return;
    }

    let (start_ms, end_ms) = clamp_range(start_ms, end_ms, chunk, video_duration_ms);
    sections.push(NoteSection {
        start_ms,
        end_ms,
        title: if title.trim().is_empty() {
            fallback_section_title()
        } else {
            title.trim().to_string()
        },
        explanation: if explanation.trim().is_empty() {
            title
        } else {
            explanation
        },
    });
}

fn parse_heading(heading: &str, chunk: &SubtitleChunk) -> (String, u64, u64) {
    let trimmed = heading.trim();
    if let Some((title, start, end)) = split_heading_times(trimmed) {
        return (title, start, end);
    }
    (trimmed.to_string(), chunk.start_ms, chunk.end_ms)
}

fn split_heading_times(heading: &str) -> Option<(String, u64, u64)> {
    let open = heading.rfind('（').or_else(|| heading.rfind('('))?;
    let close = heading.rfind('）').or_else(|| heading.rfind(')'))?;
    if close <= open {
        return None;
    }

    let open_ch = heading[open..].chars().next()?;
    let range = heading[open + open_ch.len_utf8()..close].trim();
    let (start_raw, end_raw) = split_range(range)?;
    let start = parse_timestamp_ms(start_raw)?;
    let end = parse_timestamp_ms(end_raw)?;
    let title = heading[..open].trim().to_string();
    if title.is_empty() {
        return None;
    }
    Some((title, start, end))
}

fn split_range(range: &str) -> Option<(&str, &str)> {
    for sep in [" - ", " – ", " — ", "-", "–", "—"] {
        if let Some((start, end)) = range.split_once(sep) {
            let start = start.trim();
            let end = end.trim();
            if !start.is_empty() && !end.is_empty() {
                return Some((start, end));
            }
        }
    }
    None
}

pub fn parse_timestamp_ms(value: &str) -> Option<u64> {
    let parts: Vec<&str> = value.trim().split(':').collect();
    let nums: Vec<u64> = parts
        .iter()
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;

    match nums.as_slice() {
        [minutes, seconds] if *minutes < 60 && *seconds < 60 => {
            Some(minutes * 60_000 + seconds * 1_000)
        }
        [hours, minutes, seconds] if *minutes < 60 && *seconds < 60 => {
            Some(hours * 3_600_000 + minutes * 60_000 + seconds * 1_000)
        }
        _ => None,
    }
}

fn clamp_range(
    start_ms: u64,
    end_ms: u64,
    chunk: &SubtitleChunk,
    video_duration_ms: u64,
) -> (u64, u64) {
    let max_end = chunk.end_ms.min(video_duration_ms.max(chunk.end_ms));
    let mut start = start_ms.max(chunk.start_ms).min(max_end);
    let mut end = end_ms.max(chunk.start_ms).min(max_end);
    if end <= start {
        end = (start.saturating_add(1_000)).min(max_end);
        if end <= start {
            start = chunk.start_ms;
            end = chunk.end_ms.max(start.saturating_add(1));
        }
    }
    (start, end)
}

fn section_body_after_h2(content: &str, titles: &[&str]) -> Option<String> {
    let mut take = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if take && line.starts_with("## ") {
            break;
        }
        if is_h2(line, titles) {
            take = true;
            continue;
        }
        if take {
            lines.push(line);
        }
    }
    if !take {
        return None;
    }
    let body = collapse_paragraphs(&lines.join("\n"));
    if body.is_empty() {
        None
    } else {
        Some(body)
    }
}

fn is_h2(line: &str, titles: &[&str]) -> bool {
    let Some(rest) = line.trim().strip_prefix("## ") else {
        return false;
    };
    let heading = rest.trim().trim_end_matches(':').to_ascii_lowercase();
    titles
        .iter()
        .any(|title| heading == title.to_ascii_lowercase())
}

fn collapse_paragraphs(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::SubtitleSegment;
    use crate::note::chunk::format_timestamp;

    fn chunk() -> SubtitleChunk {
        SubtitleChunk {
            index: 0,
            start_ms: 1000,
            end_ms: 5000,
            segments: vec![SubtitleSegment {
                start_ms: 1000,
                end_ms: 2000,
                text: "hi".to_string(),
            }],
        }
    }

    #[test]
    fn parses_markdown_note_schema() {
        let md = format!(
            "## Summary\n\noverall\n\n## Section breakdown\n\n### Intro ({start} - {end})\n\nSays hi\n",
            start = format_timestamp(1000),
            end = format_timestamp(2000),
        );
        let output = parse_chunk_output(&md, &chunk(), 10_000).unwrap();
        assert_eq!(output.summary, "overall");
        assert_eq!(output.sections.len(), 1);
        assert_eq!(output.sections[0].title, "Intro");
        assert_eq!(output.sections[0].start_ms, 1000);
        assert_eq!(output.sections[0].end_ms, 2000);
        assert_eq!(output.sections[0].explanation, "Says hi");
    }

    #[test]
    fn parses_chinese_headings_and_fullwidth_parens() {
        let md = format!(
            "## 总结\n\n总述\n\n## 分段讲解\n\n### 介绍（{start} - {end}）\n\n打招呼\n",
            start = format_timestamp(1000),
            end = format_timestamp(2000),
        );
        let output = parse_chunk_output(&md, &chunk(), 10_000).unwrap();
        assert_eq!(output.summary, "总述");
        assert_eq!(output.sections[0].title, "介绍");
    }

    #[test]
    fn json_payload_still_accepted() {
        let json = r#"{"summary":"s","sections":[{"start_ms":1000,"end_ms":2000,"title":"t","explanation":"e"}]}"#;
        let output = parse_chunk_output(json, &chunk(), 10_000).unwrap();
        assert_eq!(output.summary, "s");
        assert_eq!(output.sections[0].title, "t");
    }

    #[test]
    fn unstructured_prose_degrades() {
        let output = parse_chunk_output("just some notes about the video", &chunk(), 10_000).unwrap();
        assert!(output.summary.contains("just some notes"));
        assert_eq!(output.sections.len(), 1);
        assert_eq!(output.sections[0].start_ms, 1000);
    }

    #[test]
    fn clamps_out_of_range_times() {
        let md = "## Summary\n\ns\n\n## Section breakdown\n\n### Far (00:00:00 - 00:10:00)\n\nbody\n";
        let output = parse_chunk_output(md, &chunk(), 10_000).unwrap();
        assert_eq!(output.sections[0].start_ms, 1000);
        assert_eq!(output.sections[0].end_ms, 5000);
    }

    #[test]
    fn parse_timestamp_accepts_hhmmss_and_mmss() {
        assert_eq!(parse_timestamp_ms("00:00:01"), Some(1000));
        assert_eq!(parse_timestamp_ms("01:02"), Some(62_000));
        assert_eq!(parse_timestamp_ms("01:01:01"), Some(3_661_000));
    }

    #[test]
    fn empty_content_is_invalid() {
        let err = parse_chunk_output("   ", &chunk(), 10_000).unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }
}
