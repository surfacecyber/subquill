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
             必须仅回复一个 JSON 对象，不要 markdown 代码块或其它文字。\
             格式：{\"summary\":\"...\",\"sections\":[{\"start_ms\":0,\"end_ms\":1000,\"title\":\"...\",\"explanation\":\"...\"}]}。\
             start_ms/end_ms 为整数毫秒，sections 按 start_ms 升序。"
                .to_string()
        }
        NoteLocale::En => {
            "You are a study-note assistant. Use only the provided subtitle lines; do not invent facts. \
             Reply with a single JSON object only — no markdown fences or extra text. \
             Schema: {\"summary\":\"...\",\"sections\":[{\"start_ms\":0,\"end_ms\":1000,\"title\":\"...\",\"explanation\":\"...\"}]}. \
             Times are integer milliseconds; sections sorted by start_ms."
                .to_string()
        }
    }
}

pub fn chunk_user_prompt(
    metadata: &VideoMetadata,
    chunk: &SubtitleChunk,
    locale: NoteLocale,
) -> String {
    let mut lines = Vec::new();
    match locale {
        NoteLocale::Zh => {
            lines.push(format!("视频标题：{}", metadata.title));
            if let Some(part) = &metadata.part_title {
                lines.push(format!("分 P 标题：{}", part));
            }
            lines.push(format!("BV 号：{}", metadata.bvid));
            lines.push(format!(
                "本块时间范围：{} - {}（毫秒 {}-{}）",
                format_timestamp(chunk.start_ms),
                format_timestamp(chunk.end_ms),
                chunk.start_ms,
                chunk.end_ms
            ));
            lines.push("字幕（逐行，请勿改写）：".to_string());
        }
        NoteLocale::En => {
            lines.push(format!("Video title: {}", metadata.title));
            if let Some(part) = &metadata.part_title {
                lines.push(format!("Part title: {}", part));
            }
            lines.push(format!("BV id: {}", metadata.bvid));
            lines.push(format!(
                "Chunk range: {} - {} (ms {}-{})",
                format_timestamp(chunk.start_ms),
                format_timestamp(chunk.end_ms),
                chunk.start_ms,
                chunk.end_ms
            ));
            lines.push("Subtitles (line by line, do not rewrite):".to_string());
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

pub fn chunk_fix_prompt(locale: NoteLocale, invalid_json: &str, reason: &str) -> String {
    let clipped = truncate_for_fix_prompt(invalid_json);
    match locale {
        NoteLocale::Zh => format!(
            "上一次回复无效（{}）。请仅修复 JSON 结构与字段约束，不要新增事实。\
             无效内容：\n{}\n\
             仍须返回单个 JSON 对象：{{\"summary\":\"...\",\"sections\":[...]}}。",
            reason, clipped
        ),
        NoteLocale::En => format!(
            "Previous reply was invalid ({}). Fix JSON structure and field constraints only; do not add new facts. \
             Invalid content:\n{}\n\
             Return one JSON object: {{\"summary\":\"...\",\"sections\":[...]}}.",
            reason, clipped
        ),
    }
}

pub fn summary_merge_system_prompt(locale: NoteLocale) -> String {
    match locale {
        NoteLocale::Zh => "合并多段 partial summary，生成一段总结合并文本。不得新增事实。\
             仅回复 JSON：{\"summary\":\"...\"}，不要其它文字。"
            .to_string(),
        NoteLocale::En => {
            "Merge partial summaries into one overall summary. Do not add new facts. \
             Reply with JSON only: {\"summary\":\"...\"}."
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
            "合并 summary 的 JSON 无效（{}）。仅修复 JSON 结构，不要新增事实。无效内容：\n{}\n\
             仍须返回：{{\"summary\":\"...\"}}。",
            reason, clipped
        ),
        NoteLocale::En => format!(
            "Summary merge JSON was invalid ({}). Fix JSON structure only; do not add new facts. Invalid content:\n{}\n\
             Return: {{\"summary\":\"...\"}}.",
            reason, clipped
        ),
    }
}

pub fn chunk_messages(
    metadata: &VideoMetadata,
    chunk: &SubtitleChunk,
    locale: NoteLocale,
) -> Vec<ChatMessage> {
    vec![
        ChatMessage::system(chunk_system_prompt(locale)),
        ChatMessage::user(chunk_user_prompt(metadata, chunk, locale)),
    ]
}

pub fn chunk_fix_messages(
    metadata: &VideoMetadata,
    chunk: &SubtitleChunk,
    locale: NoteLocale,
    invalid_json: &str,
    reason: &str,
) -> Vec<ChatMessage> {
    let mut messages = chunk_messages(metadata, chunk, locale);
    messages.push(ChatMessage::user(chunk_fix_prompt(
        locale,
        invalid_json,
        reason,
    )));
    messages
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
}
