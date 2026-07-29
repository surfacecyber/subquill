use crate::bilibili::SubtitleSegment;

use super::types::SubtitleChunk;

/// Prefer a single LLM pass when total subtitle prompt lines fit under this limit.
/// Sized for CJK-dense transcripts (~1 char ≈ 1 token) plus room for system/output
/// on common 32k–64k context models.
pub const SINGLE_PASS_CHAR_LIMIT: usize = 24_000;

/// Per-chunk character budget used only when content exceeds [`SINGLE_PASS_CHAR_LIMIT`].
pub const LARGE_CHUNK_CHAR_BUDGET: usize = 12_000;

/// Plan chunks: single pass under the limit; otherwise split into large chunks.
pub fn plan_chunks(segments: &[SubtitleSegment]) -> Vec<SubtitleChunk> {
    if segments.is_empty() {
        return Vec::new();
    }

    let total = segments_prompt_char_count(segments);
    if total <= SINGLE_PASS_CHAR_LIMIT {
        return vec![push_chunk_value(0, segments)];
    }

    chunk_segments_with_budget(segments, LARGE_CHUNK_CHAR_BUDGET)
}

/// Split using an explicit character budget (segment boundaries only).
pub fn chunk_segments_with_budget(
    segments: &[SubtitleSegment],
    budget: usize,
) -> Vec<SubtitleChunk> {
    if segments.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current_segments: Vec<SubtitleSegment> = Vec::new();
    let mut current_chars = 0usize;
    let mut chunk_index = 0usize;

    for segment in segments {
        let line_chars = prompt_line_char_count(segment);

        if !current_segments.is_empty()
            && current_chars + line_chars > budget
            && current_chars > 0
        {
            chunks.push(push_chunk_value(chunk_index, &current_segments));
            chunk_index += 1;
            current_segments.clear();
            current_chars = 0;
        }

        current_chars += line_chars;
        current_segments.push(segment.clone());
    }

    if !current_segments.is_empty() {
        chunks.push(push_chunk_value(chunk_index, &current_segments));
    }

    chunks
}

pub fn segments_prompt_char_count(segments: &[SubtitleSegment]) -> usize {
    segments.iter().map(prompt_line_char_count).sum()
}

fn push_chunk_value(index: usize, segments: &[SubtitleSegment]) -> SubtitleChunk {
    let start_ms = segments.first().map(|s| s.start_ms).unwrap_or(0);
    let end_ms = segments.last().map(|s| s.end_ms).unwrap_or(start_ms);
    SubtitleChunk {
        index,
        start_ms,
        end_ms,
        segments: segments.to_vec(),
    }
}

fn prompt_line_char_count(segment: &SubtitleSegment) -> usize {
    format!(
        "[{}] {}\n",
        format_timestamp(segment.start_ms),
        segment.text
    )
    .chars()
    .count()
}

pub fn format_timestamp(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(start: u64, end: u64, text: &str) -> SubtitleSegment {
        SubtitleSegment {
            start_ms: start,
            end_ms: end,
            text: text.to_string(),
        }
    }

    #[test]
    fn chunk_preserves_all_segments_in_order() {
        let segments = vec![
            segment(0, 1000, "a"),
            segment(1000, 2000, "b"),
            segment(2000, 3000, "c"),
        ];
        let chunks = plan_chunks(&segments);
        let merged: Vec<_> = chunks
            .iter()
            .flat_map(|c| c.segments.iter())
            .cloned()
            .collect();
        assert_eq!(merged, segments);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn short_content_stays_single_pass() {
        let segments = vec![
            segment(0, 1000, &"x".repeat(5_000)),
            segment(1000, 2000, &"y".repeat(5_000)),
        ];
        let chunks = plan_chunks(&segments);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].segments.len(), 2);
    }

    #[test]
    fn oversized_single_segment_forms_own_chunk_when_splitting() {
        let big = "x".repeat(LARGE_CHUNK_CHAR_BUDGET + 100);
        let segments = vec![segment(0, 1000, &big), segment(1000, 2000, "tail")];
        let chunks = chunk_segments_with_budget(&segments, LARGE_CHUNK_CHAR_BUDGET);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].segments.len(), 1);
        assert_eq!(chunks[1].segments.len(), 1);
        assert_eq!(chunks[0].segments[0].text, big);
    }

    #[test]
    fn plan_chunks_splits_when_over_single_pass_limit() {
        let big_a = segment(1000, 2000, &"x".repeat(LARGE_CHUNK_CHAR_BUDGET + 50));
        let big_b = segment(5000, 6000, &"y".repeat(LARGE_CHUNK_CHAR_BUDGET + 50));
        let segments = [big_a.clone(), big_b.clone()];
        assert!(segments_prompt_char_count(&segments) > SINGLE_PASS_CHAR_LIMIT);
        let chunks = plan_chunks(&segments);
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn chunk_records_start_and_end_ms() {
        let segments = vec![segment(500, 1500, "a"), segment(1500, 2500, "b")];
        let chunks = plan_chunks(&segments);
        assert_eq!(chunks[0].start_ms, 500);
        assert_eq!(chunks[0].end_ms, 2500);
    }

    #[test]
    fn two_big_segments_form_two_chunks_that_validate() {
        use super::super::validate::parse_and_validate_chunk_output;

        let big_a = segment(1000, 2000, &"x".repeat(LARGE_CHUNK_CHAR_BUDGET + 50));
        let big_b = segment(5000, 6000, &"y".repeat(LARGE_CHUNK_CHAR_BUDGET + 50));
        let chunks = plan_chunks(&[big_a, big_b]);
        assert_eq!(chunks.len(), 2);

        let chunk1 = r#"{"summary":"part one","sections":[{"start_ms":1000,"end_ms":2000,"title":"A","explanation":"first"}]}"#;
        let chunk2 = r#"{"summary":"part two","sections":[{"start_ms":5000,"end_ms":6000,"title":"B","explanation":"second"}]}"#;

        parse_and_validate_chunk_output(chunk1, &chunks[0], 7000).expect("chunk1");
        parse_and_validate_chunk_output(chunk2, &chunks[1], 7000).expect("chunk2");
    }

    #[test]
    fn prompt_line_char_count_uses_unicode_chars_not_bytes() {
        let seg = segment(0, 1000, "中文");
        let count = prompt_line_char_count(&seg);
        // "[00:00:00] 中文\n" => 14 Unicode scalars, not 18 bytes
        assert_eq!(count, 14);
    }

    #[test]
    fn chinese_text_chunks_by_char_budget() {
        let one_line = "中".repeat(1000);
        let seg = segment(0, 1000, &one_line);
        let count = prompt_line_char_count(&seg);
        assert_eq!(count, 1000 + "[00:00:00] ".len() + 1); // timestamp ascii + newline
        assert!(count < one_line.len()); // would be 3x if counted as bytes incorrectly for CJK only part
    }

    #[test]
    fn format_timestamp_supports_over_one_hour() {
        assert_eq!(format_timestamp(3_661_000), "01:01:01");
        assert_eq!(format_timestamp(7_200_000), "02:00:00");
    }
}
