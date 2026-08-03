use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::auth::load_auth;
use crate::bilibili::BilibiliSubtitleResult;
use crate::error::ErrorPayload;
use crate::llm::{HttpTransport, LlmClient, LlmClientConfig, ReqwestTransport};
use crate::media;
use crate::note::{
    generate_note_data, render_markdown, write_markdown_to_dir, NoteLocale, NoteProgress,
    NoteProgressStage, VideoMetadata,
};
use crate::paths::StoragePaths;
use crate::settings::{load_settings, Locale as SettingsLocale};

use super::manager::JobManager;
use super::types::{
    internal_error, job_cancelled, job_timeout, JobProgress, JobProgressStage, JobResult,
    JOB_DEADLINE,
};

/// Run one or more note jobs serially under a single active `job_id`.
///
/// Per-item failures are recorded and the queue continues. Cancel stops the
/// current item and skips remaining pending URLs. Each item uses its own
/// [`JOB_DEADLINE`] wall clock.
pub fn run_note_jobs(
    app: AppHandle,
    manager: Arc<JobManager>,
    paths: StoragePaths,
    job_id: Uuid,
    urls: Vec<String>,
    cancel_token: Arc<AtomicBool>,
) {
    if urls.is_empty() {
        finalize_failure(
            &app,
            &manager,
            job_id,
            ErrorPayload {
                code: "VALIDATION_ERROR",
                message: "urls is required".to_string(),
            },
        );
        return;
    }

    if let Err(err) = manager.mark_running(job_id) {
        finalize_failure(&app, &manager, job_id, err);
        return;
    }

    let fetch = |url: &str, cookie: Option<&str>| media::fetch_subtitles(url, cookie);
    let total = urls.len();
    let mut success_count = 0usize;
    let mut last_error: Option<ErrorPayload> = None;
    let mut cancelled = false;

    for (index, url) in urls.iter().enumerate() {
        if cancel_token.load(Ordering::SeqCst) {
            cancelled = true;
            let _ = manager.skip_pending_items(job_id);
            break;
        }

        let _ = manager.mark_item_running(job_id, index);
        let started_at = Instant::now();

        match run_note_job_inner(
            &paths,
            job_id,
            url.as_str(),
            &cancel_token,
            started_at,
            &fetch,
            |progress| {
                let progress = progress.with_batch(index, total, url);
                let _ = manager.update_progress(job_id, progress.clone());
                // Emit mid-job progress so the UI need not wait for the 2s poll fallback.
                emit_progress(&app, &progress);
            },
        ) {
            Ok(job_result) => {
                success_count += 1;
                let _ = manager.record_item_success(job_id, index, job_result);
            }
            Err(err) if err.code == "JOB_CANCELLED" => {
                cancelled = true;
                let _ = manager.record_item_failure(job_id, index, err);
                let _ = manager.skip_pending_items(job_id);
                break;
            }
            Err(err) => {
                last_error = Some(err.clone());
                let _ = manager.record_item_failure(job_id, index, err);
            }
        }
    }

    if cancelled {
        finalize_failure(&app, &manager, job_id, job_cancelled());
        return;
    }

    if success_count == 0 {
        finalize_failure(
            &app,
            &manager,
            job_id,
            last_error.unwrap_or_else(|| {
                internal_error("Note generation failed for all URLs in the batch")
            }),
        );
        return;
    }

    finalize_batch_success(&app, &manager, job_id);
}

/// Backward-compatible single-URL entry point.
pub fn run_note_job(
    app: AppHandle,
    manager: Arc<JobManager>,
    paths: StoragePaths,
    job_id: Uuid,
    url: String,
    cancel_token: Arc<AtomicBool>,
    _started_at: Instant,
) {
    run_note_jobs(app, manager, paths, job_id, vec![url], cancel_token);
}

pub fn handle_job_task_join_error(app: AppHandle, manager: Arc<JobManager>, job_id: Uuid) {
    finalize_failure(
        &app,
        &manager,
        job_id,
        internal_error("Note generation task failed unexpectedly"),
    );
}

fn finalize_batch_success(app: &AppHandle, manager: &JobManager, job_id: Uuid) {
    match manager.complete_from_stored(job_id) {
        Ok(transition) => emit_progress(app, &transition.progress),
        Err(err) => finalize_failure(app, manager, job_id, err),
    }
}

