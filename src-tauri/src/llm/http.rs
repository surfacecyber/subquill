use std::fmt;
use std::io::Read;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::redact::REDACTED;

use super::bounded::{bytes_to_utf8_body, check_content_length, collect_bounded_body};
use super::error::{LlmError, Result};

pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Client hang protection. Matches the per-item job deadline so a long
/// generation is not cut off before the job itself times out.
pub const DEFAULT_TOTAL_TIMEOUT: Duration = Duration::from_secs(30 * 60);
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub body: String,
    pub api_key: String,
    /// Optional per-request timeout; falls back to the transport default.
    pub timeout: Option<Duration>,
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpRequest")
            .field("url", &self.url)
            .field("body", &self.body)
            .field("api_key", &REDACTED)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

pub trait HttpTransport: Send + Sync {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse>;

    /// Stream a chat completion. Default implementation buffers via [`send`]
    /// then replays the body as SSE events (JSON completions become one delta).
    fn send_sse(
        &self,
        request: HttpRequest,
        on_event: &mut dyn FnMut(&str) -> Result<()>,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<HttpResponse> {
        if cancelled() {
            return Err(LlmError::cancelled("Note generation was cancelled"));
        }
        let response = self.send(request)?;
        if response.is_success() {
            super::sse::replay_body_as_sse_events(&response.body, on_event)?;
        }
        Ok(response)
    }
}

pub fn build_headers(api_key: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key.trim()))
            .map_err(|_| LlmError::auth("invalid API key header value"))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(headers)
}

pub struct ReqwestTransport {
    client: Client,
}

impl ReqwestTransport {
    pub fn new(connect_timeout: Duration, total_timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(connect_timeout)
            .timeout(total_timeout)
            .build()
            .map_err(|err| LlmError::network(err.to_string()))?;

        Ok(Self { client })
    }

    pub fn with_defaults() -> Result<Self> {
        Self::new(DEFAULT_CONNECT_TIMEOUT, DEFAULT_TOTAL_TIMEOUT)
    }

    fn dispatch(&self, request: &HttpRequest) -> Result<reqwest::blocking::Response> {
        let headers = build_headers(&request.api_key)?;

        let mut builder = self
            .client
            .post(&request.url)
            .headers(headers)
            .body(request.body.clone());
        if let Some(timeout) = request.timeout {
            builder = builder.timeout(timeout);
        }

        builder.send().map_err(map_reqwest_error)
    }
}

impl HttpTransport for ReqwestTransport {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
        let mut response = self.dispatch(&request)?;
        let status = response.status().as_u16();
        check_content_length(response.content_length())?;
        let body = read_bounded_response_body(&mut response)?;
        Ok(HttpResponse { status, body })
    }

    fn send_sse(
        &self,
        request: HttpRequest,
        on_event: &mut dyn FnMut(&str) -> Result<()>,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<HttpResponse> {
        if cancelled() {
            return Err(LlmError::cancelled("Note generation was cancelled"));
        }

        let mut response = self.dispatch(&request)?;
        let status = response.status().as_u16();

        if !(200..300).contains(&status) {
            check_content_length(response.content_length())?;
            let body = read_bounded_response_body(&mut response)?;
            return Ok(HttpResponse { status, body });
        }

        read_sse_response_body(&mut response, on_event, cancelled)?;
        Ok(HttpResponse {
            status,
            body: String::new(),
        })
    }
}

fn read_bounded_response_body(response: &mut reqwest::blocking::Response) -> Result<String> {
    let mut buf = [0u8; 8192];
    let chunks = std::iter::from_fn(|| match response.read(&mut buf) {
        Ok(0) => None,
        Ok(n) => Some(Ok(buf[..n].to_vec())),
        Err(_) => Some(Err(LlmError::network("LLM response read failed"))),
    });

    let bytes = collect_bounded_body(chunks)?;
    bytes_to_utf8_body(bytes)
}

