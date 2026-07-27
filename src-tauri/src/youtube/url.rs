use url::Url;

use super::error::{Result, YoutubeError};

pub fn is_youtube_url(input: &str) -> bool {
    extract_video_id(input).is_ok()
}

pub fn extract_video_id(input: &str) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(YoutubeError::invalid_url("URL must not be empty"));
    }

    let parsed = Url::parse(trimmed).map_err(|_| YoutubeError::invalid_url("invalid URL"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(YoutubeError::invalid_url("URL must use http or https"));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| YoutubeError::invalid_url("URL host is required"))?
        .to_ascii_lowercase();

    if !is_youtube_host(&host) {
        return Err(YoutubeError::invalid_url("unsupported YouTube host"));
    }

    if let Some(v) = parsed
        .query_pairs()
        .find(|(k, _)| k == "v")
        .map(|(_, v)| v.into_owned())
    {
        if is_video_id(&v) {
            return Ok(v);
        }
    }

    let path = parsed.path();
    if host == "youtu.be" {
        if let Some(id) = path.trim_matches('/').split('/').next() {
            if is_video_id(id) {
                return Ok(id.to_string());
            }
        }
    }

    for prefix in ["/shorts/", "/embed/", "/live/", "/v/"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            let id = rest.split('/').next().unwrap_or("");
            if is_video_id(id) {
                return Ok(id.to_string());
            }
        }
    }

    Err(YoutubeError::invalid_url(
        "could not extract YouTube video id",
    ))
}

fn is_youtube_host(host: &str) -> bool {
    host == "youtu.be"
        || host == "youtube.com"
        || host == "www.youtube.com"
        || host == "m.youtube.com"
        || host == "music.youtube.com"
        || host.ends_with(".youtube.com")
}

fn is_video_id(value: &str) -> bool {
    value.len() == 11
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_watch_url() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn parse_short_and_shorts() {
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
        assert_eq!(
            extract_video_id("https://www.youtube.com/shorts/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn reject_non_youtube() {
        assert!(extract_video_id("https://www.bilibili.com/video/BV1xx411c7mD").is_err());
    }
}