fn finalize_failure(app: &AppHandle, manager: &JobManager, job_id: Uuid, error: ErrorPayload) {
    if let Ok(transition) = manager.fail(job_id, error) {
        emit_progress(app, &transition.progress);
    }
}

fn emit_progress(app: &AppHandle, progress: &JobProgress) {
    let _ = app.emit("job://progress", progress);
}

pub(crate) fn run_note_job_inner<F, P>(
    paths: &StoragePaths,
    job_id: Uuid,
    url: &str,
    cancel_token: &AtomicBool,
    started_at: Instant,
    fetch_subtitles_fn: &F,
    on_progress: P,
) -> Result<JobResult, ErrorPayload>
where
    F: Fn(&str, Option<&str>) -> Result<BilibiliSubtitleResult, ErrorPayload>,
    P: FnMut(JobProgress),
{
    let check_cancelled = || cancel_token.load(Ordering::SeqCst);
    let check_deadline = || started_at.elapsed() > JOB_DEADLINE;

    if check_cancelled() {
        return Err(job_cancelled());
    }
    if check_deadline() {
        return Err(job_timeout());
    }

    let settings = load_settings(paths).map_err(ErrorPayload::from)?;
    let locale = resolve_note_locale(&settings.locale);
    let notes_save_dir = settings.notes_save_dir.clone();

    let auth = load_auth(paths).map_err(ErrorPayload::from)?;
    let api_key = auth
        .as_ref()
        .map(|auth| auth.api_key.as_str())
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| ErrorPayload::from(crate::error::Error::validation("api_key is required")))?
        .to_string();
    let bilibili_cookie = auth
        .and_then(|auth| auth.bilibili_cookie)
        .filter(|cookie| !cookie.trim().is_empty());

    let mut on_progress = on_progress;
    on_progress(JobProgress::new(
        job_id,
        JobProgressStage::FetchingSubtitles,
    ));

    let subtitle_result = fetch_subtitles_fn(url, bilibili_cookie.as_deref())?;

    if check_cancelled() {
        return Err(job_cancelled());
    }
    if check_deadline() {
        return Err(job_timeout());
    }

    let transport = ReqwestTransport::with_defaults().map_err(ErrorPayload::from)?;
    let client = LlmClient::new(
        LlmClientConfig {
            base_url: settings.base_url,
            api_key,
            model: settings.model,
        },
        transport,
    )
    .map_err(ErrorPayload::from)?;

    let mut result = run_pipeline_with_client(
        &client,
        subtitle_result,
        locale,
        check_cancelled,
        check_deadline,
        job_id,
        on_progress,
    )?;

    if let Some(dir) = notes_save_dir.as_deref() {
        match write_markdown_to_dir(Path::new(dir), &result.title, &result.bvid, &result.markdown) {
            Ok(path) => {
                result.saved_path = Some(path.to_string_lossy().into_owned());
            }
            Err(err) => {
                // Auto-save must not fail the completed generation.
                eprintln!("OpenNote auto-save failed: {}", err.message());
            }
        }
    }

    Ok(result)
}

pub(crate) fn run_pipeline_with_client<T, P>(
    client: &LlmClient<T>,
    subtitle_result: BilibiliSubtitleResult,
    locale: NoteLocale,
    mut check_cancelled: impl FnMut() -> bool,
    mut check_deadline: impl FnMut() -> bool,
    job_id: Uuid,
    mut on_progress: P,
) -> Result<JobResult, ErrorPayload>
where
    T: HttpTransport,
    P: FnMut(JobProgress),
{
    if check_cancelled() {
        return Err(job_cancelled());
    }
    if check_deadline() {
        return Err(job_timeout());
    }

    let metadata = video_metadata_from_subtitle(&subtitle_result);
    let segment_count = subtitle_result.segments.len();
    let language = subtitle_result.language.clone();

    let note_data = generate_note_data(
        client,
        metadata.clone(),
        subtitle_result.segments,
        locale,
        || check_cancelled() || check_deadline(),
        |note_progress| {
            let job_progress = map_note_progress(job_id, note_progress);
            on_progress(job_progress);
        },
    )
    .map_err(|err| {
        map_interrupted_generation_error(err, &mut check_cancelled, &mut check_deadline)
    })?;

    if check_cancelled() {
        return Err(job_cancelled());
    }
    if check_deadline() {
        return Err(job_timeout());
    }

    on_progress(JobProgress::new(job_id, JobProgressStage::Rendering));

    let markdown = render_markdown(&note_data, locale);

    if check_cancelled() {
        return Err(job_cancelled());
    }
    if check_deadline() {
        return Err(job_timeout());
    }

    Ok(JobResult {
        markdown,
        title: metadata.title,
        bvid: metadata.bvid,
        language,
        segment_count,
        saved_path: None,
        batch_results: Vec::new(),
    })
}

