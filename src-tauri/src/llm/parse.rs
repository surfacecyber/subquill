use super::error::{LlmError, Result};

/// Extract JSON text from model output: raw JSON object or a single markdown fence.
pub fn extract_json_text(content: &str) -> Result<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(LlmError::invalid_response("model content is empty"));
    }

    if trimmed.starts_with('{') {
        return Ok(trimmed.to_string());
    }

    if trimmed.starts_with("```") {
        return extract_from_fence(trimmed);
    }

    Err(LlmError::invalid_response(
        "model content is not JSON or a fenced JSON block",
    ))
}

fn extract_from_fence(trimmed: &str) -> Result<String> {
    let after_open = trimmed.strip_prefix("```").expect("fence prefix checked");

    let body = after_open
        .strip_prefix("json")
        .unwrap_or(after_open)
        .trim_start_matches(['\r', '\n', ' ']);

    let close_idx = body
        .rfind("```")
        .ok_or_else(|| LlmError::invalid_response("fenced JSON block has no closing fence"))?;

    let inner = body[..close_idx].trim();
    if inner.is_empty() {
        return Err(LlmError::invalid_response("fenced JSON block is empty"));
    }

    if !inner.starts_with('{') {
        return Err(LlmError::invalid_response(
            "fenced block does not contain a JSON object",
        ));
    }

    Ok(inner.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_raw_json() {
        let json = extract_json_text(r#"  {"a":1}  "#).unwrap();
        assert_eq!(json, r#"{"a":1}"#);
    }

    #[test]
    fn extract_fenced_json() {
        let input = "```json\n{\"summary\":\"hi\",\"sections\":[]}\n```";
        let json = extract_json_text(input).unwrap();
        assert!(json.contains("summary"));
    }

    #[test]
    fn extract_fenced_without_lang() {
        let input = "```\n{\"ok\":true}\n```";
        let json = extract_json_text(input).unwrap();
        assert_eq!(json, r#"{"ok":true}"#);
    }

    #[test]
    fn reject_plain_text() {
        let err = extract_json_text("not json").unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }

    #[test]
    fn reject_unclosed_fence() {
        let err = extract_json_text("```json\n{\"a\":1}").unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }
}
