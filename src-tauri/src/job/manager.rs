use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

use crate::error::ErrorPayload;

use super::types::{
    internal_error, job_already_running, job_cancelled, job_not_found, job_not_ready, parse_job_id,
    CancelJobResponse, JobProgress, JobProgressStage, JobResult, JobStatus, JobStatusResponse,
    TerminalTransition,
};

struct JobRecord {
    id: Uuid,
    status: JobStatus,
    progress: JobProgress,
    result: Option<JobResult>,
    error: Option<ErrorPayload>,
    cancel_token: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct StartHandle {
    pub job_id: Uuid,
    pub cancel_token: Arc<AtomicBool>,
    pub started_at: Instant,
}

#[derive(Default)]
pub struct JobManager {
    current: Mutex<Option<JobRecord>>,
}

impl JobManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_start(&self) -> std::result::Result<StartHandle, ErrorPayload> {
        let mut guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        if let Some(ref job) = *guard {
            if job.status.is_active() {
                return Err(job_already_running());
            }
        }

        let job_id = Uuid::new_v4();
        let cancel_token = Arc::new(AtomicBool::new(false));
        let started_at = Instant::now();
        let progress = JobProgress::new(job_id, JobProgressStage::FetchingSubtitles);

        *guard = Some(JobRecord {
            id: job_id,
            status: JobStatus::Queued,
            progress,
            result: None,
            error: None,
            cancel_token: Arc::clone(&cancel_token),
        });

        Ok(StartHandle {
            job_id,
            cancel_token,
            started_at,
        })
    }

    pub fn mark_running(&self, job_id: Uuid) -> Result<(), ErrorPayload> {
        let mut guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        let job = guard
            .as_mut()
            .filter(|job| job.id == job_id)
            .ok_or_else(job_not_found)?;

        if job.status.is_terminal() {
            return Ok(());
        }

        job.status = JobStatus::Running;
        Ok(())
    }

    pub fn update_progress(&self, job_id: Uuid, progress: JobProgress) -> Result<(), ErrorPayload> {
        let mut guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        let job = guard
            .as_mut()
            .filter(|job| job.id == job_id)
            .ok_or_else(job_not_found)?;

        if job.status.is_terminal() {
            return Ok(());
        }

        job.progress = progress;
        Ok(())
    }

    pub fn complete(
        &self,
        job_id: Uuid,
        result: JobResult,
    ) -> Result<TerminalTransition, ErrorPayload> {
        let mut guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        let job = guard
            .as_mut()
            .filter(|job| job.id == job_id)
            .ok_or_else(job_not_found)?;

        if job.status.is_terminal() {
            return Ok(TerminalTransition {
                status: job.status,
                progress: job.progress.clone(),
            });
        }

        if job.cancel_token.load(Ordering::SeqCst) {
            return Ok(apply_terminal_failure(job, job_cancelled()));
        }

        job.status = JobStatus::Completed;
        job.progress = JobProgress::new(job_id, JobProgressStage::Done);
        job.result = Some(result);
        job.error = None;

        Ok(TerminalTransition {
            status: job.status,
            progress: job.progress.clone(),
        })
    }

    pub fn fail(
        &self,
        job_id: Uuid,
        error: ErrorPayload,
    ) -> Result<TerminalTransition, ErrorPayload> {
        let mut guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        let job = guard
            .as_mut()
            .filter(|job| job.id == job_id)
            .ok_or_else(job_not_found)?;

        if job.status.is_terminal() {
            return Ok(TerminalTransition {
                status: job.status,
                progress: job.progress.clone(),
            });
        }

        Ok(apply_terminal_failure(job, error))
    }

    pub fn get_status(&self, job_id: &str) -> Result<JobStatusResponse, ErrorPayload> {
        let job_id = parse_job_id(job_id)?;
        let guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        let job = guard
            .as_ref()
            .filter(|job| job.id == job_id)
            .ok_or_else(job_not_found)?;

        Ok(JobStatusResponse {
            job_id: job.id.to_string(),
            status: job.status,
            progress: job.progress.clone(),
            error: job.error.clone(),
        })
    }

    pub fn get_result(&self, job_id: &str) -> Result<JobResult, ErrorPayload> {
        let job_id = parse_job_id(job_id)?;
        let guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        let job = guard
            .as_ref()
            .filter(|job| job.id == job_id)
            .ok_or_else(job_not_found)?;

        match job.status {
            JobStatus::Completed => job
                .result
                .clone()
                .ok_or_else(|| internal_error("Completed job is missing result")),
            JobStatus::Queued | JobStatus::Running => Err(job_not_ready()),
            JobStatus::Failed => Err(job
                .error
                .clone()
                .unwrap_or_else(|| internal_error("Failed job is missing error"))),
            JobStatus::Cancelled => Err(job.error.clone().unwrap_or_else(job_cancelled)),
        }
    }

