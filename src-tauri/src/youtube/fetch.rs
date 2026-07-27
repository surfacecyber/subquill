use std::time::Duration;

use serde_json::json;

use crate::bilibili::{BilibiliSubtitleResult, SubtitleSegment};

use super::error::{Result, YoutubeError};
use super::select::{
    parse_json3_captions, select_caption_track, tracks_from_player, with_json3_format, PlayerResponse,
};
use super::url::extract_video_id;

/// Public InnerTube key used by youtube-transcript-api / Android clients.
const INNERTUBE_API_KEY: &str = "AIzaSyA8eiZmMIgJUUsuYKE20kFt96RbMwTfBvg";
const PLAYER_ENDPOINT: &str = "https://www.youtube.com/youtubei/v1/player";
const ANDROID_UA: &str = "com.google.android.youtube/20.10.38 (Linux; U; Android 14) gzip";
const CAPTION_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoutubeSubtitleResult {
    pub title: String,
    pub video_id: String,
    pub language: String,
    pub duration_ms: u64,
    pub segments: Vec<SubtitleSegment>,
}

impl YoutubeSubtitleResult {
    /// Adapt into the existing note-job pipeline shape (`bvid` holds the video id).
    pub fn into_pipeline_result(self) -> BilibiliSubtitleResult {
        BilibiliSubtitleResult {
            title: self.title,
            part_title: None,
            bvid: self.video_id,
            aid: 0,
            cid: 0,
            p: 1,
            page_count: 1,
            duration_ms: self.duration_ms,
            language: self.language,
            segments: self.segments,
        }
    }
}

pub fn fetch_subtitles(input_url: &str) -> Result<YoutubeSubtitleResult> {
    let video_id = extract_video_id(input_url)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| YoutubeError::network(format!("failed to build HTTP client: {err}")))?;

    let player = fetch_player(&client, &video_id)?;
    let status = player
        .playability_status
        .as_ref()
        .and_then(|s| s.status.as_deref())
        .unwrap_or("");
    if status == "ERROR" || status == "LOGIN_REQUIRED" {
        let reason = player
            .playability_status
            .as_ref()
            .and_then(|s| s.reason.clone())
            .unwrap_or_else(|| status.to_string());
        if status == "LOGIN_REQUIRED" {
            return Err(YoutubeError::api_rejected(format!(
                "YouTube requires login for this video: {reason}"
            )));
        }
        return Err(YoutubeError::video_not_found(format!(
            "YouTube video unavailable: {reason}"
        )));
    }

    let title = player
        .video_details
        .as_ref()
        .and_then(|d| d.title.clone())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| video_id.clone());

    let duration_ms = player
        .video_details
        .as_ref()
        .and_then(|d| d.length_seconds.as_deref())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_mul(1000);

    let tracks = tracks_from_player(&player);
    let track = select_caption_track(&tracks).ok_or_else(|| {
        YoutubeError::no_subtitle("no usable YouTube caption tracks available")
    })?;

    let caption_url = with_json3_format(&track.base_url)?;
    let body = client
        .get(&caption_url)
        .header("User-Agent", CAPTION_UA)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .map_err(|err| YoutubeError::network(format!("caption download failed: {err}")))?;

    if !body.status().is_success() {
        return Err(YoutubeError::network(format!(
            "caption download HTTP {}",
            body.status()
        )));
    }

    let text = body
        .text()
        .map_err(|err| YoutubeError::network(format!("caption body read failed: {err}")))?;
    let segments = parse_json3_captions(&text)?;

    Ok(YoutubeSubtitleResult {
        title,
        video_id,
        language: track.language_code.clone(),
        duration_ms,
        segments,
    })
}

fn fetch_player(client: &reqwest::blocking::Client, video_id: &str) -> Result<PlayerResponse> {
    let url = format!("{PLAYER_ENDPOINT}?key={INNERTUBE_API_KEY}");
    let body = json!({
        "context": {
            "client": {
                "clientName": "ANDROID",
                "clientVersion": "20.10.38",
                "hl": "en",
                "gl": "US"
            }
        },
        "videoId": video_id
    });

    let response = client
        .post(&url)
        .header("User-Agent", ANDROID_UA)
        .header("Content-Type", "application/json")
        .header("X-YouTube-Client-Name", "3")
        .header("X-YouTube-Client-Version", "20.10.38")
        .json(&body)
        .send()
        .map_err(|err| YoutubeError::network(format!("player API request failed: {err}")))?;

    if !response.status().is_success() {
        return Err(YoutubeError::network(format!(
            "player API HTTP {}",
            response.status()
        )));
    }

    response
        .json::<PlayerResponse>()
        .map_err(|err| YoutubeError::api_rejected(format!("invalid player API JSON: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_fetch_me_at_the_zoo() {
        let result = fetch_subtitles("https://www.youtube.com/watch?v=jNQXAC9IVRw")
            .expect("live YouTube subtitle fetch");
        assert_eq!(result.video_id, "jNQXAC9IVRw");
        assert!(!result.title.is_empty());
        assert!(!result.segments.is_empty());
        assert!(result.segments.iter().any(|s| s.text.to_ascii_lowercase().contains("elephant")
            || s.text.to_ascii_lowercase().contains("trunk")
            || s.text.to_ascii_lowercase().contains("all right")));
        let pipeline = result.into_pipeline_result();
        assert_eq!(pipeline.bvid, "jNQXAC9IVRw");
        assert!(!pipeline.segments.is_empty());
    }
}
