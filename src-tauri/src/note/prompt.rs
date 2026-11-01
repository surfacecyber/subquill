use crate::llm::ChatMessage;
use crate::redact::{truncate_unicode_chars, with_truncation_notice};

use super::chunk::format_timestamp;
use super::types::{NoteLocale, SubtitleChunk, VideoMetadata};

/// Max Unicode chars from an invalid model reply included in a fix prompt.
pub const CHUNK_FIX_CONTENT_MAX_CHARS: usize = 14_000;

pub fn chunk_system_prompt(locale: NoteLocale) -> String {
    match locale {
        NoteLocale::Zh => {
            "你是学习笔记助手。只能根据提供的字幕片段生成笔记，不得编造事实。\
             不要输出思维过程，不要复述或粘贴完整字幕。\
             每段讲解必须是完整句子，禁止在句中截断。\
             若字幕把一句话拆到多行，讲解里要把该句意思写完。\
             分段须按时间升序覆盖本块要点，相邻小节之间不要无故跳过中间大段内容。\
             直接按以下 Markdown 结构回复，不要用代码块包裹：\n\
             ## 总结\n\n一段总述\n\n\
             ## 分段讲解\n\n\
             ### 小节标题（hh:mm:ss - hh:mm:ss）\n\n该段讲解正文\n\n\
             时间必须来自字幕行的时间码，分段按时间升序。"
                .to_string()
        }
        NoteLocale::En => {
            "You are a study-note assistant. Use only the provided subtitle lines; do not invent facts. \
             Do not output chain-of-thought. Do not repeat or paste the full transcript. \
             Each explanation must be complete sentences; never stop mid-clause. \
             If a spoken sentence is split across subtitle lines, finish that thought in the note. \
             Cover this chunk in time order; do not skip long stretches of substance between sections. \
             Reply in this Markdown structure, with no code fences:\n\
             ## Summary\n\nOverall summary\n\n\
             ## Section breakdown\n\n\
             ### Section title (hh:mm:ss - hh:mm:ss)\n\nExplanation\n\n\
             Timestamps must come from the subtitle lines; sections sorted by start time."
                .to_string()
        }
    }
}

pub fn chunk_user_prompt(
    metadata: &VideoMetadata,
    chunk: &SubtitleChunk,
    locale: NoteLocale,
    full_pass: bool,
) -> String {
    let mut lines = Vec::new();
    match locale {
        NoteLocale::Zh => {
            lines.push(format!("视频标题：{}", metadata.title));
            if let Some(part) = &metadata.part_title {
                lines.push(format!("分 P 标题：{}", part));
            }
            lines.push(format!("视频 ID：{}", metadata.bvid));
            if full_pass {
                lines.push(format!(
                    "整场时间范围：{} - {}（毫秒 {}-{}）",
                    format_timestamp(chunk.start_ms),
                    format_timestamp(chunk.end_ms),
                    chunk.start_ms,
                    chunk.end_ms
                ));
                lines.push("整场字幕（逐行，仅作依据）：".to_string());
            } else {
                lines.push(format!(
                    "本块时间范围：{} - {}（毫秒 {}-{}）",
                    format_timestamp(chunk.start_ms),
                    format_timestamp(chunk.end_ms),
                    chunk.start_ms,
                    chunk.end_ms
                ));
                lines.push(
                    "相邻分块含重叠字幕，只用来补全跨块句子，不要对重叠部分重复开节。".to_string(),
                );
                lines.push("字幕（逐行，仅作依据）：".to_string());
            }
        }
        NoteLocale::En => {
            lines.push(format!("Video title: {}", metadata.title));
            if let Some(part) = &metadata.part_title {
                lines.push(format!("Part title: {}", part));
            }
            lines.push(format!("Video id: {}", metadata.bvid));
            if full_pass {
                lines.push(format!(
                    "Full video range: {} - {} (ms {}-{})",
                    format_timestamp(chunk.start_ms),
                    format_timestamp(chunk.end_ms),
                    chunk.start_ms,
                    chunk.end_ms
                ));
                lines.push("Full transcript (line by line, source of truth only):".to_string());
            } else {
                lines.push(format!(
                    "Chunk range: {} - {} (ms {}-{})",
                    format_timestamp(chunk.start_ms),
                    format_timestamp(chunk.end_ms),
                    chunk.start_ms,
                    chunk.end_ms
                ));
                lines.push(
                    "Adjacent chunks include overlapping lines so you can finish sentences that cross the split; do not open a duplicate section for the overlap only."
                        .to_string(),
                );
                lines.push("Subtitles (line by line, source of truth only):".to_string());
            }
        }
    }

    for segment in &chunk.segments {
        lines.push(format!(
            "[{}] {}",
            format_timestamp(segment.start_ms),
            segment.text
        ));
    }

    lines.join("\n")
}

