use url::Url;

use super::error::{BilibiliError, Result};
use super::types::ParsedVideoRef;
use crate::bilibili::http::{HttpRequest, HttpTransport};

pub const VIDEO_PAGE_HOSTS: &[&str] = &["www.bilibili.com", "m.bilibili.com"];
pub const SHORT_LINK_HOSTS: &[&str] = &["b23.tv"];
pub const API_HOST: &str = "api.bilibili.com";
pub const SUBTITLE_CDN_SUFFIXES: &[&str] = &["hdslb.com", "bilibili.com", "bilivideo.com"];
pub const MAX_SHORT_LINK_REDIRECTS: u32 = 5;

pub fn parse_video_url(input: &str) -> Result<ParsedVideoRef> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(BilibiliError::invalid_url("URL is required"));
    }

    let parsed = parse_http_url(trimmed)?;
    let host = host_str(&parsed)?;

    if is_short_link_host(host) {
        return Err(BilibiliError::invalid_url(
            "short links must be resolved before parsing video id",
        ));
    }

    if !is_video_page_host(host) {
        return Err(BilibiliError::invalid_url("unsupported bilibili host"));
    }

    extract_video_ref(&parsed)
}

pub fn is_short_link_host(host: &str) -> bool {
    host_is_exact(host, SHORT_LINK_HOSTS)
}

pub fn is_video_page_host(host: &str) -> bool {
    host_is_exact(host, VIDEO_PAGE_HOSTS)
}

pub fn is_api_host(host: &str) -> bool {
    host.eq_ignore_ascii_case(API_HOST)
}

pub fn is_subtitle_cdn_host(host: &str) -> bool {
    host_matches_suffix(host, SUBTITLE_CDN_SUFFIXES)
}

pub fn resolve_short_link(transport: &dyn HttpTransport, input: &str) -> Result<String> {
    let trimmed = input.trim();
    let parsed = parse_http_url(trimmed)?;
    let host = host_str(&parsed)?;

    if !is_short_link_host(host) {
        return Ok(trimmed.to_string());
    }

    let mut current = trimmed.to_string();
    for _hop in 0..MAX_SHORT_LINK_REDIRECTS {
        let response = transport.send(HttpRequest::redirect_probe(&current).without_cookie())?;

        if response.is_redirect() {
            let Some(location) = response.location else {
                return Err(BilibiliError::short_link_failed(
                    "short link redirect missing Location header",
                ));
            };

            let next_url = resolve_redirect_location(&current, &location)?;
            let next = parse_http_url(&next_url)?;
            let next_host = host_str(&next)?;
            if !is_short_link_host(next_host) && !is_video_page_host(next_host) {
                return Err(BilibiliError::short_link_failed(
                    "short link redirected to an unsupported host",
                ));
            }

            current = next_url;

            if is_video_page_host(next_host) {
                return Ok(current);
            }

            continue;
        }

        if response.is_success() {
            return Ok(current);
        }

        return Err(BilibiliError::short_link_failed(format!(
            "short link resolution failed with HTTP {}",
            response.status
        )));
    }

    Err(BilibiliError::short_link_failed(format!(
        "short link exceeded {MAX_SHORT_LINK_REDIRECTS} redirects"
    )))
}

pub fn normalize_subtitle_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(BilibiliError::subtitle_corrupt("subtitle URL is empty"));
    }

    let normalized = if let Some(rest) = trimmed.strip_prefix("//") {
        format!("https://{rest}")
    } else {
        trimmed.to_string()
    };

    let parsed = parse_https_url(&normalized)?;
    let host = host_str(&parsed)?;
    if !is_subtitle_cdn_host(host) {
        return Err(BilibiliError::subtitle_corrupt(
            "subtitle URL host is not allowed",
        ));
    }

    Ok(parsed.to_string())
}

fn resolve_redirect_location(current: &str, location: &str) -> Result<String> {
    let base = Url::parse(current)
        .map_err(|_| BilibiliError::short_link_failed("short link base URL is invalid"))?;
    let joined = base
        .join(location.trim())
        .map_err(|_| BilibiliError::short_link_failed("short link redirect Location is invalid"))?;
    Ok(joined.to_string())
}

