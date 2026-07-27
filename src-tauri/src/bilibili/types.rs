use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BilibiliSubtitleResult {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_title: Option<String>,
    pub bvid: String,
    pub aid: u64,
    pub cid: u64,
    pub p: u32,
    pub page_count: u32,
    /// Video/page duration in milliseconds when known from the view API.
    #[serde(default)]
    pub duration_ms: u64,
    pub language: String,
    pub segments: Vec<SubtitleSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedVideoRef {
    pub bvid: Option<String>,
    pub aid: Option<u64>,
    pub p: Option<u32>,
    pub p_explicit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoPage {
    pub cid: u64,
    pub page: u32,
    pub part: String,
    pub duration: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewData {
    pub bvid: String,
    pub aid: u64,
    pub title: String,
    pub pages: Vec<VideoPage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedPage {
    pub cid: u64,
    pub p: u32,
    pub part_title: Option<String>,
    pub page_count: u32,
    /// Page duration in seconds from the view API (0 if unknown).
    pub duration_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SubtitleTrack {
    pub lan: String,
    #[serde(default)]
    pub lan_doc: String,
    #[serde(default)]
    pub ai_type: u32,
    pub subtitle_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ViewApiResponse {
    pub code: i32,
    #[serde(default)]
    pub message: String,
    pub data: Option<ViewApiData>,
}

#[derive(Debug, Deserialize)]
pub struct ViewApiData {
    pub bvid: String,
    pub aid: u64,
    pub title: String,
    #[serde(default)]
    pub cid: u64,
    #[serde(default)]
    pub pages: Vec<ViewApiPage>,
}

#[derive(Debug, Deserialize)]
pub struct ViewApiPage {
    pub cid: u64,
    pub page: u32,
    pub part: String,
    #[serde(default)]
    pub duration: u64,
}

#[derive(Debug, Deserialize)]
pub struct PlayerApiResponse {
    pub code: i32,
    #[serde(default)]
    pub message: String,
    pub data: Option<PlayerApiData>,
}

#[derive(Debug, Deserialize)]
pub struct PlayerApiData {
    #[serde(default)]
    pub subtitle: PlayerSubtitleBlock,
}

#[derive(Debug, Default, Deserialize)]
pub struct PlayerSubtitleBlock {
    #[serde(default)]
    pub subtitles: Vec<SubtitleTrack>,
}

#[derive(Debug, Deserialize)]
pub struct SubtitleBodyResponse {
    #[serde(default)]
    pub body: Vec<SubtitleBodyItem>,
}

#[derive(Debug, Deserialize)]
pub struct SubtitleBodyItem {
    #[serde(default)]
    pub from: f64,
    #[serde(default)]
    pub to: f64,
    #[serde(default)]
    pub content: String,
}
