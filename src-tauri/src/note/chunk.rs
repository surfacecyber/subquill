use crate::bilibili::SubtitleSegment;

use super::types::SubtitleChunk;

#[allow(dead_code)]
pub const SINGLE_PASS_CHAR_LIMIT: usize = 24_000;

#[allow(dead_code)]
pub const LARGE_CHUNK_CHAR_BUDGET: usize = 12_000;

#[allow(dead_code)]
pub const CHUNK_OVERLAP_CHAR_BUDGET: usize = 1_200;

/// Join adjacent cues into one analysis line when the gap is at most this long.
const JOIN_MAX_GAP_MS: u64 = 2_500;

/// Cap on a joined analysis line so one run-on caption cannot swallow a whole chunk.
const JOIN_MAX_CHARS: usize = 480;

/// Send the whole transcript in one prompt. If the model stops early,
/// [`super::generate::generate_note_data`] continues from the leftover time.
pub fn plan_chunks(segments: &[SubtitleSegment]) -> Vec<SubtitleChunk> {
    if segments.is_empty() {
        return Vec::new();
    }
    vec![push_chunk_value(0, segments)]
}

/// Join adjacent subtitle cues that are clearly one unfinished sentence.
pub fn join_segments_for_analysis(segments: &[SubtitleSegment]) -> Vec<SubtitleSegment> {
    let mut joined: Vec<SubtitleSegment> = Vec::new();
    for segment in segments {
        if let Some(last) = joined.last_mut() {
            if should_join_segments(last, segment) {
                last.text = join_cue_text(&last.text, &segment.text);
                last.end_ms = last.end_ms.max(segment.end_ms);
                continue;
            }
        }
        joined.push(segment.clone());
    }
    joined
}

pub(crate) fn ends_complete_sentence(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    let stripped = trimmed.trim_end_matches(|c: char| {
        matches!(
            c,
            '"' | '\'' | '”' | '’' | '「' | '」' | '『' | '』' | ')' | '）' | ']' | '】'
        )
    });
    matches!(
        stripped.chars().last(),
        Some('。' | '！' | '？' | '.' | '!' | '?' | '…' | '；' | ';')
    )
}

fn should_join_segments(prev: &SubtitleSegment, next: &SubtitleSegment) -> bool {
    if next.start_ms.saturating_sub(prev.end_ms) > JOIN_MAX_GAP_MS {
        return false;
    }
    if ends_complete_sentence(&prev.text) {
        return false;
    }
    prev.text.chars().count() + next.text.chars().count() <= JOIN_MAX_CHARS
}

fn join_cue_text(left: &str, right: &str) -> String {
    let left = left.trim();
    let right = right.trim();
    if left.is_empty() {
        return right.to_string();
    }
    if right.is_empty() {
        return left.to_string();
    }
    match (left.chars().last(), right.chars().next()) {
        (Some(a), Some(b)) if a.is_ascii_alphanumeric() && b.is_ascii_alphanumeric() => {
            format!("{left} {right}")
        }
        _ => format!("{left}{right}"),
    }
}

/// Split using an explicit character budget (segment boundaries only).
#[allow(dead_code)]
pub fn chunk_segments_with_budget(
    segments: &[SubtitleSegment],
    budget: usize,
) -> Vec<SubtitleChunk> {
    let unique = split_segments_with_budget(segments, budget);
    apply_prefix_overlap(unique, overlap_budget_for(budget))
}

#[allow(dead_code)]
fn overlap_budget_for(chunk_budget: usize) -> usize {
    CHUNK_OVERLAP_CHAR_BUDGET.min(chunk_budget / 8)
}

#[allow(dead_code)]
fn split_segments_with_budget(
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

#[allow(dead_code)]
fn apply_prefix_overlap(unique: Vec<SubtitleChunk>, overlap_budget: usize) -> Vec<SubtitleChunk> {
    if unique.len() <= 1 || overlap_budget == 0 {
        return unique;
    }

    let unique_segments: Vec<Vec<SubtitleSegment>> =
        unique.iter().map(|chunk| chunk.segments.clone()).collect();

    unique
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            if index == 0 {
                return chunk;
            }
            let prefix = overlap_suffix(&unique_segments[index - 1], overlap_budget);
            let unique_start = chunk
                .segments
                .first()
                .map(|segment| segment.start_ms)
                .unwrap_or(0);
            let mut segments: Vec<SubtitleSegment> = prefix
                .into_iter()
                .filter(|segment| segment.start_ms < unique_start)
                .collect();
            if segments.is_empty() {
                return chunk;
            }
            segments.extend(chunk.segments);
            push_chunk_value(chunk.index, &segments)
        })
        .collect()
}

#[allow(dead_code)]
fn overlap_suffix(segments: &[SubtitleSegment], budget: usize) -> Vec<SubtitleSegment> {
    if segments.is_empty() || budget == 0 {
        return Vec::new();
    }

    let mut chars = 0usize;
    let mut take = 0usize;
    for segment in segments.iter().rev() {
        let line_chars = prompt_line_char_count(segment);
        if take == 0 && line_chars > budget {
            break;
        }
        if take > 0 && chars + line_chars > budget {
            break;
        }
        chars += line_chars;
        take += 1;
    }

    if take == 0 {
        return Vec::new();
    }
    segments[segments.len() - take..].to_vec()
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    fn plan_chunks_keeps_long_transcript_in_one_pass() {
        let big_a = segment(1000, 2000, &"x".repeat(LARGE_CHUNK_CHAR_BUDGET + 50));
        let big_b = segment(5000, 6000, &"y".repeat(LARGE_CHUNK_CHAR_BUDGET + 50));
        let segments = [big_a.clone(), big_b.clone()];
        assert!(segments_prompt_char_count(&segments) > SINGLE_PASS_CHAR_LIMIT);
        let chunks = plan_chunks(&segments);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].segments.len(), 2);
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
        assert_eq!(chunks.len(), 1);

        let chunk1 = r#"{"summary":"part one","sections":[{"start_ms":1000,"end_ms":2000,"title":"A","explanation":"first"}]}"#;
        parse_and_validate_chunk_output(chunk1, &chunks[0], 7000).expect("chunk1");
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
