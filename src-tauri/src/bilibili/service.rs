use super::error::{BilibiliError, Result};
use super::http::{HttpTransport, ReqwestTransport};
use super::player::fetch_subtitle_tracks;
use super::subtitle::{
    fetch_subtitle_segments, select_subtitle_track, validate_subtitle_coverage,
};
use super::types::BilibiliSubtitleResult;
use super::url::{is_short_link_host, parse_video_url, resolve_short_link};
use super::view::fetch_view;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoPreview {
    pub title: String,
    pub platform: &'static str,
    pub video_id: String,
    pub duration_ms: u64,
    pub p: u32,
    pub page_count: u32,
    pub part_title: Option<String>,
    pub has_subtitles: bool,
    pub auth_required: bool,
}

pub fn fetch_subtitles(input_url: &str, cookie: Option<&str>) -> Result<BilibiliSubtitleResult> {
    let transport = ReqwestTransport::new(cookie.map(str::to_string))?;
    fetch_subtitles_with_transport(&transport, input_url)
}

pub fn preview_video(input_url: &str, cookie: Option<&str>) -> Result<VideoPreview> {
    let transport = ReqwestTransport::new(cookie.map(str::to_string))?;
    preview_video_with_transport(&transport, input_url)
}

pub fn fetch_subtitles_with_transport(
    transport: &dyn HttpTransport,
    input_url: &str,
) -> Result<BilibiliSubtitleResult> {
    let resolved_url = resolve_input_url(transport, input_url.trim())?;
    let video_ref = parse_video_url(&resolved_url)?;
    let (view, selected) = fetch_view(transport, &video_ref)?;

    // Bilibili AI subtitle CDN is intermittently wrong/incomplete for the same cid.
    // Retry a few times before surfacing SUBTITLE_CORRUPT / NO_SUBTITLE.
    let mut last_error: Option<BilibiliError> = None;
    for _attempt in 0..3 {
        match fetch_validated_segments(transport, &view.bvid, selected.cid, selected.duration_secs)
        {
            Ok((language, segments)) => {
                return Ok(BilibiliSubtitleResult {
                    title: view.title,
                    part_title: selected.part_title,
                    bvid: view.bvid,
                    aid: view.aid,
                    cid: selected.cid,
                    p: selected.p,
                    page_count: selected.page_count,
                    duration_ms: selected.duration_secs.saturating_mul(1000),
                    language,
                    segments,
                });
            }
            Err(err) => {
                let retryable = matches!(
                    err.code(),
                    "SUBTITLE_CORRUPT" | "NO_SUBTITLE" | "NETWORK_ERROR"
                );
                last_error = Some(err);
                if !retryable {
                    break;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        BilibiliError::no_subtitle("failed to fetch usable subtitles")
    }))
}

/// Lightweight metadata + subtitle-track presence (no subtitle body download).
pub fn preview_video_with_transport(
    transport: &dyn HttpTransport,
    input_url: &str,
) -> Result<VideoPreview> {
    let resolved_url = resolve_input_url(transport, input_url.trim())?;
    let video_ref = parse_video_url(&resolved_url)?;
    let (view, selected) = fetch_view(transport, &video_ref)?;

    let mut has_subtitles = false;
    let mut auth_required = false;
    match fetch_subtitle_tracks(transport, &view.bvid, selected.cid) {
        Ok(tracks) => {
            has_subtitles = !tracks.is_empty();
        }
        Err(err) => match err.code() {
            "AUTH_REQUIRED" => auth_required = true,
            "NO_SUBTITLE" => {}
            _ => return Err(err),
        },
    }

    Ok(VideoPreview {
        title: view.title,
        platform: "bilibili",
        video_id: view.bvid,
        duration_ms: selected.duration_secs.saturating_mul(1000),
        p: selected.p,
        page_count: selected.page_count,
        part_title: selected.part_title,
        has_subtitles,
        auth_required,
    })
}

fn fetch_validated_segments(
    transport: &dyn HttpTransport,
    bvid: &str,
    cid: u64,
    duration_secs: u64,
) -> Result<(String, Vec<crate::bilibili::types::SubtitleSegment>)> {
    let tracks = fetch_subtitle_tracks(transport, bvid, cid)?;
    let track = select_subtitle_track(&tracks)?;
    let segments = fetch_subtitle_segments(transport, &track.subtitle_url)?;
    validate_subtitle_coverage(&segments, duration_secs)?;
    Ok((track.lan.clone(), segments))
}

fn resolve_input_url(transport: &dyn HttpTransport, input_url: &str) -> Result<String> {
    if input_url.trim().is_empty() {
        return Err(BilibiliError::invalid_url("URL is required"));
    }

    let parsed = url::Url::parse(input_url)
        .map_err(|_| BilibiliError::invalid_url("URL is not a valid absolute http(s) URL"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| BilibiliError::invalid_url("URL host is required"))?;

    if is_short_link_host(host) {
        return resolve_short_link(transport, input_url);
    }

    Ok(input_url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::http::mock::{MockResponse, MockTransport};

    #[test]
    fn fetch_subtitles_success_flow() {
        let transport = MockTransport::new(vec![
            (
                "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD&p=2",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "bvid": "BV1xx411c7mD",
                            "aid": 170001,
                            "title": "Main Title",
                            "pages": [
                                {"cid": 1, "page": 1, "part": "Part 1", "duration": 5},
                                {"cid": 2, "page": 2, "part": "Part 2", "duration": 5}
                            ]
                        }
                    }"#,
                ),
            ),
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=2",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "subtitle": {
                                "subtitles": [
                                    {
                                        "lan": "zh-CN",
                                        "lan_doc": "中文",
                                        "ai_type": 0,
                                        "subtitle_url": "//subtitle.bilibili.com/demo.json"
                                    }
                                ]
                            }
                        }
                    }"#,
                ),
            ),
            (
                "https://subtitle.bilibili.com/demo.json",
                MockResponse::success(
                    r#"{
                        "body": [
                            {"from": 0.0, "to": 1.2, "content": "你好"},
                            {"from": 1.2, "to": 2.0, "content": "世界"},
                            {"from": 2.0, "to": 3.0, "content": "测试"}
                        ]
                    }"#,
                ),
            ),
        ]);

        let result = fetch_subtitles_with_transport(
            &transport,
            "https://www.bilibili.com/video/BV1xx411c7mD?p=2",
        )
        .unwrap();

        assert_eq!(result.title, "Main Title");
        assert_eq!(result.part_title.as_deref(), Some("Part 2"));
        assert_eq!(result.language, "zh-CN");
        assert_eq!(result.segments.len(), 3);
        assert_eq!(result.segments[0].text, "你好");
        assert_eq!(result.duration_ms, 5000);

        let requests = transport.requests();
        let subtitle_request = requests
            .iter()
            .find(|request| request.url.contains("subtitle.bilibili.com"))
            .expect("subtitle request");
        assert!(!subtitle_request.send_cookie);
        assert_eq!(
            subtitle_request.body_limit,
            crate::bilibili::http::ResponseBodyLimit::Subtitle
        );
    }

    #[test]
    fn fetch_subtitles_reports_no_subtitle() {
        let transport = MockTransport::new(vec![
            (
                "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "bvid": "BV1xx411c7mD",
                            "aid": 170001,
                            "title": "Main Title",
                            "cid": 1,
                            "pages": []
                        }
                    }"#,
                ),
            ),
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=1",
                MockResponse::success(r#"{"code":0,"data":{"subtitle":{"subtitles":[]}}}"#),
            ),
        ]);

        let err = fetch_subtitles_with_transport(
            &transport,
            "https://www.bilibili.com/video/BV1xx411c7mD",
        )
        .unwrap_err();
        assert_eq!(err.code(), "NO_SUBTITLE");
    }

    #[test]
    fn fetch_subtitles_reports_rate_limit() {
        let transport = MockTransport::new(vec![(
            "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD",
            MockResponse::success("").with_status(429),
        )]);

        let err = fetch_subtitles_with_transport(
            &transport,
            "https://www.bilibili.com/video/BV1xx411c7mD",
        )
        .unwrap_err();
        assert_eq!(err.code(), "RATE_LIMITED");
    }

    #[test]
    fn short_link_flow_resolves_before_view() {
        let transport = MockTransport::new(vec![
            (
                "https://b23.tv/abc",
                MockResponse::redirect("https://www.bilibili.com/video/BV1xx411c7mD"),
            ),
            (
                "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "bvid": "BV1xx411c7mD",
                            "aid": 170001,
                            "title": "Main Title",
                            "cid": 1,
                            "pages": []
                        }
                    }"#,
                ),
            ),
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=1",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "subtitle": {
                                "subtitles": [
                                    {
                                        "lan": "zh-CN",
                                        "subtitle_url": "https://subtitle.bilibili.com/demo.json"
                                    }
                                ]
                            }
                        }
                    }"#,
                ),
            ),
            (
                "https://subtitle.bilibili.com/demo.json",
                MockResponse::success(r#"{"body":[{"from":0.0,"to":1.0,"content":"hi"},{"from":1.0,"to":2.0,"content":"there"},{"from":2.0,"to":3.0,"content":"friend"}]}"#),
            ),
        ]);

        let result = fetch_subtitles_with_transport(&transport, "https://b23.tv/abc").unwrap();
        assert_eq!(result.bvid, "BV1xx411c7mD");
        assert!(!transport.requests()[0].send_cookie);
    }

    #[test]
    fn preview_video_reports_metadata_without_subtitle_download() {
        let transport = MockTransport::new(vec![
            (
                "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD&p=2",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "bvid": "BV1xx411c7mD",
                            "aid": 170001,
                            "title": "Main Title",
                            "pages": [
                                {"cid": 1, "page": 1, "part": "Part 1", "duration": 5},
                                {"cid": 2, "page": 2, "part": "Part 2", "duration": 90}
                            ]
                        }
                    }"#,
                ),
            ),
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=2",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "subtitle": {
                                "subtitles": [
                                    {
                                        "lan": "zh-CN",
                                        "subtitle_url": "//subtitle.bilibili.com/demo.json"
                                    }
                                ]
                            }
                        }
                    }"#,
                ),
            ),
        ]);

        let preview = preview_video_with_transport(
            &transport,
            "https://www.bilibili.com/video/BV1xx411c7mD?p=2",
        )
        .unwrap();

        assert_eq!(preview.title, "Main Title");
        assert_eq!(preview.p, 2);
        assert_eq!(preview.page_count, 2);
        assert_eq!(preview.duration_ms, 90_000);
        assert!(preview.has_subtitles);
        assert!(!preview.auth_required);
        assert!(!transport
            .requests()
            .iter()
            .any(|request| request.url.contains("subtitle.bilibili.com")));
    }
}
