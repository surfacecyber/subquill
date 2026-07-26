use crate::llm::{extract_json_text, LlmError, LlmResult};

use super::types::{ChunkLlmOutput, ChunkLlmOutputRaw, NoteSection, SubtitleChunk};

pub fn parse_and_validate_chunk_output(
    content: &str,
    chunk: &SubtitleChunk,
    video_duration_ms: u64,
) -> LlmResult<ChunkLlmOutput> {
    let json_text = extract_json_text(content)?;
    let raw: ChunkLlmOutputRaw = serde_json::from_str(&json_text).map_err(|_| {
        LlmError::invalid_response("chunk output is not valid JSON for expected schema")
    })?;

    validate_chunk_output(&raw, chunk, video_duration_ms)?;
    Ok(ChunkLlmOutput {
        summary: raw.summary.trim().to_string(),
        sections: raw.sections,
    })
}

pub fn validate_chunk_output(
    raw: &ChunkLlmOutputRaw,
    chunk: &SubtitleChunk,
    video_duration_ms: u64,
) -> LlmResult<()> {
    let summary = raw.summary.trim();
    if summary.is_empty() {
        return Err(LlmError::invalid_response("summary must not be empty"));
    }

    if raw.sections.is_empty() {
        return Err(LlmError::invalid_response("sections must not be empty"));
    }

    let mut last_start = None;
    for section in &raw.sections {
        validate_section(section, chunk, video_duration_ms)?;
        if let Some(prev) = last_start {
            if section.start_ms < prev {
                return Err(LlmError::invalid_response(
                    "sections must be sorted by start_ms",
                ));
            }
        }
        last_start = Some(section.start_ms);
    }

    Ok(())
}

fn validate_section(
    section: &NoteSection,
    chunk: &SubtitleChunk,
    video_duration_ms: u64,
) -> LlmResult<()> {
    if section.title.trim().is_empty() {
        return Err(LlmError::invalid_response(
            "section title must not be empty",
        ));
    }
    if section.explanation.trim().is_empty() {
        return Err(LlmError::invalid_response(
            "section explanation must not be empty",
        ));
    }

    if section.end_ms <= section.start_ms {
        return Err(LlmError::invalid_response(
            "section end_ms must be greater than start_ms",
        ));
    }

    if section.start_ms < chunk.start_ms || section.end_ms > chunk.end_ms {
        return Err(LlmError::invalid_response(
            "section times must fall within chunk range",
        ));
    }

    if section.end_ms > video_duration_ms {
        return Err(LlmError::invalid_response(
            "section times must not exceed video duration",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::SubtitleSegment;

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

    fn valid_json() -> String {
        r#"{"summary":"s","sections":[{"start_ms":1000,"end_ms":2000,"title":"t","explanation":"e"}]}"#
            .to_string()
    }

    #[test]
    fn accepts_valid_chunk_output() {
        let output = parse_and_validate_chunk_output(&valid_json(), &chunk(), 10_000).unwrap();
        assert_eq!(output.summary, "s");
        assert_eq!(output.sections.len(), 1);
    }

    #[test]
    fn rejects_empty_summary() {
        let json = r#"{"summary":"  ","sections":[{"start_ms":1000,"end_ms":2000,"title":"t","explanation":"e"}]}"#;
        let err = parse_and_validate_chunk_output(json, &chunk(), 10_000).unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }

    #[test]
    fn rejects_out_of_chunk_bounds() {
        let json = r#"{"summary":"s","sections":[{"start_ms":500,"end_ms":2000,"title":"t","explanation":"e"}]}"#;
        let err = parse_and_validate_chunk_output(json, &chunk(), 10_000).unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }

    #[test]
    fn rejects_unsorted_sections() {
        let json = r#"{"summary":"s","sections":[
            {"start_ms":3000,"end_ms":4000,"title":"a","explanation":"e"},
            {"start_ms":1000,"end_ms":2000,"title":"b","explanation":"e"}
        ]}"#;
        let err = parse_and_validate_chunk_output(json, &chunk(), 10_000).unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }
}
