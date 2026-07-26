use std::fmt;
use std::io::Read;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::redact::REDACTED;

use super::bounded::{bytes_to_utf8_body, check_content_length, collect_bounded_body};
use super::error::{LlmError, Result};

pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_TOTAL_TIMEOUT: Duration = Duration::from_secs(60);
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub body: String,
    pub api_key: String,
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
}

impl HttpTransport for ReqwestTransport {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
        let headers = build_headers(&request.api_key)?;

        let mut response = self
            .client
            .post(&request.url)
            .headers(headers)
            .body(request.body)
            .send()
            .map_err(map_reqwest_error)?;

        let status = response.status().as_u16();
        check_content_length(response.content_length())?;

        let body = read_bounded_response_body(&mut response)?;

        Ok(HttpResponse { status, body })
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
        };
        let debug = format!("{request:?}");
        assert!(!debug.contains("sk-secret-debug-key"));
        assert!(debug.contains(REDACTED));
    }
}
