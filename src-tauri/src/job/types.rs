use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ErrorPayload;

/// Maximum wall-clock time for a single note-generation job.
pub const JOB_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30 * 60);

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
}

impl JobProgress {
    pub fn new(job_id: Uuid, stage: JobProgressStage) -> Self {
        Self {
            job_id: job_id.to_string(),
            stage,
            chunk_index: None,
            chunk_count: None,
            error_code: None,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalTransition {
    pub status: JobStatus,
    pub progress: JobProgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobResult {
    pub markdown: String,
    pub title: String,
    pub bvid: String,
    pub language: String,
    pub segment_count: usize,
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
