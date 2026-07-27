mod api;
mod bounded;
mod error;
mod http;
mod player;
mod service;
mod subtitle;
mod types;
mod url;
mod view;

pub use service::fetch_subtitles;
pub use types::{BilibiliSubtitleResult, SubtitleSegment};

#[cfg(test)]
mod integration_tests {
    use super::http::mock::{MockResponse, MockTransport};
    use super::service::fetch_subtitles_with_transport;

    #[test]
    fn integration_view_player_subtitle_chain() {
        let transport = MockTransport::new(vec![
            (
                "https://api.bilibili.com/x/web-interface/view?bvid=BV1xx411c7mD",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "bvid": "BV1xx411c7mD",
                            "aid": 170001,
                            "title": "Integration Title",
                            "cid": 99,
                            "pages": []
                        }
                    }"#,
                ),
            ),
            (
                "https://api.bilibili.com/x/player/wbi/v2?bvid=BV1xx411c7mD&cid=99",
                MockResponse::success(
                    r#"{
                        "code": 0,
                        "data": {
                            "subtitle": {
                                "subtitles": [
                                    {
                                        "lan": "en-US",
                                        "subtitle_url": "https://subtitle.bilibili.com/en.json"
                                    },
                                    {
                                        "lan": "zh-CN",
                                        "ai_type": 1,
                                        "subtitle_url": "https://subtitle.bilibili.com/zh.json"
                                    }
                                ]
                            }
                        }
                    }"#,
                ),
            ),
            (
                "https://subtitle.bilibili.com/zh.json",
                MockResponse::success(r#"{"body":[{"from":0.5,"to":1.5,"content":"测试"},{"from":1.5,"to":2.5,"content":"内容"},{"from":2.5,"to":3.5,"content":"完整"}]}"#),
            ),
        ]);

        let result = fetch_subtitles_with_transport(
            &transport,
            "https://www.bilibili.com/video/BV1xx411c7mD",
        )
        .expect("integration chain");

        assert_eq!(result.title, "Integration Title");
        assert_eq!(result.language, "zh-CN");
        assert_eq!(result.segments[0].start_ms, 500);
    }
}
