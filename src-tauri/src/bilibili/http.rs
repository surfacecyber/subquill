use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, REFERER, USER_AGENT};

use super::bounded::{check_content_length, read_bounded_body};
use super::error::{BilibiliError, Result};
use super::url::is_api_host;

pub const USER_AGENT_VALUE: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
pub const BILIBILI_REFERER: &str = "https://www.bilibili.com";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum response body sizes per request kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseBodyLimit {
    /// Short-link probe: headers matter; body should be tiny.
    Redirect,
    /// View/player API JSON payloads.
    Api,
    /// Subtitle CDN JSON payloads.
    Subtitle,
}

impl ResponseBodyLimit {
    pub const REDIRECT_MAX: u64 = 64 * 1024;
    pub const API_MAX: u64 = 2 * 1024 * 1024;
    pub const SUBTITLE_MAX: u64 = 16 * 1024 * 1024;

    pub fn max_bytes(self) -> u64 {
        match self {
            Self::Redirect => Self::REDIRECT_MAX,
            Self::Api => Self::API_MAX,
            Self::Subtitle => Self::SUBTITLE_MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub send_cookie: bool,
    pub body_limit: ResponseBodyLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub location: Option<String>,
}

impl HttpRequest {
    pub fn api(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            send_cookie: false,
            body_limit: ResponseBodyLimit::Api,
        }
    }

    pub fn subtitle(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            send_cookie: false,
            body_limit: ResponseBodyLimit::Subtitle,
        }
    }

    pub fn redirect_probe(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            send_cookie: false,
            body_limit: ResponseBodyLimit::Redirect,
        }
    }

    pub fn with_cookie(mut self) -> Self {
        self.send_cookie = true;
        self
    }

    pub fn without_cookie(mut self) -> Self {
        self.send_cookie = false;
        self
    }
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn is_redirect(&self) -> bool {
        matches!(self.status, 301 | 302 | 303 | 307 | 308)
    }
}

pub trait HttpTransport: Send + Sync {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse>;
}

