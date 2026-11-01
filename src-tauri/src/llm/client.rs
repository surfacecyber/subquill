use std::fmt;

use url::Url;

use crate::redact::REDACTED;

use super::error::{LlmError, Result};
use super::http::{HttpRequest, HttpResponse, HttpTransport};
use super::parse::extract_json_text;
use super::sanitize::sanitize_api_error_message;
use super::sse::{apply_chunk_payload, completion_message_content};
use super::types::{ChatApiErrorBody, ChatCompletionRequest, ChatMessage, ChatStreamEvent};
use super::validate::{validate_llm_base_url, validate_llm_model};

/// Build `<base_url>/chat/completions` without duplicating path segments such as `/v1/v1`.
pub fn chat_completions_url(base_url: &str) -> Result<Url> {
    let normalized = validate_llm_base_url(base_url)?;

    let endpoint = if normalized.ends_with("/chat/completions") {
        normalized
    } else {
        format!("{normalized}/chat/completions")
    };

    Url::parse(&endpoint).map_err(|_| LlmError::api("failed to construct chat/completions URL"))
}

#[derive(Clone)]
pub struct LlmClientConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl fmt::Debug for LlmClientConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LlmClientConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &REDACTED)
            .field("model", &self.model)
            .finish()
    }
}

pub struct LlmClient<T: HttpTransport> {
    transport: T,
    endpoint: Url,
    api_key: String,
    model: String,
}

impl<T: HttpTransport> fmt::Debug for LlmClient<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LlmClient")
            .field("endpoint", &self.endpoint)
            .field("api_key", &REDACTED)
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl<T: HttpTransport> LlmClient<T> {
    pub fn new(config: LlmClientConfig, transport: T) -> Result<Self> {
        let base_url = validate_llm_base_url(&config.base_url)?;
        let model = validate_llm_model(&config.model)?;
        let endpoint = chat_completions_url(&base_url)?;

        let api_key = config.api_key.trim();
        if api_key.is_empty() {
            return Err(LlmError::auth("API key is required"));
        }

        Ok(Self {
            transport,
            endpoint,
            api_key: api_key.to_string(),
            model,
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        let body = serde_json::to_string(&request)
            .map_err(|_| LlmError::api("failed to encode chat request"))?;

        let response = self.transport.send(HttpRequest {
            url: self.endpoint.to_string(),
            body,
            api_key: self.api_key.clone(),
            timeout: None,
        })?;

        map_http_response(response, &self.api_key)
    }

    /// Stream a chat completion. `on_event` receives accumulated visible
    /// content (not reasoning). Cancellation is checked on each SSE payload.
    pub fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        mut on_event: impl FnMut(&ChatStreamEvent),
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<String> {
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream: true,
        };

        let body = serde_json::to_string(&request)
            .map_err(|_| LlmError::api("failed to encode chat request"))?;

        let mut content = String::new();
        let mut thinking = false;
        let mut last_sig: Option<(usize, bool)> = None;

        let response = self.transport.send_sse(
            HttpRequest {
                url: self.endpoint.to_string(),
                body,
                api_key: self.api_key.clone(),
                timeout: None,
            },
            &mut |payload| {
                apply_chunk_payload(&mut content, &mut thinking, payload)?;
                let visible_thinking = thinking && content.is_empty();
                let sig = (content.len(), visible_thinking);
                if last_sig != Some(sig) {
                    on_event(&ChatStreamEvent {
                        content: content.clone(),
                        thinking: visible_thinking,
                    });
                    last_sig = Some(sig);
                }
                Ok(())
            },
            &mut cancelled,
        )?;

        if cancelled() {
            return Err(LlmError::cancelled("Note generation was cancelled"));
        }

        if !response.is_success() {
            return map_http_response(response, &self.api_key).map(|_| String::new());
        }

        if content.trim().is_empty() {
            if let Some(fallback) = completion_message_content(&response.body) {
                content = fallback;
                on_event(&ChatStreamEvent {
                    content: content.clone(),
                    thinking: false,
                });
            }
        }

        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(LlmError::invalid_response(
                "LLM response missing message content",
            ));
        }

        Ok(trimmed.to_string())
    }

    /// Minimal connectivity check for settings UI — JSON-only system prompt.
    pub fn test_connection(&self) -> Result<()> {
        let messages = vec![
            ChatMessage::system("Reply with a JSON object only: {\"ok\":true}. No other text."),
            ChatMessage::user("ping"),
        ];

        let content = self.chat(messages)?;
        let json_text = extract_json_text(&content)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_text)
            .map_err(|_| LlmError::invalid_response("test response is not valid JSON"))?;

        if parsed.get("ok") != Some(&serde_json::Value::Bool(true)) {
            return Err(LlmError::invalid_response(
                "test response JSON missing ok:true",
            ));
        }

        Ok(())
    }
}

