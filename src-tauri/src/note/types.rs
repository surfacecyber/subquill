use crate::bilibili::SubtitleSegment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteLocale {
    Zh,
    En,
}

impl NoteLocale {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" => Self::En,
            _ => Self::Zh,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoMetadata {
    pub title: String,
    pub part_title: Option<String>,
    pub bvid: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NoteSection {
    pub start_ms: u64,
    pub end_ms: u64,
    pub title: String,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteData {
    pub metadata: VideoMetadata,
    pub summary: String,
    pub sections: Vec<NoteSection>,
    pub segments: Vec<SubtitleSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleChunk {
    pub index: usize,
    pub start_ms: u64,
    pub end_ms: u64,
    pub segments: Vec<SubtitleSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkLlmOutput {
    pub summary: String,
    pub sections: Vec<NoteSection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteProgressStage {
    Chunking,
    AnalyzingChunk,
    MergingSummaries,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteProgress {
    pub stage: NoteProgressStage,
    pub chunk_index: Option<usize>,
    pub chunk_count: usize,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ChunkLlmOutputRaw {
    pub summary: String,
    pub sections: Vec<NoteSection>,
}