/// Build headers for `api.bilibili.com` requests. Cookie is attached only when
/// `attach_cookie` is true **and** a non-empty cookie value is configured.
pub fn build_api_headers(cookie: Option<&str>, attach_cookie: bool) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    headers.insert(REFERER, HeaderValue::from_static(BILIBILI_REFERER));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/plain, */*"),
    );

    if attach_cookie {
        if let Some(cookie) = cookie.filter(|value| !value.trim().is_empty()) {
            headers.insert(
                HeaderName::from_static("cookie"),
                HeaderValue::from_str(cookie)
                    .map_err(|_| BilibiliError::network("invalid cookie header value"))?,
            );
        }
    }

    Ok(headers)
}

pub struct ReqwestTransport {
    client: Client,
    cookie: Option<String>,
}

impl ReqwestTransport {
    pub fn new(cookie: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .map_err(|err| BilibiliError::network(err.to_string()))?;

        Ok(Self {
            client,
            cookie: cookie.filter(|value| !value.trim().is_empty()),
        })
    }
}

fn read_bounded_response_body(
    response: &mut reqwest::blocking::Response,
    limit: ResponseBodyLimit,
) -> Result<String> {
    check_content_length(response.content_length(), limit)?;
    read_bounded_body(response, limit)
}

impl HttpTransport for ReqwestTransport {
    fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
        let parsed = url::Url::parse(&request.url)
            .map_err(|_| BilibiliError::network("request URL is invalid"))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| BilibiliError::network("request URL host is required"))?;

        let headers = if is_api_host(host) {
            build_api_headers(self.cookie.as_deref(), request.send_cookie)?
        } else {
            let mut headers = HeaderMap::new();
            headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
            headers
        };

        let mut response = self
            .client
            .get(request.url)
            .headers(headers)
            .send()
            .map_err(|err| BilibiliError::network(err.to_string()))?;

        let status = response.status().as_u16();

        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);

        let body = read_bounded_response_body(&mut response, request.body_limit)?;

        Ok(HttpResponse {
            status,
            body,
            location,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_headers_without_cookie_include_referer_and_accept() {
        let headers = build_api_headers(None, true).expect("headers");
        assert!(headers.contains_key(USER_AGENT));
        assert!(headers.contains_key(REFERER));
        assert!(headers.contains_key(ACCEPT));
        assert!(!headers.contains_key(HeaderName::from_static("cookie")));
    }

    #[test]
    fn api_headers_attach_cookie_only_when_configured() {
        let without = build_api_headers(None, true).expect("headers");
        assert!(!without.contains_key(HeaderName::from_static("cookie")));

        let with = build_api_headers(Some("SESSDATA=abc"), true).expect("headers");
        assert!(with.contains_key(HeaderName::from_static("cookie")));
    }

    #[test]
    fn api_headers_skip_cookie_when_attach_disabled() {
        let headers = build_api_headers(Some("SESSDATA=abc"), false).expect("headers");
        assert!(!headers.contains_key(HeaderName::from_static("cookie")));
    }

    #[test]
    fn request_constructors_set_body_limits() {
        assert_eq!(
            HttpRequest::api("https://api.bilibili.com/x").body_limit,
            ResponseBodyLimit::Api
        );
        assert_eq!(
            HttpRequest::subtitle("https://subtitle.bilibili.com/x.json").body_limit,
            ResponseBodyLimit::Subtitle
        );
        assert_eq!(
            HttpRequest::redirect_probe("https://b23.tv/x").body_limit,
            ResponseBodyLimit::Redirect
        );
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use crate::bilibili::bounded::{body_limit_exceeded, check_content_length};
    use reqwest::StatusCode;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone)]
    pub struct MockResponse {
        pub status: u16,
        pub body: String,
        pub location: Option<String>,
        pub content_length: Option<u64>,
    }

    impl MockResponse {
        pub fn success(body: impl Into<String>) -> Self {
            Self {
                status: StatusCode::OK.as_u16(),
                body: body.into(),
                location: None,
                content_length: None,
            }
        }

        pub fn redirect(location: impl Into<String>) -> Self {
            Self {
                status: StatusCode::FOUND.as_u16(),
                body: String::new(),
                location: Some(location.into()),
                content_length: None,
            }
        }

        pub fn with_status(mut self, status: u16) -> Self {
            self.status = status;
            self
        }

        pub fn with_content_length(mut self, len: u64) -> Self {
            self.content_length = Some(len);
            self
        }
    }

    #[derive(Debug)]
    pub struct MockTransport {
        responses: HashMap<String, MockResponse>,
        requests: Arc<Mutex<Vec<HttpRequest>>>,
    }

    impl MockTransport {
        pub fn new(entries: Vec<(&str, MockResponse)>) -> Self {
            Self {
                responses: entries
                    .into_iter()
                    .map(|(url, response)| (url.to_string(), response))
                    .collect(),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn requests(&self) -> Vec<HttpRequest> {
            self.requests.lock().expect("lock requests").clone()
        }
    }

    impl HttpTransport for MockTransport {
        fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
            self.requests
                .lock()
                .expect("lock requests")
                .push(request.clone());

            let response = self.responses.get(&request.url).cloned().ok_or_else(|| {
                BilibiliError::network(format!("no mock response for {}", request.url))
            })?;

            let declared = response.content_length.or(Some(response.body.len() as u64));
            check_content_length(declared, request.body_limit)?;

            if response.body.len() as u64 > request.body_limit.max_bytes() {
                return Err(body_limit_exceeded(request.body_limit));
            }

            Ok(HttpResponse {
                status: response.status,
                body: response.body,
                location: response.location,
            })
        }
    }

    #[cfg(test)]
    mod limit_tests {
        use super::*;

        #[test]
        fn mock_accepts_body_at_api_limit() {
            let body = "a".repeat(ResponseBodyLimit::Api.max_bytes() as usize);
            let transport = MockTransport::new(vec![(
                "https://api.bilibili.com/x",
                MockResponse::success(body),
            )]);

            transport
                .send(HttpRequest::api("https://api.bilibili.com/x"))
                .expect("at limit");
        }

        #[test]
        fn mock_rejects_body_over_api_limit() {
            let body = "a".repeat(ResponseBodyLimit::Api.max_bytes() as usize + 1);
            let transport = MockTransport::new(vec![(
                "https://api.bilibili.com/x",
                MockResponse::success(body),
            )]);

            let err = transport
                .send(HttpRequest::api("https://api.bilibili.com/x"))
                .unwrap_err();
            assert_eq!(err.code(), "API_REJECTED");
        }

        #[test]
        fn mock_subtitle_allows_larger_body_than_api() {
            let size = ResponseBodyLimit::Api.max_bytes() as usize + 1;
            let body = "b".repeat(size);
            let transport = MockTransport::new(vec![(
                "https://subtitle.bilibili.com/x.json",
                MockResponse::success(body.clone()),
            )]);

            let api_err = MockTransport::new(vec![(
                "https://api.bilibili.com/x",
                MockResponse::success(body),
            )])
            .send(HttpRequest::api("https://api.bilibili.com/x"))
            .unwrap_err();
            assert_eq!(api_err.code(), "API_REJECTED");

            transport
                .send(HttpRequest::subtitle(
                    "https://subtitle.bilibili.com/x.json",
                ))
                .expect("subtitle limit");
        }

        #[test]
        fn mock_rejects_declared_content_length_over_limit() {
            let transport = MockTransport::new(vec![(
                "https://api.bilibili.com/x",
                MockResponse::success("small")
                    .with_content_length(ResponseBodyLimit::Api.max_bytes() + 1),
            )]);

            let err = transport
                .send(HttpRequest::api("https://api.bilibili.com/x"))
                .unwrap_err();
            assert_eq!(err.code(), "API_REJECTED");
        }
    }
}
