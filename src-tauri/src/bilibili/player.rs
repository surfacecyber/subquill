use super::api::{ensure_api_success, ensure_http_success};
use super::error::{BilibiliError, Result};
use super::http::{HttpRequest, HttpTransport};
use super::types::{PlayerApiData, PlayerApiResponse, SubtitleTrack};

/// Match BiliNote: `/x/player/wbi/v2` with `bvid`+`cid`.
/// Using `/x/player/v2` with `aid` can return empty or *wrong-video* `subtitle_url` values.
const PLAYER_API: &str = "https://api.bilibili.com/x/player/wbi/v2";

pub fn fetch_subtitle_tracks(
    transport: &dyn HttpTransport,
    bvid: &str,
    cid: u64,
) -> Result<Vec<SubtitleTrack>> {
    if bvid.trim().is_empty() {
        return Err(BilibiliError::api_rejected(
            "bvid is required for player API",
        ));
    }
    if cid == 0 {
        return Err(BilibiliError::api_rejected(
            "cid must be non-zero for player API",
        ));
    }

    // Empty subtitle lists can appear on a cold call; retry once like before.
    let url = format!("{PLAYER_API}?bvid={bvid}&cid={cid}");
    let mut need_login_subtitle = false;
    for attempt in 0..2 {
        let response = transport.send(HttpRequest::api(url.clone()).with_cookie())?;
        ensure_http_success(response.status, "player API")?;

        let payload: PlayerApiResponse = serde_json::from_str(&response.body).map_err(|err| {
            BilibiliError::network(format!("failed to parse player API response: {err}"))
        })?;

        ensure_api_success(payload.code, &payload.message, "player API")?;

        let data = payload.data.unwrap_or(PlayerApiData {
            subtitle: Default::default(),
            need_login_subtitle: false,
        });
        need_login_subtitle = data.need_login_subtitle;
        let subtitles = data.subtitle.subtitles;

        // A track with empty subtitle_url is unusable (seen on non-wbi player/v2).
        let usable: Vec<_> = subtitles
            .into_iter()
            .filter(|track| !track.subtitle_url.trim().is_empty())
            .collect();

        if !usable.is_empty() {
            return Ok(usable);
        }

        if attempt == 0 {
            continue;
        }
    }

    // Logged-out (or expired cookie) responses often look like "no subtitles"
    // while the site still shows AI captions — surface AUTH_REQUIRED instead.
    if need_login_subtitle {
        return Err(BilibiliError::auth_required(
            "player API hid subtitle tracks; a valid Bilibili SESSDATA cookie is required",
        ));
    }

    Err(BilibiliError::no_subtitle(
        "player API returned no downloadable subtitle tracks",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::http::mock::{MockResponse, MockTransport};

    #[test]
    fn fetch_subtitle_tracks_returns_no_subtitle_error() {
        let transport = MockTransport::new(vec![
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=2",
                MockResponse::success(r#"{"code":0,"data":{"subtitle":{"subtitles":[]}}}"#),
            ),
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=2",
                MockResponse::success(r#"{"code":0,"data":{"subtitle":{"subtitles":[]}}}"#),
            ),
        ]);

        let err = fetch_subtitle_tracks(&transport, "BV1xx411c7mD", 2).unwrap_err();
        assert_eq!(err.code(), "NO_SUBTITLE");
    }

    #[test]
    fn fetch_subtitle_tracks_maps_need_login_to_auth_required() {
        let transport = MockTransport::new(vec![
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=2",
                MockResponse::success(
                    r#"{"code":0,"data":{"need_login_subtitle":true,"subtitle":{"subtitles":[]}}}"#,
                ),
            ),
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=2",
                MockResponse::success(
                    r#"{"code":0,"data":{"need_login_subtitle":true,"subtitle":{"subtitles":[]}}}"#,
                ),
            ),
        ]);

        let err = fetch_subtitle_tracks(&transport, "BV1xx411c7mD", 2).unwrap_err();
        assert_eq!(err.code(), "AUTH_REQUIRED");
    }

    #[test]
    fn fetch_subtitle_tracks_ignores_empty_subtitle_url() {
        let transport = MockTransport::new(vec![
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=2",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "subtitle": {
                                "subtitles": [
                                    {
                                        "lan": "ai-zh",
                                        "ai_type": 0,
                                        "subtitle_url": ""
                                    }
                                ]
                            }
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
                                        "lan": "ai-zh",
                                        "ai_type": 0,
                                        "subtitle_url": "//aisubtitle.hdslb.com/demo.json"
                                    }
                                ]
                            }
                        }
                    }"#,
                ),
            ),
        ]);

        let tracks = fetch_subtitle_tracks(&transport, "BV1xx411c7mD", 2).unwrap();
        assert_eq!(tracks.len(), 1);
        assert!(!tracks[0].subtitle_url.is_empty());
        assert_eq!(transport.requests().len(), 2);
    }

    #[test]
    fn fetch_subtitle_tracks_maps_rate_limit_code() {
        let transport = MockTransport::new(vec![(
            "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=2",
            MockResponse::success(r#"{"code":-412,"message":"request blocked"}"#),
        )]);

        let err = fetch_subtitle_tracks(&transport, "BV1xx411c7mD", 2).unwrap_err();
        assert_eq!(err.code(), "RATE_LIMITED");
    }

    #[test]
    fn fetch_subtitle_tracks_rejects_zero_cid_without_request() {
        let transport = MockTransport::new(vec![]);
        let err = fetch_subtitle_tracks(&transport, "BV1xx411c7mD", 0).unwrap_err();
        assert_eq!(err.code(), "API_REJECTED");
        assert!(transport.requests().is_empty());
    }
}
