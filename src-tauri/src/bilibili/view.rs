use url::Url;

use super::api::{ensure_api_success, ensure_http_success};
use super::error::{BilibiliError, Result};
use super::http::{HttpRequest, HttpTransport};
use super::types::{SelectedPage, VideoPage, ViewApiData, ViewApiResponse, ViewData};
use super::url::API_HOST;
use crate::bilibili::types::ParsedVideoRef;

const VIEW_API: &str = "https://api.bilibili.com/x/web-interface/view";

pub fn fetch_view(
    transport: &dyn HttpTransport,
    video_ref: &ParsedVideoRef,
) -> Result<(ViewData, SelectedPage)> {
    let url = build_view_url(video_ref)?;
    let response = transport.send(HttpRequest::api(url).with_cookie())?;

    if response.status == 404 {
        return Err(BilibiliError::video_not_found("video not found"));
    }

    ensure_http_success(response.status, "view API")?;

    let payload: ViewApiResponse = serde_json::from_str(&response.body).map_err(|err| {
        BilibiliError::network(format!("failed to parse view API response: {err}"))
    })?;

    if payload.code == -404 {
        return Err(BilibiliError::video_not_found("video not found"));
    }

    ensure_api_success(payload.code, &payload.message, "view API")?;

    let data = payload
        .data
        .ok_or_else(|| BilibiliError::video_not_found("view API returned empty data"))?;

    validate_view_data(&data)?;

    let pages = normalize_pages(&data)?;
    let page_count = pages.len().max(1) as u32;
    let requested_p = video_ref.p.unwrap_or(1);

    if video_ref.p_explicit && requested_p > page_count {
        return Err(BilibiliError::part_out_of_range(format!(
            "requested part p={requested_p} but video only has {page_count} parts"
        )));
    }

    let selected = select_page(&pages, requested_p, page_count)?;

    let view = ViewData {
        bvid: data.bvid,
        aid: data.aid,
        title: data.title,
        pages,
    };

    Ok((view, selected))
}

fn build_view_url(video_ref: &ParsedVideoRef) -> Result<String> {
    let mut url = Url::parse(VIEW_API).expect("view API URL is valid");
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(bvid) = &video_ref.bvid {
            pairs.append_pair("bvid", bvid);
        } else if let Some(aid) = video_ref.aid {
            pairs.append_pair("aid", &aid.to_string());
        }
        if let Some(p) = video_ref.p {
            pairs.append_pair("p", &p.to_string());
        }
    }

    let parsed = url::Url::parse(url.as_ref()).expect("built view URL is valid");
    if parsed.host_str() != Some(API_HOST) {
        return Err(BilibiliError::network("view API host mismatch"));
    }

    Ok(url.to_string())
}

fn validate_view_data(data: &ViewApiData) -> Result<()> {
    if data.bvid.trim().is_empty() {
        return Err(BilibiliError::api_rejected("view API returned empty bvid"));
    }
    if data.title.trim().is_empty() {
        return Err(BilibiliError::api_rejected("view API returned empty title"));
    }
    Ok(())
}

fn normalize_pages(data: &ViewApiData) -> Result<Vec<VideoPage>> {
    if !data.pages.is_empty() {
        let pages: Vec<VideoPage> = data
            .pages
            .iter()
            .map(|page| VideoPage {
                cid: page.cid,
                page: page.page,
                part: page.part.clone(),
                duration: page.duration,
            })
            .collect();

        for page in &pages {
            ensure_valid_cid(page.cid, "view API page")?;
        }

        return Ok(pages);
    }

    ensure_valid_cid(data.cid, "view API")?;

    Ok(vec![VideoPage {
        cid: data.cid,
        page: 1,
        part: data.title.clone(),
        duration: data.duration,
    }])
}

fn ensure_valid_cid(cid: u64, context: &str) -> Result<()> {
    if cid == 0 {
        return Err(BilibiliError::api_rejected(format!(
            "{context} returned invalid cid"
        )));
    }
    Ok(())
}

