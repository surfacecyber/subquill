use super::chunk::ends_complete_sentence;
use super::types::{ChunkLlmOutput, NoteSection};

/// Merge chunk outputs: concatenate sections, sort, and collapse overlapping spans.
pub fn merge_chunk_outputs(outputs: &[ChunkLlmOutput]) -> (Vec<NoteSection>, Vec<String>) {
    let mut sections: Vec<NoteSection> = outputs
        .iter()
        .flat_map(|output| output.sections.clone())
        .collect();
    let partial_summaries: Vec<String> = outputs.iter().map(|o| o.summary.clone()).collect();

    sections.sort_by_key(|section| section.start_ms);
    sections = dedupe_sections(sections);

    (sections, partial_summaries)
}

fn dedupe_sections(sections: Vec<NoteSection>) -> Vec<NoteSection> {
    let mut deduped = Vec::new();
    for section in sections {
        if let Some(last) = deduped.last_mut() {
            if should_merge_sections(last, &section) {
                *last = merge_section_pair(last.clone(), section);
                continue;
            }
        }
        deduped.push(section);
    }
    deduped
}

fn should_merge_sections(a: &NoteSection, b: &NoteSection) -> bool {
    if sections_same_span_title(a, b) {
        return true;
    }
    let overlap = overlap_ms(a, b);
    if overlap == 0 {
        return false;
    }
    titles_similar(a, b) || overlap * 2 >= shorter_span_ms(a, b)
}

fn merge_section_pair(a: NoteSection, b: NoteSection) -> NoteSection {
    NoteSection {
        start_ms: a.start_ms.min(b.start_ms),
        end_ms: a.end_ms.max(b.end_ms),
        title: prefer_title(&a.title, &b.title),
        explanation: prefer_explanation(&a.explanation, &b.explanation),
    }
}

fn prefer_title(a: &str, b: &str) -> String {
    if a.trim().chars().count() >= b.trim().chars().count() {
        a.trim().to_string()
    } else {
        b.trim().to_string()
    }
}

fn prefer_explanation(a: &str, b: &str) -> String {
    let a_complete = ends_complete_sentence(a);
    let b_complete = ends_complete_sentence(b);
    match (a_complete, b_complete) {
        (true, false) => a.trim().to_string(),
        (false, true) => b.trim().to_string(),
        _ if b.trim().chars().count() > a.trim().chars().count() => b.trim().to_string(),
        _ => a.trim().to_string(),
    }
}

fn overlap_ms(a: &NoteSection, b: &NoteSection) -> u64 {
    let start = a.start_ms.max(b.start_ms);
    let end = a.end_ms.min(b.end_ms);
    end.saturating_sub(start)
}

fn shorter_span_ms(a: &NoteSection, b: &NoteSection) -> u64 {
    a.end_ms
        .saturating_sub(a.start_ms)
        .min(b.end_ms.saturating_sub(b.start_ms))
        .max(1)
}

fn titles_similar(a: &NoteSection, b: &NoteSection) -> bool {
    a.title.trim().eq_ignore_ascii_case(b.title.trim())
}

fn sections_same_span_title(a: &NoteSection, b: &NoteSection) -> bool {
    a.start_ms == b.start_ms && a.end_ms == b.end_ms && titles_similar(a, b)
}

#[derive(Debug, serde::Deserialize)]
struct SummaryOnly {
    summary: String,
}

pub fn parse_summary_merge_output(content: &str) -> Result<String, crate::llm::LlmError> {
    let text = super::parse_md::unwrap_markdown_fence(content);
    if text.trim().is_empty() {
        return Err(crate::llm::LlmError::invalid_response(
            "merged summary must not be empty",
        ));
    }

    if text.trim_start().starts_with('{') {
        if let Ok(json_text) = crate::llm::extract_json_text(text) {
            if let Ok(raw) = serde_json::from_str::<SummaryOnly>(&json_text) {
                let summary = raw.summary.trim();
                if summary.is_empty() {
                    return Err(crate::llm::LlmError::invalid_response(
                        "merged summary must not be empty",
                    ));
                }
                return Ok(summary.to_string());
            }
        }
    }

    let summary = extract_merged_summary_text(text);
    if summary.is_empty() {
        return Err(crate::llm::LlmError::invalid_response(
            "merged summary must not be empty",
        ));
    }
    Ok(summary)
}

fn extract_merged_summary_text(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        lines.push(line);
    }
    lines.join("\n").trim().to_string()
}

/// Deterministic fallback when LLM summary merge cannot be validated.
pub fn fallback_merge_summaries(partials: &[String], locale: super::types::NoteLocale) -> String {
    let cleaned: Vec<String> = partials
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();

    if cleaned.is_empty() {
        return String::new();
    }

    match locale {
        super::types::NoteLocale::Zh => cleaned.join("\n\n"),
        super::types::NoteLocale::En => cleaned.join("\n\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(start: u64, end: u64, title: &str) -> NoteSection {
        NoteSection {
            start_ms: start,
            end_ms: end,
            title: title.to_string(),
            explanation: "e".to_string(),
        }
    }

    #[test]
    fn merge_sorts_sections() {
        let outputs = vec![
            ChunkLlmOutput {
                summary: "a".to_string(),
                sections: vec![section(3000, 4000, "late")],
            },
            ChunkLlmOutput {
                summary: "b".to_string(),
                sections: vec![section(1000, 2000, "early")],
            },
        ];
        let (sections, summaries) = merge_chunk_outputs(&outputs);
        assert_eq!(sections[0].start_ms, 1000);
        assert_eq!(sections[1].start_ms, 3000);
        assert_eq!(summaries, vec!["a", "b"]);
    }

    #[test]
    fn merge_dedupes_identical_sections() {
        let outputs = vec![
            ChunkLlmOutput {
                summary: "a".to_string(),
                sections: vec![section(1000, 2000, "Same")],
            },
            ChunkLlmOutput {
                summary: "b".to_string(),
                sections: vec![section(1000, 2000, "same")],
            },
        ];
        let (sections, _) = merge_chunk_outputs(&outputs);
        assert_eq!(sections.len(), 1);
    }

    #[test]
    fn merge_keeps_richer_explanation_on_overlap() {
        let short = NoteSection {
            start_ms: 1000,
            end_ms: 2000,
            title: "Same".to_string(),
            explanation: "short".to_string(),
        };
        let long = NoteSection {
            start_ms: 1000,
            end_ms: 2000,
            title: "same".to_string(),
            explanation: "a much longer explanation with detail".to_string(),
        };
        let outputs = vec![
            ChunkLlmOutput {
                summary: "a".to_string(),
                sections: vec![short],
            },
            ChunkLlmOutput {
                summary: "b".to_string(),
                sections: vec![long.clone()],
            },
        ];
        let (sections, _) = merge_chunk_outputs(&outputs);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].explanation, long.explanation);
    }

    #[test]
    fn fallback_merge_joins_partials_in_order() {
        use super::super::types::NoteLocale;

        let merged =
            fallback_merge_summaries(&["first".to_string(), "second".to_string()], NoteLocale::En);
        assert_eq!(merged, "first\n\nsecond");
    }
}
