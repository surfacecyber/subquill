use crate::bilibili::SubtitleSegment;

use super::chunk::format_timestamp;
use super::types::{NoteData, NoteLocale, NoteSection};

pub fn render_markdown(data: &NoteData, locale: NoteLocale) -> String {
    let mut out = String::new();

    let display_title = build_display_title(&data.metadata);
    out.push_str(&format!("# {}\n\n", escape_heading_text(&display_title)));

    let summary_heading = match locale {
        NoteLocale::Zh => "总结",
        NoteLocale::En => "Summary",
    };
    out.push_str(&format!("## {}\n\n", summary_heading));
    out.push_str(&escape_markdown_paragraph(&data.summary));
    out.push('\n');
    out.push('\n');

    let sections_heading = match locale {
        NoteLocale::Zh => "分段讲解",
        NoteLocale::En => "Section breakdown",
    };
    out.push_str(&format!("## {}\n\n", sections_heading));
    for section in &data.sections {
        out.push_str(&format_section(section, locale));
        out.push('\n');
    }

    let transcript_heading = match locale {
        NoteLocale::Zh => "完整字幕",
        NoteLocale::En => "Full transcript",
    };
    out.push_str(&format!("## {}\n\n", transcript_heading));
    for segment in &data.segments {
        out.push_str(&format_transcript_line(segment));
        out.push('\n');
    }

    out
}

fn build_display_title(metadata: &super::types::VideoMetadata) -> String {
    if let Some(part) = &metadata.part_title {
        format!("{} — {}", metadata.title, part)
    } else {
        metadata.title.clone()
    }
}

fn format_section(section: &NoteSection, locale: NoteLocale) -> String {
    let range = format!(
        "{} - {}",
        format_timestamp(section.start_ms),
        format_timestamp(section.end_ms)
    );
    let heading = match locale {
        NoteLocale::Zh => format!("### {}（{}）", escape_heading_text(&section.title), range),
        NoteLocale::En => format!("### {} ({})", escape_heading_text(&section.title), range),
    };
    format!(
        "{}\n\n{}\n",
        heading,
        escape_markdown_paragraph(&section.explanation)
    )
}

fn format_transcript_line(segment: &SubtitleSegment) -> String {
    format!(
        "- [{}] {}",
        format_timestamp(segment.start_ms),
        escape_transcript_text(&segment.text)
    )
}

/// Fold internal newlines and escape for a single transcript list item.
fn escape_transcript_text(text: &str) -> String {
    escape_markdown_inline(&fold_newlines_to_space(text))
}

/// Escape text used in `#` / `###` headings — fold newlines to spaces.
pub fn escape_heading_text(text: &str) -> String {
    escape_markdown_inline(&fold_newlines_to_space(text))
}

/// Escape paragraph body while preserving intentional line breaks between paragraphs.
pub fn escape_markdown_paragraph(text: &str) -> String {
    text.lines()
        .map(escape_markdown_inline)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Inline-safe escaping for list items and short fields.
pub fn escape_markdown_inline(text: &str) -> String {
    let html_safe = escape_html_entities(text);
    escape_markdown_metacharacters(&html_safe)
}

fn fold_newlines_to_space(text: &str) -> String {
    text.split(['\r', '\n'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_markdown_metacharacters(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(
            ch,
            '\\' | '`'
                | '*'
                | '_'
                | '{'
                | '}'
                | '['
                | ']'
                | '('
                | ')'
                | '#'
                | '+'
                | '-'
                | '.'
                | '!'
                | '|'
                | '<'
                | '>'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn escape_html_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::types::VideoMetadata;

    fn sample_data() -> NoteData {
        NoteData {
            metadata: VideoMetadata {
                title: "<script>alert(1)</script>".to_string(),
                part_title: Some("# Part".to_string()),
                bvid: "BV1".to_string(),
                duration_ms: 5000,
            },
            summary: "Line one\n<script>".to_string(),
            sections: vec![NoteSection {
                start_ms: 3661000,
                end_ms: 3662000,
                title: "## evil".to_string(),
                explanation: "<b>ok</b>".to_string(),
            }],
            segments: vec![SubtitleSegment {
                start_ms: 1000,
                end_ms: 2000,
                text: "原文\n保留".to_string(),
            }],
        }
    }

    #[test]
    fn markdown_escapes_html_and_heading_injection() {
        let md = render_markdown(&sample_data(), NoteLocale::Zh);
        assert!(!md.contains("<script>"));
        assert!(md.contains("&lt;script&gt;"));
        assert!(md.contains("\\#\\# evil"));
        assert!(md.contains("&lt;b&gt;ok&lt;/b&gt;"));
    }

    #[test]
    fn markdown_localizes_headings() {
        let md = render_markdown(&sample_data(), NoteLocale::En);
        assert!(md.contains("## Summary"));
        assert!(md.contains("## Full transcript"));
    }

    #[test]
    fn markdown_formats_over_one_hour() {
        let md = render_markdown(&sample_data(), NoteLocale::Zh);
        assert!(md.contains("01:01:01"));
    }

    #[test]
    fn transcript_preserves_source_text_semantics() {
        let md = render_markdown(&sample_data(), NoteLocale::Zh);
        assert!(md.contains("原文"));
        assert!(md.contains("保留"));
        assert!(!md.contains("原文\n保留"));
    }

    #[test]
    fn paragraph_blocks_injection_after_newline() {
        let md = render_markdown(
            &NoteData {
                summary: "safe\n# injected heading\n- list item\n> quote".to_string(),
                ..sample_data()
            },
            NoteLocale::En,
        );
        let summary_section = md
            .split("## Section breakdown")
            .next()
            .expect("summary section");
        assert!(!summary_section.contains("\n# injected"));
        assert!(summary_section.contains("\\# injected"));
        assert!(summary_section.contains("\\- list"));
        assert!(summary_section.contains("&gt; quote"));
    }

    #[test]
    fn transcript_escapes_markdown_metacharacters() {
        let md = render_markdown(
            &NoteData {
                segments: vec![SubtitleSegment {
                    start_ms: 0,
                    end_ms: 1000,
                    text: "`- code\n[click](javascript:alert(1))".to_string(),
                }],
                ..sample_data()
            },
            NoteLocale::En,
        );
        assert!(md.contains("\\`\\- code"));
        assert!(md.contains("\\[click\\]"));
        assert!(md.contains("javascript:alert"));
        assert!(!md.contains("\n[click]"));
    }

    #[test]
    fn heading_folds_internal_newlines() {
        let escaped = escape_heading_text("Title\n# break");
        assert!(!escaped.contains('\n'));
        assert!(escaped.contains("Title"));
        assert!(escaped.contains("\\# break"));
    }
}