pub fn resolve_note_locale(locale: &SettingsLocale) -> NoteLocale {
    match locale {
        SettingsLocale::Zh => NoteLocale::Zh,
        SettingsLocale::En => NoteLocale::En,
        SettingsLocale::System => {
            if sys_locale::get_locale()
                .is_some_and(|value| value.to_ascii_lowercase().starts_with("zh"))
            {
                NoteLocale::Zh
            } else {
                NoteLocale::En
            }
        }
    }
}

fn video_metadata_from_subtitle(result: &BilibiliSubtitleResult) -> VideoMetadata {
    let duration_ms = if result.duration_ms > 0 {
        result.duration_ms
    } else {
        result
            .segments
            .iter()
            .map(|segment| segment.end_ms)
            .max()
            .unwrap_or(0)
    };

    VideoMetadata {
        title: result.title.clone(),
        part_title: result.part_title.clone(),
        bvid: result.bvid.clone(),
        duration_ms,
    }
}

fn map_note_progress(job_id: Uuid, progress: NoteProgress) -> JobProgress {
    let stage = match progress.stage {
        NoteProgressStage::Chunking => JobProgressStage::Chunking,
        NoteProgressStage::AnalyzingChunk => JobProgressStage::CallingLlm,
        NoteProgressStage::MergingSummaries => JobProgressStage::Merging,
        NoteProgressStage::Complete => JobProgressStage::Rendering,
    };

    JobProgress::new(job_id, stage).with_chunks(progress.chunk_index, progress.chunk_count)
}

