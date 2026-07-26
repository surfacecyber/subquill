use url::Url;

use super::error::{LlmError, Result};

/// Validates an OpenAI-compatible API v1 root URL for LLM requests.
pub fn validate_llm_base_url(base_url: &str) -> Result<String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(LlmError::api("base_url is required for LLM requests"));
    }

    let parsed = Url::parse(trimmed)
        .map_err(|_| LlmError::api("base_url is not a valid URL for LLM requests"))?;

    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err(LlmError::api("base_url must use http or https")),
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(LlmError::api("base_url must not contain credentials"));
    }

    if parsed.fragment().is_some() {
        return Err(LlmError::api("base_url must not contain a fragment"));
    }

    if parsed.query().is_some() {
        return Err(LlmError::api("base_url must not contain a query string"));
    }

    parsed
        .host_str()
        .filter(|host| !host.is_empty())
        .ok_or_else(|| LlmError::api("base_url must include a host"))?;

    let mut normalized = format!(
        "{}://{}",
        parsed.scheme(),
        parsed.host_str().expect("host checked")
    );

    if let Some(port) = parsed.port() {
        let is_default = (parsed.scheme() == "http" && port == 80)
            || (parsed.scheme() == "https" && port == 443);
        if !is_default {
            normalized.push(':');
            normalized.push_str(&port.to_string());
        }
    }

    let path = parsed.path().trim_end_matches('/');
    if !path.is_empty() {
        normalized.push_str(path);
    }

    Ok(normalized)
}

pub fn validate_llm_model(model: &str) -> Result<String> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return Err(LlmError::api("model is required"));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_query_string() {
        let err = validate_llm_base_url("https://api.example.com/v1?key=1").unwrap_err();
        assert_eq!(err.code(), "LLM_API_ERROR");
    }

    #[test]
    fn rejects_userinfo() {
        let err = validate_llm_base_url("https://user:pass@example.com/v1").unwrap_err();
        assert_eq!(err.code(), "LLM_API_ERROR");
    }

    #[test]
    fn rejects_empty_model() {
        let err = validate_llm_model("  ").unwrap_err();
        assert_eq!(err.code(), "LLM_API_ERROR");
    }

    #[test]
    fn accepts_localhost_v1() {
        let url = validate_llm_base_url("http://localhost:11434/v1/").unwrap();
        assert_eq!(url, "http://localhost:11434/v1");
    }
}