    pub fn cancel(&self, job_id: &str) -> Result<CancelJobResponse, ErrorPayload> {
        let job_id = parse_job_id(job_id)?;
        let mut guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        let job = guard
            .as_mut()
            .filter(|job| job.id == job_id)
            .ok_or_else(job_not_found)?;

        if job.status.is_terminal() {
            return Ok(CancelJobResponse {
                job_id: job.id.to_string(),
                status: job.status,
                cancel_requested: false,
            });
        }

        job.cancel_token.store(true, Ordering::SeqCst);
        Ok(CancelJobResponse {
            job_id: job.id.to_string(),
            status: job.status,
            cancel_requested: true,
        })
    }

    pub fn is_cancelled(&self, job_id: Uuid) -> bool {
        let Ok(guard) = self.current.lock() else {
            return true;
        };

        guard
            .as_ref()
            .filter(|job| job.id == job_id)
            .is_some_and(|job| job.cancel_token.load(Ordering::SeqCst))
    }
}

fn apply_terminal_failure(job: &mut JobRecord, error: ErrorPayload) -> TerminalTransition {
    let stage = match error.code {
        "JOB_CANCELLED" => JobProgressStage::Cancelled,
        _ => JobProgressStage::Failed,
    };

    job.status = match error.code {
        "JOB_CANCELLED" => JobStatus::Cancelled,
        _ => JobStatus::Failed,
    };
    job.progress = JobProgress::new(job.id, stage).with_error_code(error.code);
    job.error = Some(error);
    job.result = None;

    TerminalTransition {
        status: job.status,
        progress: job.progress.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::types::job_timeout;

    fn sample_result() -> JobResult {
        JobResult {
            markdown: "# Test".to_string(),
            title: "Test".to_string(),
            bvid: "BVTEST".to_string(),
            language: "zh-CN".to_string(),
            segment_count: 1,
        }
    }

    #[test]
    fn start_rejects_second_active_job() {
        let manager = JobManager::new();
        let first = manager.try_start().expect("first start");
        match manager.try_start() {
            Err(err) => assert_eq!(err.code, "JOB_ALREADY_RUNNING"),
            Ok(_) => panic!("expected JOB_ALREADY_RUNNING"),
        }
        manager
            .complete(first.job_id, sample_result())
            .expect("complete");
    }

    #[test]
    fn new_job_replaces_terminal_job() {
        let manager = JobManager::new();
        let first = manager.try_start().expect("first");
        manager
            .fail(first.job_id, job_timeout())
            .expect("fail first");

        let second = manager.try_start().expect("second start");
        assert_ne!(first.job_id, second.job_id);

        let status = manager
            .get_status(&second.job_id.to_string())
            .expect("status");
        assert_eq!(status.status, JobStatus::Queued);
    }

    #[test]
    fn cancel_is_idempotent_and_terminal_jobs_unchanged() {
        let manager = JobManager::new();
        let handle = manager.try_start().expect("start");
        manager
            .complete(handle.job_id, sample_result())
            .expect("complete");

        let cancelled = manager
            .cancel(&handle.job_id.to_string())
            .expect("cancel completed");
        assert_eq!(cancelled.status, JobStatus::Completed);
        assert!(!cancelled.cancel_requested);

        let status = manager
            .get_status(&handle.job_id.to_string())
            .expect("status");
        assert_eq!(status.status, JobStatus::Completed);
    }

    #[test]
    fn cancel_sets_token_and_reports_requested_for_active_job() {
        let manager = JobManager::new();
        let handle = manager.try_start().expect("start");
        manager.mark_running(handle.job_id).expect("running");

        let response = manager.cancel(&handle.job_id.to_string()).expect("cancel");
        assert_eq!(response.status, JobStatus::Running);
        assert!(response.cancel_requested);
        assert!(manager.is_cancelled(handle.job_id));
    }

    #[test]
    fn result_ready_not_ready_not_found() {
        let manager = JobManager::new();
        let missing = manager.get_result("00000000-0000-0000-0000-000000000099");
        assert_eq!(missing.unwrap_err().code, "JOB_NOT_FOUND");

        let handle = manager.try_start().expect("start");
        let not_ready = manager.get_result(&handle.job_id.to_string());
        assert_eq!(not_ready.unwrap_err().code, "JOB_NOT_READY");

        manager
            .complete(handle.job_id, sample_result())
            .expect("complete");
        let result = manager
            .get_result(&handle.job_id.to_string())
            .expect("result");
        assert_eq!(result.title, "Test");
    }

    #[test]
    fn get_result_failed_returns_stored_error() {
        let manager = JobManager::new();
        let handle = manager.try_start().expect("start");
        manager.mark_running(handle.job_id).expect("running");
        manager.fail(handle.job_id, job_timeout()).expect("fail");

        let err = manager.get_result(&handle.job_id.to_string()).unwrap_err();
        assert_eq!(err.code, "JOB_TIMEOUT");
    }

    #[test]
    fn get_result_cancelled_returns_stored_error() {
        let manager = JobManager::new();
        let handle = manager.try_start().expect("start");
        manager
            .fail(handle.job_id, job_cancelled())
            .expect("cancel");

        let err = manager.get_result(&handle.job_id.to_string()).unwrap_err();
        assert_eq!(err.code, "JOB_CANCELLED");
    }

    #[test]
    fn complete_visible_immediately_before_emit() {
        let manager = JobManager::new();
        let handle = manager.try_start().expect("start");
        manager.mark_running(handle.job_id).expect("running");

        let transition = manager
            .complete(handle.job_id, sample_result())
            .expect("complete");
        assert_eq!(transition.status, JobStatus::Completed);
        assert_eq!(transition.progress.stage, JobProgressStage::Done);

        let result = manager
            .get_result(&handle.job_id.to_string())
            .expect("result ready");
        assert_eq!(result.title, "Test");
    }

    #[test]
    fn complete_loses_race_to_cancel_token() {
        let manager = JobManager::new();
        let handle = manager.try_start().expect("start");
        manager.mark_running(handle.job_id).expect("running");
        handle.cancel_token.store(true, Ordering::SeqCst);

        let transition = manager
            .complete(handle.job_id, sample_result())
            .expect("complete");
        assert_eq!(transition.status, JobStatus::Cancelled);
        assert_eq!(transition.progress.stage, JobProgressStage::Cancelled);
        assert_eq!(
            transition.progress.error_code.as_deref(),
            Some("JOB_CANCELLED")
        );

        let err = manager.get_result(&handle.job_id.to_string()).unwrap_err();
        assert_eq!(err.code, "JOB_CANCELLED");
    }

    #[test]
    fn fail_stores_before_emit_would_run() {
        let manager = JobManager::new();
        let handle = manager.try_start().expect("start");
        manager.mark_running(handle.job_id).expect("running");

        let transition = manager.fail(handle.job_id, job_timeout()).expect("fail");
        assert_eq!(transition.status, JobStatus::Failed);
        assert_eq!(
            transition.progress.error_code.as_deref(),
            Some("JOB_TIMEOUT")
        );

        let status = manager
            .get_status(&handle.job_id.to_string())
            .expect("status");
        assert_eq!(status.status, JobStatus::Failed);
        assert_eq!(status.error.as_ref().map(|e| e.code), Some("JOB_TIMEOUT"));
    }

    #[test]
    fn terminal_state_not_overwritten_by_late_updates() {
        let manager = JobManager::new();
        let handle = manager.try_start().expect("start");
        manager
            .complete(handle.job_id, sample_result())
            .expect("complete");

        manager
            .fail(handle.job_id, job_timeout())
            .expect("late fail");
        let status = manager
            .get_status(&handle.job_id.to_string())
            .expect("status");
        assert_eq!(status.status, JobStatus::Completed);
    }

    #[test]
    fn invalid_job_id_rejected() {
        let manager = JobManager::new();
        let err = manager.get_status("not-a-uuid").unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn internal_error_uses_internal_error_code() {
        let err = internal_error("test");
        assert_eq!(err.code, "INTERNAL_ERROR");
    }

    #[test]
    fn progress_payload_serializes_without_secrets() {
        let progress = JobProgress::new(Uuid::new_v4(), JobProgressStage::Failed)
            .with_error_code("JOB_TIMEOUT");
        let json = serde_json::to_string(&progress).expect("serialize");
        assert!(json.contains("JOB_TIMEOUT"));
        assert!(json.contains("error_code"));
        assert!(!json.contains("message"));
        assert!(!json.to_lowercase().contains("api_key"));
        assert!(!json.to_lowercase().contains("cookie"));
        assert!(!json.to_lowercase().contains("authorization"));
    }
}