fn map_http_response(response: HttpResponse, api_key: &str) -> Result<String> {
    if response.status == 401 || response.status == 403 {
        return Err(LlmError::auth("LLM API rejected credentials"));
    }
    if response.status == 429 {
        return Err(LlmError::rate_limited("LLM API rate limit exceeded"));
    }
    if !response.is_success() {
        let message = parse_api_error_message(&response.body, api_key)
            .unwrap_or_else(|| format!("LLM API returned HTTP {}", response.status));
        return Err(LlmError::api(message));
    }

    let content = completion_message_content(&response.body)
        .ok_or_else(|| LlmError::invalid_response("LLM response missing message content"))?;

    Ok(content)
}

fn parse_api_error_message(body: &str, api_key: &str) -> Option<String> {
    let parsed: ChatApiErrorBody = serde_json::from_str(body).ok()?;
    let message = parsed.error.message.trim();
    if message.is_empty() {
        None
    } else {
        Some(sanitize_api_error_message(message, api_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::http::mock::MockResponse;
    use crate::llm::http::{HttpRequest, HttpResponse};
    use crate::redact::REDACTED;

    fn client_with_mock(
        responses: Vec<MockResponse>,
    ) -> LlmClient<crate::llm::http::mock::MockTransport> {
        let (transport, _) = crate::llm::http::mock::MockTransport::new(responses);
        LlmClient::new(
            LlmClientConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                model: "test-model".to_string(),
            },
            transport,
        )
        .expect("client")
    }

    fn success_body(content: &str) -> String {
        serde_json::json!({
            "choices": [{
                "message": { "content": content }
            }]
        })
        .to_string()
    }

    #[test]
    fn chat_completions_url_appends_path_safely() {
        let url = chat_completions_url("https://api.openai.com/v1").unwrap();
        assert_eq!(url.as_str(), "https://api.openai.com/v1/chat/completions");

        let local = chat_completions_url("http://localhost:11434/v1/").unwrap();
        assert_eq!(local.as_str(), "http://localhost:11434/v1/chat/completions");
    }

    #[test]
    fn chat_completions_url_no_double_v1() {
        let url = chat_completions_url("https://example.com/v1").unwrap();
        assert!(!url.as_str().contains("/v1/v1"));
    }

    #[test]
    fn chat_completions_url_rejects_query() {
        let err = chat_completions_url("https://api.example.com/v1?x=1").unwrap_err();
        assert_eq!(err.code(), "LLM_API_ERROR");
    }

    #[test]
    fn llm_client_config_debug_redacts_api_key() {
        let config = LlmClientConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-secret-config-key".to_string(),
            model: "m".to_string(),
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("sk-secret-config-key"));
        assert!(debug.contains(REDACTED));
    }

    #[test]
    fn maps_401_to_auth_error() {
        let client = client_with_mock(vec![MockResponse::success(
            r#"{"error":{"message":"bad key"}}"#,
        )
        .with_status(401)]);
        let err = client.chat(vec![ChatMessage::user("hi")]).unwrap_err();
        assert_eq!(err.code(), "LLM_AUTH_ERROR");
    }

    #[test]
    fn maps_429_to_rate_limited() {
        let client = client_with_mock(vec![MockResponse::success("").with_status(429)]);
        let err = client.chat(vec![ChatMessage::user("hi")]).unwrap_err();
        assert_eq!(err.code(), "LLM_RATE_LIMITED");
    }

    #[test]
    fn maps_api_error_status() {
        let client = client_with_mock(vec![MockResponse::success(
            r#"{"error":{"message":"model not found"}}"#,
        )
        .with_status(400)]);
        let err = client.chat(vec![ChatMessage::user("hi")]).unwrap_err();
        assert_eq!(err.code(), "LLM_API_ERROR");
        assert_eq!(err.to_string(), "model not found");
    }

    #[test]
    fn api_error_redacts_api_key_echo() {
        let secret = "sk-echoed-in-error-body";
        let body = format!(r#"{{"error":{{"message":"Invalid key {secret}"}}}}"#);
        let client = LlmClient::new(
            LlmClientConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: secret.to_string(),
                model: "m".to_string(),
            },
            crate::llm::http::mock::MockTransport::new(vec![
                MockResponse::success(body).with_status(400)
            ])
            .0,
        )
        .expect("client");

        let err = client.chat(vec![ChatMessage::user("hi")]).unwrap_err();
        assert_eq!(err.code(), "LLM_API_ERROR");
        assert!(!err.to_string().contains(secret));
        assert!(err.to_string().contains(REDACTED));
    }

    #[test]
    fn extracts_message_content() {
        let client = client_with_mock(vec![MockResponse::success(success_body(r#"{"ok":true}"#))]);
        let content = client.chat(vec![ChatMessage::user("hi")]).unwrap();
        assert_eq!(content, r#"{"ok":true}"#);
    }

    #[test]
    fn chat_stream_replays_buffered_json_and_sets_stream_flag() {
        let (transport, requests_log) =
            crate::llm::http::mock::MockTransport::new(vec![MockResponse::success(success_body(
                "## Summary\n\nHello",
            ))]);
        let client = LlmClient::new(
            LlmClientConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                model: "test-model".to_string(),
            },
            transport,
        )
        .expect("client");

        let mut events = Vec::new();
        let content = client
            .chat_stream(
                vec![ChatMessage::user("hi")],
                |event| events.push(event.clone()),
                || false,
            )
            .unwrap();

        assert_eq!(content, "## Summary\n\nHello");
        assert_eq!(
            events.last().map(|e| e.content.as_str()),
            Some("## Summary\n\nHello")
        );
        let requests = requests_log.lock().expect("lock").clone();
        assert!(requests[0].body.contains("\"stream\":true"));
    }

    #[test]
    fn chat_stream_cancel_before_send() {
        let client = client_with_mock(vec![MockResponse::success(success_body("hi"))]);
        let err = client
            .chat_stream(vec![ChatMessage::user("hi")], |_| {}, || true)
            .unwrap_err();
        assert_eq!(err.code(), "JOB_CANCELLED");
    }

    #[test]
    fn rejects_empty_choices() {
        let client = client_with_mock(vec![MockResponse::success(r#"{"choices":[]}"#)]);
        let err = client.chat(vec![ChatMessage::user("hi")]).unwrap_err();
        assert_eq!(err.code(), "LLM_INVALID_RESPONSE");
    }

    #[test]
    fn test_connection_accepts_fenced_json() {
        let client = client_with_mock(vec![MockResponse::success(success_body(
            "```json\n{\"ok\":true}\n```",
        ))]);
        client.test_connection().expect("ok");
    }

    #[test]
    fn maps_timeout_and_network_from_transport() {
        struct TimeoutTransport;
        impl crate::llm::HttpTransport for TimeoutTransport {
            fn send(&self, _: HttpRequest) -> Result<HttpResponse> {
                Err(LlmError::timeout("LLM request timed out"))
            }
        }

        let client = LlmClient::new(
            LlmClientConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                model: "m".to_string(),
            },
            TimeoutTransport,
        )
        .expect("client");

        let err = client.chat(vec![ChatMessage::user("hi")]).unwrap_err();
        assert_eq!(err.code(), "LLM_TIMEOUT");

        struct NetworkTransport;
        impl crate::llm::HttpTransport for NetworkTransport {
            fn send(&self, _: HttpRequest) -> Result<HttpResponse> {
                Err(LlmError::network("LLM network request failed"))
            }
        }

        let client = LlmClient::new(
            LlmClientConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                model: "m".to_string(),
            },
            NetworkTransport,
        )
        .expect("client");

        let err = client.chat(vec![ChatMessage::user("hi")]).unwrap_err();
        assert_eq!(err.code(), "LLM_NETWORK_ERROR");
    }

    #[test]
    fn request_body_does_not_contain_api_key() {
        let (transport, requests_log) =
            crate::llm::http::mock::MockTransport::new(vec![MockResponse::success(success_body(
                r#"{"ok":true}"#,
            ))]);
        let client = LlmClient::new(
            LlmClientConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-secret-key".to_string(),
                model: "m".to_string(),
            },
            transport,
        )
        .expect("client");

        client.test_connection().expect("ok");
        let requests = requests_log.lock().expect("lock").clone();
        assert_eq!(requests.len(), 1);
        assert!(!requests[0].body.contains("sk-secret-key"));
        assert!(!requests[0].body.contains("max_tokens"));
        assert!(!requests[0].body.contains("thinking"));
    }
}