fn map_interrupted_generation_error(
    err: crate::note::NoteError,
    check_cancelled: &mut impl FnMut() -> bool,
    check_deadline: &mut impl FnMut() -> bool,
) -> ErrorPayload {
    if err.code() == "JOB_CANCELLED" {
        if check_cancelled() {
            return job_cancelled();
        }
        if check_deadline() {
            return job_timeout();
        }
        return job_cancelled();
    }
    ErrorPayload::from(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bilibili::SubtitleSegment;
    use crate::llm::{MockResponse, MockTransport};
    use crate::paths::StoragePaths;
    use crate::settings::{save_settings, Settings};
    use std::time::Instant;

    fn success_chat_body(content: &str) -> String {
        serde_json::json!({
            "choices": [{ "message": { "content": content } }]
        })
        .to_string()
    }

    fn setup_paths() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn save_test_settings(paths: &StoragePaths) {
        let settings = Settings {
            version: 1,
            base_url: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            locale: SettingsLocale::En,
            onboarding_completed: true,
            notes_save_dir: None,
        };
        save_settings(paths, &settings).expect("save settings");
    }

    fn save_test_auth(paths: &StoragePaths) {
        use crate::auth::{save_auth, SaveAuthInput};
        save_auth(
            paths,
            SaveAuthInput {
                api_key: Some("sk-test".to_string()),
                bilibili_cookie: None,
                clear_bilibili_cookie: false,
                clear_api_key: false,
            },
        )
        .expect("save auth");
    }

    fn mock_subtitle_result() -> BilibiliSubtitleResult {
        BilibiliSubtitleResult {
            title: "Demo".to_string(),
            part_title: None,
            bvid: "BVTEST".to_string(),
            aid: 1,
            cid: 1,
            p: 1,
            page_count: 1,
            duration_ms: 0,
            language: "zh-CN".to_string(),
            segments: vec![SubtitleSegment {
                start_ms: 1000,
                end_ms: 2000,
                text: "hello".to_string(),
            }],
        }
    }

    #[test]
    fn resolve_locale_mapping() {
        assert_eq!(resolve_note_locale(&SettingsLocale::Zh), NoteLocale::Zh);
        assert_eq!(resolve_note_locale(&SettingsLocale::En), NoteLocale::En);
    }

    #[test]
    fn orchestration_with_mock_fetch_and_llm() {
        let dir = setup_paths();
        let paths = StoragePaths::from_dirs(dir.path().join("auth"), dir.path());
        save_test_settings(&paths);
        save_test_auth(&paths);

        let chunk_json = r#"{"summary":"overall","sections":[{"start_ms":1000,"end_ms":2000,"title":"Intro","explanation":"Says hi"}]}"#;
        let (transport, _) =
            MockTransport::new(vec![MockResponse::success(success_chat_body(chunk_json))]);
        let client = LlmClient::new(
            LlmClientConfig {
                base_url: "https://api.example.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                model: "test-model".to_string(),
            },
            transport,
        )
        .expect("client");

        let subtitle = mock_subtitle_result();
        let metadata = video_metadata_from_subtitle(&subtitle);
        assert_eq!(metadata.duration_ms, 2000);

        let job_id = Uuid::new_v4();
        let cancel = AtomicBool::new(false);
        let result = run_pipeline_with_client(
            &client,
            subtitle,
            NoteLocale::En,
            || cancel.load(Ordering::SeqCst),
            || false,
            job_id,
            |_| {},
        )
        .expect("pipeline");

        assert!(result.markdown.contains("overall"));
        assert_eq!(result.segment_count, 1);
        assert_eq!(result.bvid, "BVTEST");
    }

    #[test]
    fn cancelled_after_render_discards_result() {
        let dir = setup_paths();
        let paths = StoragePaths::from_dirs(dir.path().join("auth"), dir.path());
        save_test_settings(&paths);
        save_test_auth(&paths);

        let chunk_json = r#"{"summary":"overall","sections":[{"start_ms":1000,"end_ms":2000,"title":"Intro","explanation":"Says hi"}]}"#;
        let (transport, _) =
            MockTransport::new(vec![MockResponse::success(success_chat_body(chunk_json))]);
        let client = LlmClient::new(
            LlmClientConfig {
                base_url: "https://api.example.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                model: "test-model".to_string(),
            },
            transport,
        )
        .expect("client");

        let cancel = AtomicBool::new(false);
        let err = run_pipeline_with_client(
            &client,
            mock_subtitle_result(),
            NoteLocale::En,
            || cancel.load(Ordering::SeqCst),
            || false,
            Uuid::new_v4(),
            |progress| {
                if progress.stage == JobProgressStage::Rendering {
                    cancel.store(true, Ordering::SeqCst);
                }
            },
        )
        .unwrap_err();

        assert_eq!(err.code, "JOB_CANCELLED");
    }

    #[test]
    fn cancelled_after_fetch_discards_result() {
        let dir = setup_paths();
        let paths = StoragePaths::from_dirs(dir.path().join("auth"), dir.path());
        save_test_settings(&paths);
        save_test_auth(&paths);

        let cancel = AtomicBool::new(true);
        let fetch = |_: &str, _: Option<&str>| Ok(mock_subtitle_result());

        let err = run_note_job_inner(
            &paths,
            Uuid::new_v4(),
            "https://www.bilibili.com/video/BVTEST",
            &cancel,
            Instant::now(),
            &fetch,
            |_| {},
        )
        .unwrap_err();

        assert_eq!(err.code, "JOB_CANCELLED");
    }

    #[test]
    fn deadline_maps_to_job_timeout() {
        let dir = setup_paths();
        let paths = StoragePaths::from_dirs(dir.path().join("auth"), dir.path());
        save_test_settings(&paths);
        save_test_auth(&paths);

        let cancel = AtomicBool::new(false);
        let started = Instant::now() - JOB_DEADLINE - std::time::Duration::from_secs(1);
        let fetch = |_: &str, _: Option<&str>| Ok(mock_subtitle_result());

        let err = run_note_job_inner(
            &paths,
            Uuid::new_v4(),
            "https://www.bilibili.com/video/BVTEST",
            &cancel,
            started,
            &fetch,
            |_| {},
        )
        .unwrap_err();

        assert_eq!(err.code, "JOB_TIMEOUT");
    }
}
