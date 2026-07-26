use super::error::{BilibiliError, Result};
use super::http::{HttpRequest, HttpTransport};
use super::types::{SubtitleBodyItem, SubtitleBodyResponse, SubtitleSegment, SubtitleTrack};
use super::url::{is_subtitle_cdn_host, normalize_subtitle_url};

pub fn select_subtitle_track(tracks: &[SubtitleTrack]) -> Result<&SubtitleTrack> {
    if tracks.is_empty() {
        return Err(BilibiliError::no_subtitle("no subtitle tracks available"));
    }

    if let Some(track) = tracks
        .iter()
        .find(|track| is_chinese(track) && is_manual(track) && has_subtitle_url(track))
    {
        return Ok(track);
    }

    if let Some(track) = tracks
        .iter()
        .find(|track| is_chinese(track) && is_ai(track) && has_subtitle_url(track))
    {
        return Ok(track);
    }

    if let Some(track) = tracks
        .iter()
        .find(|track| is_chinese(track) && has_subtitle_url(track))
    {
        return Ok(track);
    }

    tracks
        .iter()
        .find(|track| has_subtitle_url(track))
        .ok_or_else(|| {
            BilibiliError::auth_required(
                "subtitle tracks exist but no downloadable subtitle_url was returned; login may be required",
            )
        })
}

pub fn fetch_subtitle_segments(
    transport: &dyn HttpTransport,
    subtitle_url: &str,
) -> Result<Vec<SubtitleSegment>> {
    let normalized = normalize_subtitle_url(subtitle_url)?;
    let parsed = url::Url::parse(&normalized)
        .map_err(|_| BilibiliError::subtitle_corrupt("subtitle URL is invalid"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| BilibiliError::subtitle_corrupt("subtitle URL host is required"))?;
    if !is_subtitle_cdn_host(host) {
        return Err(BilibiliError::subtitle_corrupt(
            "subtitle URL host is not allowed",
        ));
    }

    let response = transport.send(HttpRequest::subtitle(normalized).without_cookie())?;
    if !response.is_success() {
        return Err(BilibiliError::network(format!(
            "subtitle download failed with HTTP {}",
            response.status
        )));
    }

    let payload: SubtitleBodyResponse = serde_json::from_str(&response.body).map_err(|err| {
        BilibiliError::subtitle_corrupt(format!("subtitle JSON is invalid: {err}"))
    })?;

    parse_subtitle_body(&payload.body)
}

pub fn parse_subtitle_body(items: &[SubtitleBodyItem]) -> Result<Vec<SubtitleSegment>> {
    let mut segments = Vec::new();

    for item in items {
        let text = item.content.trim();
        if text.is_empty() {
            continue;
        }

        let start_ms = seconds_to_millis(item.from)?;
        let end_ms = seconds_to_millis(item.to)?;
        if end_ms <= start_ms {
            continue;
        }

        segments.push(SubtitleSegment {
            start_ms,
            end_ms,
            text: text.to_string(),
        });
    }

    if segments.is_empty() {
        return Err(BilibiliError::subtitle_corrupt(
            "subtitle payload contained no valid segments",
        ));
    }

    Ok(segments)
}

fn is_chinese(track: &SubtitleTrack) -> bool {
    let lan = track.lan.to_ascii_lowercase();
    lan.starts_with("zh") || lan == "ai-zh"
}

fn is_manual(track: &SubtitleTrack) -> bool {
    track.ai_type == 0
}

fn is_ai(track: &SubtitleTrack) -> bool {
    track.ai_type != 0
}

fn has_subtitle_url(track: &SubtitleTrack) -> bool {
    !track.subtitle_url.trim().is_empty()
}

fn seconds_to_millis(value: f64) -> Result<u64> {
    if !value.is_finite() || value < 0.0 {
        return Err(BilibiliError::subtitle_corrupt("subtitle time is invalid"));
    }
    Ok((value * 1000.0).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(lan: &str, ai_type: u32, subtitle_url: &str) -> SubtitleTrack {
        SubtitleTrack {
            lan: lan.to_string(),
            lan_doc: String::new(),
            ai_type,
            subtitle_url: subtitle_url.to_string(),
        }
    }

    #[test]
    fn select_prefers_manual_chinese_over_ai_chinese() {
        let tracks = vec![
            track("zh-CN", 1, "https://example.bilibili.com/ai.json"),
            track("zh-CN", 0, "https://example.bilibili.com/manual.json"),
        ];

        let selected = select_subtitle_track(&tracks).unwrap();
        assert_eq!(
            selected.subtitle_url,
            "https://example.bilibili.com/manual.json"
        );
    }

    #[test]
    fn select_prefers_ai_chinese_over_english() {
        let tracks = vec![
            track("en-US", 0, "https://example.bilibili.com/en.json"),
            track("zh-CN", 1, "https://example.bilibili.com/ai.json"),
        ];

        let selected = select_subtitle_track(&tracks).unwrap();
        assert_eq!(
            selected.subtitle_url,
            "https://example.bilibili.com/ai.json"
        );
    }

    #[test]
    fn select_returns_auth_required_when_urls_missing() {
        let tracks = vec![track("zh-CN", 0, "")];
        let err = select_subtitle_track(&tracks).unwrap_err();
        assert_eq!(err.code(), "AUTH_REQUIRED");
    }

    #[test]
    fn parse_subtitle_body_converts_seconds_to_milliseconds() {
        let segments = parse_subtitle_body(&[SubtitleBodyItem {
            from: 1.25,
            to: 2.5,
            content: " hello ".to_string(),
        }])
        .unwrap();

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_ms, 1250);
        assert_eq!(segments[0].end_ms, 2500);
        assert_eq!(segments[0].text, "hello");
    }

    #[test]
    fn parse_subtitle_body_filters_invalid_segments() {
        let segments = parse_subtitle_body(&[
            SubtitleBodyItem {
                from: 0.0,
                to: 0.0,
                content: "invalid timing".to_string(),
            },
            SubtitleBodyItem {
                from: 1.0,
                to: 2.0,
                content: "   ".to_string(),
            },
            SubtitleBodyItem {
                from: 1.0,
                to: 2.0,
                content: "valid".to_string(),
            },
        ])
        .unwrap();

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "valid");
    }
}