fn read_sse_response_body(
    response: &mut reqwest::blocking::Response,
    on_event: &mut dyn FnMut(&str) -> Result<()>,
    cancelled: &mut dyn FnMut() -> bool,
) -> Result<()> {
    let mut parser = super::sse::SseParser::new();
    let mut buf = [0u8; 8192];
    let mut byte_leftover = Vec::new();
    let mut saw_event = false;

    loop {
        if cancelled() {
            return Err(LlmError::cancelled("Note generation was cancelled"));
        }

        let n = response
            .read(&mut buf)
            .map_err(|_| LlmError::network("LLM response read failed"))?;
        if n == 0 {
            break;
        }

        byte_leftover.extend_from_slice(&buf[..n]);
        let valid_up_to = match std::str::from_utf8(&byte_leftover) {
            Ok(_) => byte_leftover.len(),
            Err(err) => err.valid_up_to(),
        };
        if valid_up_to == 0 && byte_leftover.len() > 4 {
            return Err(LlmError::invalid_response(
                "LLM response body is not valid UTF-8",
            ));
        }

        let chunk = std::str::from_utf8(&byte_leftover[..valid_up_to])
            .map_err(|_| LlmError::invalid_response("LLM response body is not valid UTF-8"))?;
        parser.push(chunk, &mut |payload| {
            saw_event = true;
            on_event(payload)
        })?;
        byte_leftover.drain(..valid_up_to);
    }

    if !byte_leftover.is_empty() {
        return Err(LlmError::invalid_response(
            "LLM response body is not valid UTF-8",
        ));
    }

    if !saw_event {
        let leftover = parser.leftover().to_string();
        if leftover.trim().starts_with('{') {
            return super::sse::replay_body_as_sse_events(&leftover, on_event);
        }
    }

    parser.finish(on_event)
}

fn map_reqwest_error(err: reqwest::Error) -> LlmError {
    if err.is_timeout() {
        LlmError::timeout("LLM request timed out")
    } else if err.is_connect() || err.is_request() {
        LlmError::network("LLM network request failed")
    } else {
        LlmError::network("LLM HTTP transport failed")
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    pub struct MockResponse {
        pub status: u16,
        pub body: String,
    }

    impl MockResponse {
        pub fn success(body: impl Into<String>) -> Self {
            Self {
                status: 200,
                body: body.into(),
            }
        }

        pub fn with_status(mut self, status: u16) -> Self {
            self.status = status;
            self
        }
    }

    #[derive(Debug)]
    pub struct MockTransport {
        responses: Vec<MockResponse>,
        next_index: Mutex<usize>,
        requests: Arc<Mutex<Vec<HttpRequest>>>,
    }

    impl MockTransport {
        pub fn new(responses: Vec<MockResponse>) -> (Self, Arc<Mutex<Vec<HttpRequest>>>) {
            let requests = Arc::new(Mutex::new(Vec::new()));
            let transport = Self {
                responses,
                next_index: Mutex::new(0),
                requests: requests.clone(),
            };
            (transport, requests)
        }

        pub fn requests(&self) -> Vec<HttpRequest> {
            self.requests.lock().expect("lock requests").clone()
        }
    }

    impl HttpTransport for MockTransport {
        fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
            self.requests.lock().expect("lock requests").push(request);

            let mut index = self.next_index.lock().expect("lock index");
            let response = self.responses.get(*index).cloned().unwrap_or(MockResponse {
                status: 500,
                body: r#"{"error":"no mock response"}"#.to_string(),
            });
            *index += 1;

            Ok(HttpResponse {
                status: response.status,
                body: response.body,
            })
        }
    }

    #[test]
    fn mock_transport_returns_responses_in_order() {
        let (transport, _) = MockTransport::new(vec![
            MockResponse::success("first"),
            MockResponse::success("second"),
        ]);

        let send = |body: &str| {
            transport.send(HttpRequest {
                url: "https://example.com".to_string(),
                body: body.to_string(),
                api_key: "sk-test".to_string(),
                timeout: None,
            })
        };

        assert_eq!(send("1").expect("1").body, "first");
        assert_eq!(send("2").expect("2").body, "second");
    }

    #[test]
    fn http_request_debug_redacts_api_key() {
        let request = HttpRequest {
            url: "https://example.com".to_string(),
            body: "{}".to_string(),
            api_key: "sk-secret-debug-key".to_string(),
            timeout: None,
        };
        let debug = format!("{request:?}");
        assert!(!debug.contains("sk-secret-debug-key"));
        assert!(debug.contains(REDACTED));
    }
}
