use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ErrorPayload;

/// Maximum wall-clock time for a single note-generation item (one URL).
pub const JOB_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Soft cap on URLs accepted in one batch start.
pub const MAX_BATCH_URLS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobProgressStage {
    FetchingSubtitles,
    Chunking,
    CallingLlm,
    Merging,
    Rendering,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchItemStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchItemView {
    pub index: usize,
    pub url: String,
    pub status: BatchItemStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Safe error code only — never includes message or secrets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobProgress {
    pub job_id: String,
    pub stage: JobProgressStage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<usize>,
    /// Safe error code only — never includes message or secrets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// 0-based index of the URL currently being processed (batch-aware).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_url: Option<String>,
}

impl JobProgress {
    pub fn new(job_id: Uuid, stage: JobProgressStage) -> Self {
        Self {
            job_id: job_id.to_string(),
            stage,
            chunk_index: None,
            chunk_count: None,
            error_code: None,
            item_index: None,
            item_total: None,
            item_url: None,
        }
    }

    pub fn with_chunks(mut self, chunk_index: Option<usize>, chunk_count: usize) -> Self {
        self.chunk_index = chunk_index;
        self.chunk_count = Some(chunk_count);
        self
    }

    pub fn with_error_code(mut self, code: &'static str) -> Self {
        self.error_code = Some(code.to_string());
        self
    }

    pub fn with_batch(mut self, item_index: usize, item_total: usize, item_url: &str) -> Self {
        self.item_index = Some(item_index);
        self.item_total = Some(item_total);
        self.item_url = Some(item_url.to_string());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalTransition {
    pub status: JobStatus,
    pub progress: JobProgress,
}

/// One successful note within a batch job (returned via [`JobResult::batch_results`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchItemResult {
    pub index: usize,
    pub url: String,
    pub markdown: String,
    pub title: String,
    pub bvid: String,
    pub language: String,
    pub segment_count: usize,
    /// Absolute path when auto-save succeeded; omitted when disabled or failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobResult {
    pub markdown: String,
    pub title: String,
    pub bvid: String,
    pub language: String,
    pub segment_count: usize,
    /// Absolute path when auto-save succeeded; omitted when disabled or failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_path: Option<String>,
    /// All successful per-URL notes for multi-URL jobs.
    /// Empty / omitted for single-URL jobs so the primary fields remain the sole payload.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub batch_results: Vec<BatchItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartJobResponse {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub status: JobStatus,
    pub progress: JobProgress,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorPayload>,
    /// Per-URL outcomes for the active/last batch (length >= 1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub batch_items: Vec<BatchItemView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelJobResponse {
    pub job_id: String,
    pub status: JobStatus,
    /// True when cancellation was requested for an active job. Status may still
    /// be `running` until the background task observes the cancel token.
    pub cancel_requested: bool,
}

pub fn parse_job_id(job_id: &str) -> Result<Uuid, ErrorPayload> {
    Uuid::parse_str(job_id.trim()).map_err(|_| ErrorPayload {
        code: "VALIDATION_ERROR",
        message: "job_id must be a valid UUID".to_string(),
    })
}

/// Trim, drop empties, dedupe (first wins), enforce max batch size.
pub fn normalize_job_urls(urls: Vec<String>) -> Result<Vec<String>, ErrorPayload> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for url in urls {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !seen.insert(trimmed.to_string()) {
            continue;
        }
        out.push(trimmed.to_string());
    }

    if out.is_empty() {
        return Err(ErrorPayload {
            code: "VALIDATION_ERROR",
            message: "urls is required".to_string(),
        });
    }

    if out.len() > MAX_BATCH_URLS {
        return Err(ErrorPayload {
            code: "VALIDATION_ERROR",
            message: format!("at most {MAX_BATCH_URLS} URLs are allowed per batch"),
        });
    }

    Ok(out)
}

pub fn job_not_found() -> ErrorPayload {
    ErrorPayload {
        code: "JOB_NOT_FOUND",
        message: "No job exists with the given job_id".to_string(),
    }
}

pub fn job_not_ready() -> ErrorPayload {
    ErrorPayload {
        code: "JOB_NOT_READY",
        message: "Job has not completed yet".to_string(),
    }
}

pub fn job_already_running() -> ErrorPayload {
    ErrorPayload {
        code: "JOB_ALREADY_RUNNING",
        message: "Another note generation job is already active".to_string(),
    }
}

pub fn job_cancelled() -> ErrorPayload {
    ErrorPayload {
        code: "JOB_CANCELLED",
        message: "Note generation was cancelled".to_string(),
    }
}

pub fn job_timeout() -> ErrorPayload {
    ErrorPayload {
        code: "JOB_TIMEOUT",
        message: "Note generation exceeded the time limit".to_string(),
    }
}

pub fn internal_error(message: impl Into<String>) -> ErrorPayload {
    ErrorPayload {
        code: "INTERNAL_ERROR",
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_job_urls_trims_dedupes_and_rejects_empty() {
        let urls = normalize_job_urls(vec![
            "  https://a.example/1  ".to_string(),
            "".to_string(),
            "https://a.example/1".to_string(),
            "https://b.example/2".to_string(),
        ])
        .expect("ok");
        assert_eq!(
            urls,
            vec![
                "https://a.example/1".to_string(),
                "https://b.example/2".to_string()
            ]
        );

        let err = normalize_job_urls(vec!["  ".to_string(), "".to_string()]).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn normalize_job_urls_enforces_max_batch() {
        let urls = (0..=MAX_BATCH_URLS)
            .map(|i| format!("https://example.com/{i}"))
            .collect::<Vec<_>>();
        let err = normalize_job_urls(urls).unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert!(err.message.contains(&MAX_BATCH_URLS.to_string()));
    }
}