fn select_page(pages: &[VideoPage], requested_p: u32, page_count: u32) -> Result<SelectedPage> {
    let page = pages
        .iter()
        .find(|page| page.page == requested_p)
        .or_else(|| pages.first())
        .ok_or_else(|| BilibiliError::video_not_found("video has no playable pages"))?;

    ensure_valid_cid(page.cid, "selected page")?;

    let part_title = if page_count > 1 {
        Some(page.part.clone())
    } else {
        None
    };

    Ok(SelectedPage {
        cid: page.cid,
        p: page.page,
        part_title,
        page_count,
        duration_secs: page.duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::http::mock::{MockResponse, MockTransport};

    #[test]
    fn fetch_view_selects_requested_part() {
        let transport = MockTransport::new(vec![(
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
        )]);

        let video_ref = ParsedVideoRef {
            bvid: Some("BV1xx411c7mD".to_string()),
            aid: None,
            p: Some(2),
            p_explicit: true,
        };

        let (view, selected) = fetch_view(&transport, &video_ref).unwrap();
        assert_eq!(view.title, "Main Title");
        assert_eq!(selected.cid, 2);
        assert_eq!(selected.part_title.as_deref(), Some("Part 2"));
        assert!(transport.requests()[0].send_cookie);
    }

    #[test]
    fn fetch_view_errors_when_part_out_of_range() {
        let transport = MockTransport::new(vec![(
            "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD&p=9",
            MockResponse::success(
                r#"{
                    "code": 0,
                    "data": {
                        "bvid": "BV1xx411c7mD",
                        "aid": 170001,
                        "title": "Main Title",
                        "pages": [
                            {"cid": 1, "page": 1, "part": "Part 1", "duration": 100}
                        ]
                    }
                }"#,
            ),
        )]);

        let video_ref = ParsedVideoRef {
            bvid: Some("BV1xx411c7mD".to_string()),
            aid: None,
            p: Some(9),
            p_explicit: true,
        };

        let err = fetch_view(&transport, &video_ref).unwrap_err();
        assert_eq!(err.code(), "PART_OUT_OF_RANGE");
    }

    #[test]
    fn fetch_view_maps_login_required_code() {
        let transport = MockTransport::new(vec![(
            "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD",
            MockResponse::success(r#"{"code":-101,"message":"账号未登录"}"#),
        )]);

        let video_ref = ParsedVideoRef {
            bvid: Some("BV1xx411c7mD".to_string()),
            aid: None,
            p: None,
            p_explicit: false,
        };

        let err = fetch_view(&transport, &video_ref).unwrap_err();
        assert_eq!(err.code(), "AUTH_REQUIRED");
    }

    #[test]
    fn fetch_view_rejects_zero_cid() {
        let transport = MockTransport::new(vec![(
            "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD",
            MockResponse::success(
                r#"{
                    "code": 0,
                    "data": {
                        "bvid": "BV1xx411c7mD",
                        "aid": 170001,
                        "title": "Main Title",
                        "cid": 0,
                        "pages": []
                    }
                }"#,
            ),
        )]);

        let video_ref = ParsedVideoRef {
            bvid: Some("BV1xx411c7mD".to_string()),
            aid: None,
            p: None,
            p_explicit: false,
        };

        let err = fetch_view(&transport, &video_ref).unwrap_err();
        assert_eq!(err.code(), "API_REJECTED");
    }

    #[test]
    fn fetch_view_uses_root_duration_when_pages_empty() {
        let transport = MockTransport::new(vec![(
            "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD",
            MockResponse::success(
                r#"{
                    "code": 0,
                    "data": {
                        "bvid": "BV1xx411c7mD",
                        "aid": 170001,
                        "title": "Main Title",
                        "cid": 42,
                        "duration": 321,
                        "pages": []
                    }
                }"#,
            ),
        )]);

        let video_ref = ParsedVideoRef {
            bvid: Some("BV1xx411c7mD".to_string()),
            aid: None,
            p: None,
            p_explicit: false,
        };

        let (_, selected) = fetch_view(&transport, &video_ref).unwrap();
        assert_eq!(selected.cid, 42);
        assert_eq!(selected.duration_secs, 321);
        assert_eq!(selected.p, 1);
    }

    #[test]
    fn select_page_records_actual_page_number_on_fallback() {
        let pages = vec![
            VideoPage {
                cid: 10,
                page: 2,
                part: "Only".to_string(),
                duration: 50,
            },
        ];
        let selected = select_page(&pages, 1, 1).unwrap();
        assert_eq!(selected.cid, 10);
        assert_eq!(selected.p, 2);
    }
}
