use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

use crate::error::ErrorPayload;

use super::types::{
    internal_error, job_already_running, job_cancelled, job_not_found, job_not_ready, parse_job_id,
    BatchItemResult, BatchItemStatus, BatchItemView, CancelJobResponse, JobProgress,
    JobProgressStage, JobResult, JobStatus, JobStatusResponse, TerminalTransition,
};

struct BatchItemRecord {
    index: usize,
    url: String,
    status: BatchItemStatus,
    result: Option<JobResult>,
    error: Option<ErrorPayload>,
}

impl BatchItemRecord {
    fn to_view(&self) -> BatchItemView {
        BatchItemView {
            index: self.index,
            url: self.url.clone(),
            status: self.status,
            title: self.result.as_ref().map(|r| r.title.clone()),
            error_code: self.error.as_ref().map(|e| e.code.to_string()),
            saved_path: self.result.as_ref().and_then(|r| r.saved_path.clone()),
        }
    }
}

struct JobRecord {
    id: Uuid,
    status: JobStatus,
    progress: JobProgress,
    result: Option<JobResult>,
    error: Option<ErrorPayload>,
    cancel_token: Arc<AtomicBool>,
    batch_items: Vec<BatchItemRecord>,
}

#[derive(Debug)]
pub struct StartHandle {
    pub job_id: Uuid,
    pub cancel_token: Arc<AtomicBool>,
    pub started_at: Instant,
    pub urls: Vec<String>,
}

#[derive(Default)]
pub struct JobManager {
    current: Mutex<Option<JobRecord>>,
}

impl JobManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_start(&self, urls: Vec<String>) -> std::result::Result<StartHandle, ErrorPayload> {
        if urls.is_empty() {
            return Err(ErrorPayload {
                code: "VALIDATION_ERROR",
                message: "urls is required".to_string(),
            });
        }

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
        let progress = JobProgress::new(job_id, JobProgressStage::FetchingSubtitles).with_batch(
            0,
            urls.len(),
            &urls[0],
        );

        let batch_items = urls
            .iter()
            .enumerate()
            .map(|(index, url)| BatchItemRecord {
                index,
                url: url.clone(),
                status: BatchItemStatus::Pending,
                result: None,
                error: None,
            })
            .collect();

        *guard = Some(JobRecord {
            id: job_id,
            status: JobStatus::Queued,
            progress,
            result: None,
            error: None,
            cancel_token: Arc::clone(&cancel_token),
            batch_items,
        });

