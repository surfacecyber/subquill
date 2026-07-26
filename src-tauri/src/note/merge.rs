use super::types::{ChunkLlmOutput, NoteSection};

/// Merge chunk outputs: concatenate sections, sort, and deduplicate exact overlaps.
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
        if let Some(last) = deduped.last() {
            if sections_equivalent(last, &section) {
                continue;
            }
        }
        deduped.push(section);
    }
    deduped
}

fn sections_equivalent(a: &NoteSection, b: &NoteSection) -> bool {
    a.start_ms == b.start_ms
        && a.end_ms == b.end_ms
        && a.title.trim().eq_ignore_ascii_case(b.title.trim())
}

#[derive(Debug, serde::Deserialize)]
struct SummaryOnly {
    summary: String,
}

pub fn parse_summary_merge_output(content: &str) -> Result<String, crate::llm::LlmError> {
    let json_text = crate::llm::extract_json_text(content)?;
    let raw: SummaryOnly = serde_json::from_str(&json_text).map_err(|_| {
        crate::llm::LlmError::invalid_response("summary merge output is not valid JSON")
    })?;
    let summary = raw.summary.trim();
    if summary.is_empty() {
        return Err(crate::llm::LlmError::invalid_response(
            "merged summary must not be empty",
        ));
    }
    Ok(summary.to_string())
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
    fn fallback_merge_joins_partials_in_order() {
        use super::super::types::NoteLocale;

        let merged =
            fallback_merge_summaries(&["first".to_string(), "second".to_string()], NoteLocale::En);
        assert_eq!(merged, "first\n\nsecond");
    }
}