pub fn truncate_for_fix_prompt(content: &str) -> String {
    let total_chars = content.chars().count();
    let (truncated, was_truncated) = truncate_unicode_chars(content, CHUNK_FIX_CONTENT_MAX_CHARS);
    with_truncation_notice(
        truncated,
        was_truncated,
        total_chars.saturating_sub(CHUNK_FIX_CONTENT_MAX_CHARS),
    )
}

pub fn summary_merge_system_prompt(locale: NoteLocale) -> String {
    match locale {
        NoteLocale::Zh => "合并多段 partial summary，生成一段总结合并文本。不得新增事实。\
             直接输出合并后的总结正文，不要标题、代码块或其它结构。"
            .to_string(),
        NoteLocale::En => {
            "Merge partial summaries into one overall summary. Do not add new facts. \
             Reply with the merged summary prose only — no headings, fences, or extra structure."
                .to_string()
        }
    }
}

pub fn summary_merge_user_prompt(partials: &[String], locale: NoteLocale) -> String {
    let header = match locale {
        NoteLocale::Zh => "各块 partial summary（按顺序）：",
        NoteLocale::En => "Partial summaries in order:",
    };
    let mut lines = vec![header.to_string()];
    for (index, summary) in partials.iter().enumerate() {
        lines.push(format!("{}. {}", index + 1, summary));
    }
    lines.join("\n")
}

pub fn summary_merge_fix_prompt(locale: NoteLocale, invalid_json: &str, reason: &str) -> String {
    let clipped = truncate_for_fix_prompt(invalid_json);
    match locale {
        NoteLocale::Zh => format!(
            "合并 summary 无效（{}）。仅整理已有内容为一段总结，不要新增事实。无效内容：\n{}",
            reason, clipped
        ),
        NoteLocale::En => format!(
            "Summary merge was invalid ({}). Rewrite existing content as one summary; do not add new facts. Invalid content:\n{}",
            reason, clipped
        ),
    }
}

pub fn chunk_messages(
    metadata: &VideoMetadata,
    chunk: &SubtitleChunk,
    locale: NoteLocale,
    full_pass: bool,
) -> Vec<ChatMessage> {
    vec![
        ChatMessage::system(chunk_system_prompt(locale)),
        ChatMessage::user(chunk_user_prompt(metadata, chunk, locale, full_pass)),
    ]
}

pub fn summary_merge_messages(partials: &[String], locale: NoteLocale) -> Vec<ChatMessage> {
    vec![
        ChatMessage::system(summary_merge_system_prompt(locale)),
        ChatMessage::user(summary_merge_user_prompt(partials, locale)),
    ]
}

pub fn summary_merge_fix_messages(
    partials: &[String],
    locale: NoteLocale,
    invalid_json: &str,
    reason: &str,
) -> Vec<ChatMessage> {
    let mut messages = summary_merge_messages(partials, locale);
    messages.push(ChatMessage::user(summary_merge_fix_prompt(
        locale,
        invalid_json,
        reason,
    )));
    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::SubtitleSegment;

    fn meta() -> VideoMetadata {
        VideoMetadata {
            title: "T".to_string(),
            part_title: None,
            bvid: "BV1".to_string(),
            duration_ms: 3000,
        }
    }

    fn chunk() -> SubtitleChunk {
        SubtitleChunk {
            index: 0,
            start_ms: 0,
            end_ms: 2000,
            segments: vec![SubtitleSegment {
                start_ms: 0,
                end_ms: 2000,
                text: "hello".to_string(),
            }],
        }
    }

    #[test]
    fn truncate_for_fix_prompt_limits_unicode_chars() {
        let huge = "字".repeat(CHUNK_FIX_CONTENT_MAX_CHARS + 500);
        let clipped = truncate_for_fix_prompt(&huge);
        assert!(clipped.chars().count() <= CHUNK_FIX_CONTENT_MAX_CHARS + 64);
        assert!(clipped.contains("[truncated"));
    }

    #[test]
    fn truncate_for_fix_prompt_preserves_valid_utf8() {
        let text = "🎬".repeat(CHUNK_FIX_CONTENT_MAX_CHARS + 10);
        let clipped = truncate_for_fix_prompt(&text);
        assert!(clipped.is_char_boundary(clipped.len()));
    }

    #[test]
    fn full_pass_prompt_uses_whole_transcript_wording() {
        let full = chunk_user_prompt(&meta(), &chunk(), NoteLocale::Zh, true);
        assert!(full.contains("整场字幕"));
        assert!(full.contains("整场时间范围"));
        assert!(!full.contains("本块时间范围"));

        let part = chunk_user_prompt(&meta(), &chunk(), NoteLocale::En, false);
        assert!(part.contains("Chunk range"));
        assert!(part.contains("Subtitles (line by line, source of truth only)"));
        assert!(!part.contains("Full transcript"));
    }
}