fn parse_http_url(input: &str) -> Result<Url> {
    let parsed = Url::parse(input)
        .map_err(|_| BilibiliError::invalid_url("URL is not a valid absolute http(s) URL"))?;

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(BilibiliError::invalid_url(
            "only http and https are allowed",
        ));
    }

    if parsed.username() != "" || parsed.password().is_some() {
        return Err(BilibiliError::invalid_url(
            "URLs with embedded credentials are not allowed",
        ));
    }

    if parsed.fragment().is_some() {
        return Err(BilibiliError::invalid_url(
            "URLs with fragments are not allowed",
        ));
    }

    let host = host_str(&parsed)?;
    if host.is_empty() {
        return Err(BilibiliError::invalid_url("URL host is required"));
    }

    Ok(parsed)
}

fn parse_https_url(input: &str) -> Result<Url> {
    let parsed = parse_http_url(input)?;
    if parsed.scheme() != "https" {
        return Err(BilibiliError::subtitle_corrupt(
            "subtitle URL must use https",
        ));
    }
    Ok(parsed)
}

fn host_str(url: &Url) -> Result<&str> {
    url.host_str()
        .ok_or_else(|| BilibiliError::invalid_url("URL host is required"))
}

fn host_is_exact(host: &str, allowed: &[&str]) -> bool {
    let host = host.to_ascii_lowercase();
    allowed
        .iter()
        .any(|candidate| host == candidate.to_ascii_lowercase())
}

fn host_matches_suffix(host: &str, suffixes: &[&str]) -> bool {
    let host = host.to_ascii_lowercase();
    suffixes.iter().any(|suffix| {
        let suffix = suffix.to_ascii_lowercase();
        host == suffix || host.ends_with(&format!(".{suffix}"))
    })
}

fn extract_video_ref(url: &Url) -> Result<ParsedVideoRef> {
    let path = url.path();
    let (bvid, aid) = extract_ids_from_path(path)?;

    let mut p_explicit = false;
    let mut p = None;

    for (key, value) in url.query_pairs() {
        if key == "p" {
            p_explicit = true;
            p = Some(parse_p_value(&value)?);
        }
    }

    if !p_explicit {
        if let Some(path_p) = extract_p_from_path_suffix(path) {
            p_explicit = true;
            p = Some(path_p);
        }
    }

    if bvid.is_none() && aid.is_none() {
        return Err(BilibiliError::invalid_url(
            "could not extract BV or av id from URL",
        ));
    }

    Ok(ParsedVideoRef {
        bvid,
        aid,
        p,
        p_explicit,
    })
}

fn extract_ids_from_path(path: &str) -> Result<(Option<String>, Option<u64>)> {
    let mut bvid = None;
    let mut aid = None;

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    for segment in segments {
        if let Some(rest) = segment.strip_prefix("BV") {
            if rest.chars().all(is_bvid_char) && !rest.is_empty() {
                bvid = Some(format!("BV{rest}"));
            }
        } else if let Some(rest) = segment.strip_prefix("av") {
            if let Ok(parsed_aid) = rest.parse::<u64>() {
                aid = Some(parsed_aid);
            }
        }
    }

    Ok((bvid, aid))
}

