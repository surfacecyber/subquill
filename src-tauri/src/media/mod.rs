use crate::bilibili::{self, BilibiliSubtitleResult};
use crate::error::ErrorPayload;
use crate::youtube::{self, is_youtube_url};

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
}
