use super::error::{BilibiliError, Result};
use super::http::{HttpTransport, ReqwestTransport};
use super::player::fetch_subtitle_tracks;
use super::subtitle::{fetch_subtitle_segments, select_subtitle_track};
use super::types::BilibiliSubtitleResult;
use super::url::{is_short_link_host, parse_video_url, resolve_short_link};
use super::view::fetch_view;

pub fn fetch_subtitles(input_url: &str, cookie: Option<&str>) -> Result<BilibiliSubtitleResult> {
    let transport = ReqwestTransport::new(cookie.map(str::to_string))?;
    fetch_subtitles_with_transport(&transport, input_url)
}

pub fn fetch_subtitles_with_transport(
    transport: &dyn HttpTransport,
    input_url: &str,
) -> Result<BilibiliSubtitleResult> {
    let resolved_url = resolve_input_url(transport, input_url.trim())?;
    let video_ref = parse_video_url(&resolved_url)?;
    let (view, selected) = fetch_view(transport, &video_ref)?;
    let tracks = fetch_subtitle_tracks(transport, &view.bvid, selected.cid)?;
    let track = select_subtitle_track(&tracks)?;
    let segments = fetch_subtitle_segments(transport, &track.subtitle_url)?;

    Ok(BilibiliSubtitleResult {
        title: view.title,
        part_title: selected.part_title,
        bvid: view.bvid,
        aid: view.aid,
        cid: selected.cid,
        p: selected.p,
        page_count: selected.page_count,
        language: track.lan.clone(),
        segments,
    })
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
                                {"cid": 1, "page": 1, "part": "Part 1", "duration": 100},
                                {"cid": 2, "page": 2, "part": "Part 2", "duration": 200}
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
                            {"from": 1.2, "to": 2.0, "content": "世界"}
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
        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[0].text, "你好");

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
                MockResponse::success(r#"{"body":[{"from":0.0,"to":1.0,"content":"hi"}]}"#),
            ),
        ]);

        let result = fetch_subtitles_with_transport(&transport, "https://b23.tv/abc").unwrap();
        assert_eq!(result.bvid, "BV1xx411c7mD");
        assert!(!transport.requests()[0].send_cookie);
    }
}