        Ok(StartHandle {
            job_id,
            cancel_token,
            started_at,
            urls,
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

    pub fn mark_item_running(&self, job_id: Uuid, index: usize) -> Result<(), ErrorPayload> {
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

        if let Some(item) = job.batch_items.get_mut(index) {
            item.status = BatchItemStatus::Running;
            item.error = None;
        }

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

    /// Store a successful item outcome without terminalizing the batch job.
    pub fn record_item_success(
        &self,
        job_id: Uuid,
        index: usize,
        result: JobResult,
    ) -> Result<(), ErrorPayload> {
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

        if let Some(item) = job.batch_items.get_mut(index) {
            item.status = BatchItemStatus::Completed;
            item.result = Some(result.clone());
            item.error = None;
        }
        // Primary result for get_job_result / export: last successful note.
        job.result = Some(result);
        job.error = None;
        Ok(())
    }

    /// Store a failed/cancelled item outcome without terminalizing (unless caller does later).
    pub fn record_item_failure(
        &self,
        job_id: Uuid,
        index: usize,
        error: ErrorPayload,
    ) -> Result<(), ErrorPayload> {
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

        if let Some(item) = job.batch_items.get_mut(index) {
            item.status = if error.code == "JOB_CANCELLED" {
                BatchItemStatus::Cancelled
            } else {
                BatchItemStatus::Failed
            };
            item.error = Some(error);
            item.result = None;
        }
        Ok(())
    }

    /// Mark remaining pending items as skipped after cancel.
    pub fn skip_pending_items(&self, job_id: Uuid) -> Result<(), ErrorPayload> {
        let mut guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        let job = guard
            .as_mut()
            .filter(|job| job.id == job_id)
            .ok_or_else(job_not_found)?;

        for item in &mut job.batch_items {
            if item.status == BatchItemStatus::Pending {
                item.status = BatchItemStatus::Skipped;
            }
        }
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
        job.progress = terminal_progress(job, JobProgressStage::Done, None);
        job.result = Some(result);
        job.error = None;

        Ok(TerminalTransition {
            status: job.status,
            progress: job.progress.clone(),
        })
    }

    /// Complete using the last stored successful result (batch path).
    pub fn complete_from_stored(&self, job_id: Uuid) -> Result<TerminalTransition, ErrorPayload> {
        let result = {
            let guard = self
                .current
                .lock()
                .map_err(|_| internal_error("Job manager lock poisoned"))?;
            let job = guard
                .as_ref()
                .filter(|job| job.id == job_id)
                .ok_or_else(job_not_found)?;
            job.result
                .clone()
                .ok_or_else(|| internal_error("Batch job completed with no successful item"))?
        };
        self.complete(job_id, result)
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
            batch_items: job.batch_items.iter().map(BatchItemRecord::to_view).collect(),
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
                .map(|result| attach_batch_results(result, job))
                .ok_or_else(|| internal_error("Completed job is missing result")),
            // Mid-batch: expose already-finished notes so the UI can switch/preview
            // without waiting for the whole queue (still JOB_NOT_READY until first success).
            JobStatus::Queued | JobStatus::Running => match job.result.clone() {
                Some(result) => Ok(attach_batch_results(result, job)),
                None => Err(job_not_ready()),
            },
            // Partial batch success may still have a primary note after fail/cancel.
            JobStatus::Failed | JobStatus::Cancelled => match job.result.clone() {
                Some(result) => Ok(attach_batch_results(result, job)),
                None => Err(job.error.clone().unwrap_or_else(|| {
                    if job.status == JobStatus::Cancelled {
                        job_cancelled()
                    } else {
                        internal_error("Failed job is missing error")
                    }
                })),
            },
        }
    }

    /// Resolve a single successful batch item for export/copy by 0-based index.
    pub fn get_item_result(
        &self,
        job_id: &str,
        item_index: usize,
    ) -> Result<JobResult, ErrorPayload> {
        let job_id = parse_job_id(job_id)?;
        let guard = self
            .current
            .lock()
            .map_err(|_| internal_error("Job manager lock poisoned"))?;

        let job = guard
            .as_ref()
            .filter(|job| job.id == job_id)
            .ok_or_else(job_not_found)?;

        let item = job
            .batch_items
            .iter()
            .find(|item| item.index == item_index)
            .ok_or_else(|| {
                ErrorPayload {
                    code: "VALIDATION_ERROR",
                    message: format!("No batch item at index {item_index}"),
                }
            })?;

        item.result.clone().ok_or_else(|| ErrorPayload {
            code: "JOB_NOT_READY",
            message: format!("Batch item {item_index} has no successful result yet"),
        })
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

fn collect_batch_results(job: &JobRecord) -> Vec<BatchItemResult> {
    // Single-URL jobs keep the historical payload shape (no batch_results array).
    if job.batch_items.len() <= 1 {
        return Vec::new();
    }

    job.batch_items
        .iter()
        .filter_map(|item| {
            let result = item.result.as_ref()?;
            Some(BatchItemResult {
                index: item.index,
                url: item.url.clone(),
                markdown: result.markdown.clone(),
                title: result.title.clone(),
                bvid: result.bvid.clone(),
                language: result.language.clone(),
                segment_count: result.segment_count,
                saved_path: result.saved_path.clone(),
            })
        })
        .collect()
}

fn attach_batch_results(mut result: JobResult, job: &JobRecord) -> JobResult {
    result.batch_results = collect_batch_results(job);
    result
}

fn terminal_progress(
    job: &JobRecord,
    stage: JobProgressStage,
    error_code: Option<&'static str>,
) -> JobProgress {
    let mut progress = JobProgress::new(job.id, stage);
    if let Some(code) = error_code {
        progress = progress.with_error_code(code);
    }

    let total = job.batch_items.len();
    if total == 0 {
        return progress;
    }

    let (index, url) = job
        .batch_items
        .iter()
        .rev()
        .find(|item| {
            matches!(
                item.status,
                BatchItemStatus::Running
                    | BatchItemStatus::Completed
                    | BatchItemStatus::Failed
                    | BatchItemStatus::Cancelled
            )
        })
        .map(|item| (item.index, item.url.as_str()))
        .unwrap_or((0, job.batch_items[0].url.as_str()));

    progress.with_batch(index, total, url)
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
    job.progress = terminal_progress(job, stage, Some(error.code));
    job.error = Some(error);
    // Keep any successful item results for UI/export of partial batch on
    // cancel or failure (primary remains the last successful note).

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
            saved_path: None,
            batch_results: Vec::new(),
        }
    }

    fn sample_result_named(title: &str, markdown: &str) -> JobResult {
        JobResult {
            markdown: markdown.to_string(),
            title: title.to_string(),
            bvid: "BVTEST".to_string(),
            language: "zh-CN".to_string(),
            segment_count: 1,
            saved_path: None,
            batch_results: Vec::new(),
        }
    }

    fn sample_urls() -> Vec<String> {
        vec!["https://www.bilibili.com/video/BVTEST".to_string()]
    }

    #[test]
    fn start_rejects_second_active_job() {
        let manager = JobManager::new();
        let first = manager.try_start(sample_urls()).expect("first start");
        match manager.try_start(sample_urls()) {
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
        let first = manager.try_start(sample_urls()).expect("first");
        manager
            .fail(first.job_id, job_timeout())
            .expect("fail first");

        let second = manager.try_start(sample_urls()).expect("second start");
        assert_ne!(first.job_id, second.job_id);

        let status = manager
            .get_status(&second.job_id.to_string())
            .expect("status");
        assert_eq!(status.status, JobStatus::Queued);
    }

    #[test]
    fn cancel_is_idempotent_and_terminal_jobs_unchanged() {
        let manager = JobManager::new();
        let handle = manager.try_start(sample_urls()).expect("start");
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
        let handle = manager.try_start(sample_urls()).expect("start");
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

        let handle = manager.try_start(sample_urls()).expect("start");
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
        let handle = manager.try_start(sample_urls()).expect("start");
        manager.mark_running(handle.job_id).expect("running");
        manager.fail(handle.job_id, job_timeout()).expect("fail");

        let err = manager.get_result(&handle.job_id.to_string()).unwrap_err();
        assert_eq!(err.code, "JOB_TIMEOUT");
    }

    #[test]
    fn get_result_cancelled_returns_stored_error() {
        let manager = JobManager::new();
        let handle = manager.try_start(sample_urls()).expect("start");
        manager
            .fail(handle.job_id, job_cancelled())
            .expect("cancel");

        let err = manager.get_result(&handle.job_id.to_string()).unwrap_err();
        assert_eq!(err.code, "JOB_CANCELLED");
    }

    #[test]
    fn complete_visible_immediately_before_emit() {
        let manager = JobManager::new();
        let handle = manager.try_start(sample_urls()).expect("start");
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
        let handle = manager.try_start(sample_urls()).expect("start");
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
        let handle = manager.try_start(sample_urls()).expect("start");
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
        let handle = manager.try_start(sample_urls()).expect("start");
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
            .with_error_code("JOB_TIMEOUT")
            .with_batch(0, 2, "https://example.com/a");
        let json = serde_json::to_string(&progress).expect("serialize");
        assert!(json.contains("JOB_TIMEOUT"));
        assert!(json.contains("error_code"));
        assert!(json.contains("item_index"));
        assert!(!json.contains("message"));
        assert!(!json.to_lowercase().contains("api_key"));
        assert!(!json.to_lowercase().contains("cookie"));
        assert!(!json.to_lowercase().contains("authorization"));
    }

    #[test]
    fn batch_continues_after_item_failure_then_completes() {
        let manager = JobManager::new();
        let urls = vec![
            "https://example.com/a".to_string(),
            "https://example.com/b".to_string(),
        ];
        let handle = manager.try_start(urls).expect("start");
        manager.mark_running(handle.job_id).expect("running");

        manager
            .mark_item_running(handle.job_id, 0)
            .expect("item 0");
        manager
            .record_item_failure(handle.job_id, 0, job_timeout())
            .expect("fail item 0");

        manager
            .mark_item_running(handle.job_id, 1)
            .expect("item 1");
        manager
            .record_item_success(handle.job_id, 1, sample_result())
            .expect("ok item 1");

        let transition = manager
            .complete_from_stored(handle.job_id)
            .expect("complete batch");
        assert_eq!(transition.status, JobStatus::Completed);

        let status = manager
            .get_status(&handle.job_id.to_string())
            .expect("status");
        assert_eq!(status.batch_items.len(), 2);
        assert_eq!(status.batch_items[0].status, BatchItemStatus::Failed);
        assert_eq!(status.batch_items[1].status, BatchItemStatus::Completed);
        assert_eq!(status.batch_items[0].error_code.as_deref(), Some("JOB_TIMEOUT"));
    }

    #[test]
    fn batch_get_result_includes_all_successful_notes() {
        let manager = JobManager::new();
        let urls = vec![
            "https://example.com/a".to_string(),
            "https://example.com/b".to_string(),
            "https://example.com/c".to_string(),
        ];
        let handle = manager.try_start(urls).expect("start");
        manager.mark_running(handle.job_id).expect("running");

        manager
            .record_item_success(handle.job_id, 0, sample_result_named("A", "# A"))
            .expect("ok 0");
        manager
            .record_item_failure(handle.job_id, 1, job_timeout())
            .expect("fail 1");
        manager
            .record_item_success(handle.job_id, 2, sample_result_named("C", "# C"))
            .expect("ok 2");

        // Mid-batch / pre-complete: partial successes are readable.
        let partial = manager
            .get_result(&handle.job_id.to_string())
            .expect("partial");
        assert_eq!(partial.title, "C");
        assert_eq!(partial.batch_results.len(), 2);
        assert_eq!(partial.batch_results[0].title, "A");
        assert_eq!(partial.batch_results[0].url, "https://example.com/a");
        assert_eq!(partial.batch_results[1].markdown, "# C");

        manager
            .complete_from_stored(handle.job_id)
            .expect("complete");
        let done = manager
            .get_result(&handle.job_id.to_string())
            .expect("done");
        assert_eq!(done.batch_results.len(), 2);
        assert_eq!(done.markdown, "# C");

        let item0 = manager
            .get_item_result(&handle.job_id.to_string(), 0)
            .expect("item 0");
        assert_eq!(item0.title, "A");
        assert!(item0.batch_results.is_empty());
    }

    #[test]
    fn single_url_get_result_omits_batch_results() {
        let manager = JobManager::new();
        let handle = manager.try_start(sample_urls()).expect("start");
        manager
            .complete(handle.job_id, sample_result())
            .expect("complete");
        let result = manager
            .get_result(&handle.job_id.to_string())
            .expect("result");
        assert!(result.batch_results.is_empty());
    }

    #[test]
    fn batch_cancel_skips_pending_items() {
        let manager = JobManager::new();
        let urls = vec![
            "https://example.com/a".to_string(),
            "https://example.com/b".to_string(),
            "https://example.com/c".to_string(),
        ];
        let handle = manager.try_start(urls).expect("start");
        manager.mark_running(handle.job_id).expect("running");
        manager
            .mark_item_running(handle.job_id, 0)
            .expect("item 0");
        manager
            .record_item_success(handle.job_id, 0, sample_result())
            .expect("ok");

        manager.skip_pending_items(handle.job_id).expect("skip");
        manager
            .fail(handle.job_id, job_cancelled())
            .expect("cancel");

        let status = manager
            .get_status(&handle.job_id.to_string())
            .expect("status");
        assert_eq!(status.status, JobStatus::Cancelled);
        assert_eq!(status.batch_items[0].status, BatchItemStatus::Completed);
        assert_eq!(status.batch_items[1].status, BatchItemStatus::Skipped);
        assert_eq!(status.batch_items[2].status, BatchItemStatus::Skipped);
        // Partial success retained for preview after cancel.
        let result = manager
            .get_result(&handle.job_id.to_string())
            .expect("partial result");
        assert_eq!(result.title, "Test");
    }

    #[test]
    fn batch_fail_keeps_partial_successful_notes() {
        let manager = JobManager::new();
        let urls = vec![
            "https://example.com/a".to_string(),
            "https://example.com/b".to_string(),
            "https://example.com/c".to_string(),
        ];
        let handle = manager.try_start(urls).expect("start");
        manager.mark_running(handle.job_id).expect("running");
        manager
            .record_item_success(handle.job_id, 0, sample_result_named("A", "# A"))
            .expect("ok 0");
        manager
            .record_item_failure(handle.job_id, 1, job_timeout())
            .expect("fail 1");

        manager
            .fail(handle.job_id, internal_error("Note generation task failed unexpectedly"))
            .expect("terminal fail");

        let status = manager
            .get_status(&handle.job_id.to_string())
            .expect("status");
        assert_eq!(status.status, JobStatus::Failed);
        assert_eq!(status.batch_items[0].status, BatchItemStatus::Completed);
        assert_eq!(status.error.as_ref().map(|e| e.code), Some("INTERNAL_ERROR"));

        // Partial success retained for preview after Failed (same as cancel).
        let result = manager
            .get_result(&handle.job_id.to_string())
            .expect("partial result");
        assert_eq!(result.title, "A");
        assert_eq!(result.batch_results.len(), 1);
        assert_eq!(result.batch_results[0].markdown, "# A");
    }
}
