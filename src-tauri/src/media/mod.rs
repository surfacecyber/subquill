use crate::bilibili::{self, BilibiliSubtitleResult, VideoPreview as BiliPreview};
use crate::error::ErrorPayload;
use crate::youtube::{self, is_youtube_url, VideoPreview as YtPreview};
use serde::Serialize;

/// Unified lightweight video preview for the workspace UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoPreview {
    pub title: String,
    pub platform: String,
    pub video_id: String,
    pub duration_ms: u64,
    pub p: u32,
    pub page_count: u32,
    pub part_title: Option<String>,
    pub has_subtitles: bool,
    pub auth_required: bool,
}

impl From<BiliPreview> for VideoPreview {
    fn from(value: BiliPreview) -> Self {
        Self {
            title: value.title,
            platform: value.platform.to_string(),
            video_id: value.video_id,
            duration_ms: value.duration_ms,
            p: value.p,
            page_count: value.page_count,
            part_title: value.part_title,
            has_subtitles: value.has_subtitles,
            auth_required: value.auth_required,
        }
    }
}

impl From<YtPreview> for VideoPreview {
    fn from(value: YtPreview) -> Self {
        Self {
            title: value.title,
            platform: value.platform.to_string(),
            video_id: value.video_id,
            duration_ms: value.duration_ms,
            p: value.p,
            page_count: value.page_count,
            part_title: value.part_title,
            has_subtitles: value.has_subtitles,
            auth_required: value.auth_required,
        }
    }
}

/// Fetch subtitles for a Bilibili or YouTube URL.
/// `cookie` is only used for Bilibili (SESSDATA).
pub fn fetch_subtitles(
    input_url: &str,
    cookie: Option<&str>,
) -> Result<BilibiliSubtitleResult, ErrorPayload> {
    let trimmed = input_url.trim();
    if trimmed.is_empty() {
        return Err(ErrorPayload::from(crate::error::Error::validation(
            "URL must not be empty",
        )));
    }

    if is_youtube_url(trimmed) {
        return youtube::fetch_subtitles(trimmed)
            .map(youtube::YoutubeSubtitleResult::into_pipeline_result)
            .map_err(ErrorPayload::from);
    }

    bilibili::fetch_subtitles(trimmed, cookie).map_err(ErrorPayload::from)
}

/// Lightweight metadata preview (no subtitle body download).
pub fn preview_video(
    input_url: &str,
    cookie: Option<&str>,
) -> Result<VideoPreview, ErrorPayload> {
    let trimmed = input_url.trim();
    if trimmed.is_empty() {
        return Err(ErrorPayload::from(crate::error::Error::validation(
            "URL must not be empty",
        )));
    }

    if is_youtube_url(trimmed) {
        return youtube::preview_video(trimmed)
            .map(VideoPreview::from)
            .map_err(ErrorPayload::from);
    }

    bilibili::preview_video(trimmed, cookie)
        .map(VideoPreview::from)
        .map_err(ErrorPayload::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_youtube_urls() {
        assert!(is_youtube_url("https://youtu.be/jNQXAC9IVRw"));
        assert!(!is_youtube_url(
            "https://www.bilibili.com/video/BV1xx411c7mD"
        ));
    }

    #[test]
    fn preview_rejects_empty_url() {
        let err = preview_video("  ", None).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }
}
