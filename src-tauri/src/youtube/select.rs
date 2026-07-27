use serde::Deserialize;
use url::Url;

use crate::bilibili::SubtitleSegment;

use super::error::{Result, YoutubeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionTrack {
    pub language_code: String,
    pub base_url: String,
    pub is_generated: bool,
}

#[derive(Debug, Deserialize)]
pub struct PlayerResponse {
    #[serde(rename = "playabilityStatus")]
    pub playability_status: Option<PlayabilityStatus>,
    #[serde(rename = "videoDetails")]
    pub video_details: Option<VideoDetails>,
    pub captions: Option<CaptionsBlock>,
}

#[derive(Debug, Deserialize)]
pub struct PlayabilityStatus {
    pub status: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VideoDetails {
    pub title: Option<String>,
    #[serde(rename = "lengthSeconds")]
    pub length_seconds: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CaptionsBlock {
    #[serde(rename = "playerCaptionsTracklistRenderer")]
    pub player_captions_tracklist_renderer: Option<CaptionTracklist>,
}

#[derive(Debug, Deserialize)]
pub struct CaptionTracklist {
    #[serde(rename = "captionTracks")]
    pub caption_tracks: Option<Vec<CaptionTrackRaw>>,
}

#[derive(Debug, Deserialize)]
pub struct CaptionTrackRaw {
    #[serde(rename = "languageCode")]
    pub language_code: Option<String>,
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Json3Caption {
    #[serde(default)]
    events: Vec<Json3Event>,
}

#[derive(Debug, Deserialize)]
struct Json3Event {
    #[serde(rename = "tStartMs")]
    t_start_ms: Option<u64>,
    #[serde(rename = "dDurationMs")]
    d_duration_ms: Option<u64>,
    #[serde(default)]
    segs: Vec<Json3Seg>,
}

#[derive(Debug, Deserialize)]
struct Json3Seg {
    #[serde(default)]
    utf8: String,
}

pub fn tracks_from_player(player: &PlayerResponse) -> Vec<CaptionTrack> {
    let Some(list) = player
        .captions
        .as_ref()
        .and_then(|c| c.player_captions_tracklist_renderer.as_ref())
        .and_then(|r| r.caption_tracks.as_ref())
    else {
        return Vec::new();
    };

    list.iter()
        .filter_map(|raw| {
            let language_code = raw.language_code.as_deref()?.trim();
            let base_url = raw.base_url.as_deref()?.trim();
            if language_code.is_empty() || base_url.is_empty() {
                return None;
            }
            Some(CaptionTrack {
                language_code: language_code.to_string(),
                base_url: base_url.to_string(),
                is_generated: raw.kind.as_deref() == Some("asr"),
            })
        })
        .collect()
}

/// Align with BiliNote: zh-Hans > zh > zh-CN > zh-TW > en > en-US > ja,
/// preferring manual captions over auto-generated.
pub fn select_caption_track<'a>(tracks: &'a [CaptionTrack]) -> Option<&'a CaptionTrack> {
    const LANGS: &[&str] = &["zh-Hans", "zh", "zh-CN", "zh-TW", "en", "en-US", "ja"];

    for &lang in LANGS {
        if let Some(track) = tracks
            .iter()
            .find(|t| !t.is_generated && lang_eq(&t.language_code, lang))
        {
            return Some(track);
        }
    }
    for &lang in LANGS {
        if let Some(track) = tracks
            .iter()
            .find(|t| t.is_generated && lang_eq(&t.language_code, lang))
        {
            return Some(track);
        }
    }
    tracks.first()
}

fn lang_eq(code: &str, wanted: &str) -> bool {
    code.eq_ignore_ascii_case(wanted)
}

pub fn parse_json3_captions(body: &str) -> Result<Vec<SubtitleSegment>> {
    let parsed: Json3Caption = serde_json::from_str(body).map_err(|_| {
        YoutubeError::subtitle_corrupt("failed to parse YouTube caption JSON")
    })?;

    let mut segments = Vec::new();
    for event in parsed.events {
        let text = event
            .segs
            .iter()
            .map(|s| s.utf8.as_str())
            .collect::<String>()
            .replace('\n', " ")
            .trim()
            .to_string();
        if text.is_empty() {
            continue;
        }
        let start_ms = event.t_start_ms.unwrap_or(0);
        let duration_ms = event.d_duration_ms.unwrap_or(0);
        let end_ms = start_ms.saturating_add(duration_ms.max(1));
        segments.push(SubtitleSegment {
            start_ms,
            end_ms,
            text,
        });
    }

    if segments.is_empty() {
        return Err(YoutubeError::no_subtitle(
            "YouTube caption track contained no text",
        ));
    }

    Ok(segments)
}

pub fn with_json3_format(base_url: &str) -> Result<String> {
    let mut url = Url::parse(base_url)
        .map_err(|_| YoutubeError::subtitle_corrupt("invalid YouTube caption URL"))?;
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .filter(|(k, _)| k != "fmt")
        .collect();
    pairs.push(("fmt".to_string(), "json3".to_string()));
    url.set_query(None);
    {
        let mut qp = url.query_pairs_mut();
        for (k, v) in &pairs {
            qp.append_pair(k, v);
        }
    }
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_prefers_manual_zh_over_asr_en() {
        let tracks = vec![
            CaptionTrack {
                language_code: "en".into(),
                base_url: "https://example/en".into(),
                is_generated: false,
            },
            CaptionTrack {
                language_code: "zh-Hans".into(),
                base_url: "https://example/zh".into(),
                is_generated: false,
            },
            CaptionTrack {
                language_code: "zh".into(),
                base_url: "https://example/zh-asr".into(),
                is_generated: true,
            },
        ];
        let selected = select_caption_track(&tracks).unwrap();
        assert_eq!(selected.language_code, "zh-Hans");
        assert!(!selected.is_generated);
    }

    #[test]
    fn parse_json3_builds_segments() {
        let body = r#"{
          "events": [
            {"tStartMs": 1200, "dDurationMs": 2160, "segs": [{"utf8": "All right"}]},
            {"tStartMs": 5000, "dDurationMs": 1000, "segs": [{"utf8": "hello"}, {"utf8": " world"}]}
          ]
        }"#;
        let segs = parse_json3_captions(body).unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].start_ms, 1200);
        assert_eq!(segs[0].end_ms, 3360);
        assert_eq!(segs[0].text, "All right");
        assert_eq!(segs[1].text, "hello world");
    }
}