fn extract_p_from_path_suffix(path: &str) -> Option<u32> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    for segment in segments {
        if let Some(rest) = segment.strip_prefix('p') {
            if let Ok(value) = rest.parse::<u32>() {
                if value >= 1 {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn parse_p_value(raw: &str) -> Result<u32> {
    let value = raw
        .parse::<u32>()
        .map_err(|_| BilibiliError::invalid_url("p must be a positive integer"))?;
    if value < 1 {
        return Err(BilibiliError::invalid_url("p must be >= 1"));
    }
    Ok(value)
}

fn is_bvid_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::http::mock::{MockResponse, MockTransport};

    #[test]
    fn parse_standard_bv_url_with_p_query() {
        let parsed = parse_video_url("https://www.bilibili.com/video/BV1xx411c7mD?p=3").unwrap();
        assert_eq!(parsed.bvid.as_deref(), Some("BV1xx411c7mD"));
        assert_eq!(parsed.p, Some(3));
        assert!(parsed.p_explicit);
    }

    #[test]
    fn parse_mobile_av_url() {
        let parsed = parse_video_url("https://m.bilibili.com/video/av170001").unwrap();
        assert_eq!(parsed.aid, Some(170001));
        assert!(!parsed.p_explicit);
    }

    #[test]
    fn parse_p_path_suffix() {
        let parsed = parse_video_url("https://www.bilibili.com/video/BV1xx411c7mD/p5").unwrap();
        assert_eq!(parsed.p, Some(5));
        assert!(parsed.p_explicit);
    }

    #[test]
    fn reject_evil_host_suffix() {
        let err = parse_video_url("https://bilibili.com.evil/video/BV1xx411c7mD").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn reject_userinfo_in_url() {
        let err =
            parse_video_url("https://user:pass@www.bilibili.com/video/BV1xx411c7mD").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn reject_any_fragment_in_url() {
        let err =
            parse_video_url("https://www.bilibili.com/video/BV1xx411c7mD#comment").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn reject_fragment_only_bvid() {
        let err = parse_video_url("https://www.bilibili.com/video/#BV1xx411c7mD").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn reject_non_http_scheme() {
        let err = parse_video_url("javascript://www.bilibili.com/video/BV1xx411c7mD").unwrap_err();
        assert_eq!(err.code(), "VALIDATION_ERROR");
    }

    #[test]
    fn subtitle_cdn_host_blocks_suffix_bypass() {
        assert!(is_subtitle_cdn_host("i0.hdslb.com"));
        assert!(is_subtitle_cdn_host("example.bilibili.com"));
        assert!(!is_subtitle_cdn_host("evil-hdslb.com"));
        assert!(!is_subtitle_cdn_host("hdslb.com.evil"));
    }

    #[test]
    fn normalize_protocol_relative_subtitle_url() {
        let normalized = normalize_subtitle_url("//example.bilibili.com/subtitle.json").unwrap();
        assert_eq!(normalized, "https://example.bilibili.com/subtitle.json");
    }

    #[test]
    fn normalize_subtitle_url_rejects_http() {
        let err = normalize_subtitle_url("http://example.bilibili.com/subtitle.json").unwrap_err();
        assert_eq!(err.code(), "SUBTITLE_CORRUPT");
    }

    #[test]
    fn resolve_short_link_returns_video_page_without_followup_get() {
        let transport = MockTransport::new(vec![(
            "https://b23.tv/abc",
            MockResponse::redirect("https://www.bilibili.com/video/BV1xx411c7mD?p=2"),
        )]);

        let resolved = resolve_short_link(&transport, "https://b23.tv/abc").unwrap();
        assert_eq!(resolved, "https://www.bilibili.com/video/BV1xx411c7mD?p=2");
        assert_eq!(transport.requests().len(), 1);
        assert!(!transport.requests()[0].send_cookie);
    }

    #[test]
    fn resolve_short_link_joins_relative_location() {
        let transport = MockTransport::new(vec![
            ("https://b23.tv/a", MockResponse::redirect("/b")),
            (
                "https://b23.tv/b",
                MockResponse::redirect("https://www.bilibili.com/video/BV1xx411c7mD"),
            ),
        ]);

        let resolved = resolve_short_link(&transport, "https://b23.tv/a").unwrap();
        assert_eq!(resolved, "https://www.bilibili.com/video/BV1xx411c7mD");
        assert_eq!(transport.requests().len(), 2);
    }

    #[test]
    fn resolve_short_link_follows_chained_b23_redirects() {
        let transport = MockTransport::new(vec![
            (
                "https://b23.tv/a",
                MockResponse::redirect("https://b23.tv/b"),
            ),
            (
                "https://b23.tv/b",
                MockResponse::redirect("https://www.bilibili.com/video/BV1xx411c7mD"),
            ),
        ]);

        let resolved = resolve_short_link(&transport, "https://b23.tv/a").unwrap();
        assert_eq!(resolved, "https://www.bilibili.com/video/BV1xx411c7mD");
        assert_eq!(transport.requests().len(), 2);
    }

    #[test]
    fn resolve_short_link_rejects_unsupported_redirect_host() {
        let transport = MockTransport::new(vec![(
            "https://b23.tv/abc",
            MockResponse::redirect("https://evil.example/video"),
        )]);

        let err = resolve_short_link(&transport, "https://b23.tv/abc").unwrap_err();
        assert_eq!(err.code(), "SHORT_LINK_FAILED");
    }
}
