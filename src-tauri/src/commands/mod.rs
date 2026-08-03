use std::fmt;
use std::sync::Arc;

use crate::auth::{
    auth_status, load_auth, normalize_optional_secret, save_auth as persist_auth, validate_api_key,
    AuthStatus, SaveAuthInput,
};
use crate::bilibili::{fetch_subtitles, BilibiliSubtitleResult};
use crate::error::{Error, ErrorPayload};
use crate::job::{
    handle_job_task_join_error, run_note_job, CancelJobResponse, JobManager, JobResult,
    JobStatusResponse, StartJobResponse,
};
use crate::llm::{LlmClient, LlmClientConfig, ReqwestTransport};
use crate::note::sanitize_markdown_filename;
use crate::paths::StoragePaths;
use crate::redact::REDACTED;
use crate::settings::{
    load_settings, save_settings as persist_settings, validate_base_url, validate_model,
    validate_settings_input, SaveSettingsInput, SettingsView,
};
use serde::{Deserialize, Serialize};
use tauri_plugin_dialog::DialogExt;

type CommandResult<T> = std::result::Result<T, ErrorPayload>;

pub struct AppState {
    pub paths: StoragePaths,
    pub jobs: Arc<JobManager>,
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> CommandResult<SettingsView> {
    load_settings(&state.paths)
        .map(SettingsView::from)
        .map_err(ErrorPayload::from)
}

#[tauri::command]
pub fn save_settings(
    state: tauri::State<'_, AppState>,
    input: SaveSettingsInput,
) -> CommandResult<SettingsView> {
    let settings = validate_settings_input(input).map_err(ErrorPayload::from)?;
    persist_settings(&state.paths, &settings).map_err(ErrorPayload::from)?;
    Ok(SettingsView::from(settings))
}

#[tauri::command]
pub async fn pick_notes_save_dir(app: tauri::AppHandle) -> CommandResult<Option<String>> {
    let app_for_dialog = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = app_for_dialog.dialog().file().blocking_pick_folder();
        match path {
            Some(path) => {
                let file_path = path.into_path().map_err(|err| {
                    ErrorPayload::from(Error::validation(format!("Invalid folder path: {err}")))
                })?;
                Ok(Some(file_path.to_string_lossy().into_owned()))
            }
            None => Ok(None),
        }
    })
    .await
    .map_err(|_| ErrorPayload {
        code: "INTERNAL_ERROR",
        message: "Folder picker failed to complete".to_string(),
    })?
}

#[tauri::command]
pub fn get_auth_status(state: tauri::State<'_, AppState>) -> CommandResult<AuthStatus> {
    auth_status(&state.paths).map_err(ErrorPayload::from)
}

#[tauri::command]
pub fn save_auth(
    state: tauri::State<'_, AppState>,
    input: SaveAuthInput,
) -> CommandResult<AuthStatus> {
    persist_auth(&state.paths, input).map_err(ErrorPayload::from)
}

#[tauri::command]
pub async fn fetch_bilibili_subtitles(
    state: tauri::State<'_, AppState>,
    url: String,
) -> CommandResult<BilibiliSubtitleResult> {
    let paths = state.paths.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let cookie = load_auth(&paths)
            .map_err(ErrorPayload::from)?
            .and_then(|auth| auth.bilibili_cookie);
        fetch_subtitles(&url, cookie.as_deref()).map_err(ErrorPayload::from)
    })
    .await
    .map_err(|_| ErrorPayload {
        code: "NETWORK_ERROR",
        message: "Failed to complete subtitle fetch".to_string(),
    })?
}

#[derive(Debug, Serialize)]
pub struct TestLlmSuccess {
    pub ok: bool,
    pub model: String,
}

#[derive(Clone, Deserialize)]
pub struct TestLlmInput {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl fmt::Debug for TestLlmInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestLlmInput")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_ref().map(|_| REDACTED))
            .finish()
    }
}

#[tauri::command]
pub async fn test_llm(
    state: tauri::State<'_, AppState>,
    input: TestLlmInput,
) -> CommandResult<TestLlmSuccess> {
    let paths = state.paths.clone();

    tauri::async_runtime::spawn_blocking(move || run_test_llm(&paths, input))
        .await
        .map_err(|_| ErrorPayload {
            code: "NETWORK_ERROR",
            message: "Failed to complete LLM connectivity test".to_string(),
        })?
}

fn resolve_test_api_key(paths: &StoragePaths, input: &TestLlmInput) -> CommandResult<String> {
    if let Some(key) = normalize_optional_secret(input.api_key.clone()) {
        return validate_api_key(&key).map_err(ErrorPayload::from);
    }

    load_auth(paths)
        .map_err(ErrorPayload::from)?
        .map(|auth| auth.api_key)
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| ErrorPayload::from(Error::validation("api_key is required")))
}

fn run_test_llm(paths: &StoragePaths, input: TestLlmInput) -> CommandResult<TestLlmSuccess> {
    let base_url = validate_base_url(&input.base_url).map_err(ErrorPayload::from)?;
    let model = validate_model(&input.model).map_err(ErrorPayload::from)?;
    let api_key = resolve_test_api_key(paths, &input)?;

    let transport = ReqwestTransport::with_defaults().map_err(ErrorPayload::from)?;
    let client = LlmClient::new(
        LlmClientConfig {
            base_url,
            api_key,
            model: model.clone(),
        },
        transport,
    )
    .map_err(ErrorPayload::from)?;

    client.test_connection().map_err(ErrorPayload::from)?;

    Ok(TestLlmSuccess { ok: true, model })
}

#[tauri::command]
pub async fn start_note_job(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
) -> CommandResult<StartJobResponse> {
    if url.trim().is_empty() {
        return Err(ErrorPayload::from(Error::validation("url is required")));
    }
    let url = url.trim().to_string();

    let handle = state.jobs.try_start()?;
    let job_id = handle.job_id;
    let paths = state.paths.clone();
    let jobs = Arc::clone(&state.jobs);
    let response = StartJobResponse {
        job_id: job_id.to_string(),
    };

    tauri::async_runtime::spawn(async move {
        let app_for_task = app.clone();
        let jobs_for_error = Arc::clone(&jobs);
        let join_result = tauri::async_runtime::spawn_blocking(move || {
            run_note_job(
                app_for_task,
                jobs,
                paths,
                job_id,
                url,
                handle.cancel_token,
                handle.started_at,
            );
        })
        .await;

        if join_result.is_err() {
            handle_job_task_join_error(app, jobs_for_error, job_id);
        }
    });

    Ok(response)
}

#[tauri::command]
pub fn get_job_status(
    state: tauri::State<'_, AppState>,
    job_id: String,
) -> CommandResult<JobStatusResponse> {
    state.jobs.get_status(&job_id)
}

#[tauri::command]
pub fn get_job_result(
    state: tauri::State<'_, AppState>,
    job_id: String,
) -> CommandResult<JobResult> {
    state.jobs.get_result(&job_id)
}

#[tauri::command]
pub fn cancel_job(
    state: tauri::State<'_, AppState>,
    job_id: String,
) -> CommandResult<CancelJobResponse> {
    state.jobs.cancel(&job_id)
}

#[derive(Debug, Serialize)]
pub struct ExportJobMarkdownResponse {
    pub saved: bool,
}

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
    }
}

#[tauri::command]
pub async fn export_job_markdown(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    job_id: String,
) -> CommandResult<ExportJobMarkdownResponse> {
    let result = state.jobs.get_result(&job_id)?;
    let default_name = sanitize_markdown_filename(&result.title);
    let markdown = result.markdown;

    // `blocking_save_file` is synchronous and must not run on the async runtime.
    // Run dialog + disk write on a blocking thread pool instead.
    let app_for_dialog = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = app_for_dialog
            .dialog()
            .file()
            .set_file_name(&default_name)
            .add_filter("Markdown", &["md"])
            .blocking_save_file();

        match path {
            Some(path) => {
                let file_path = path.into_path().map_err(|err| {
                    ErrorPayload::from(Error::validation(format!("Invalid save path: {err}")))
                })?;
                std::fs::write(&file_path, markdown.as_bytes()).map_err(|err| {
                    ErrorPayload::from(Error::storage(format!(
                        "Failed to write markdown file: {err}"
                    )))
                })?;
                Ok(ExportJobMarkdownResponse { saved: true })
            }
            None => Ok(ExportJobMarkdownResponse { saved: false }),
        }
    })
    .await
    .map_err(|_| ErrorPayload {
        code: "INTERNAL_ERROR",
        message: "Export task failed to complete".to_string(),
    })?
}

#[cfg(test)]
mod test_llm_tests {
    use super::{resolve_test_api_key, TestLlmInput};
    use crate::auth::{save_auth, SaveAuthInput};
    use crate::paths::StoragePaths;

    #[test]
    fn test_llm_input_debug_redacts_api_key() {
        let input = TestLlmInput {
            base_url: "https://api.example.com/v1".to_string(),
            model: "deepseek-v4-flash".to_string(),
            api_key: Some("sk-secret".to_string()),
        };
        let debug = format!("{input:?}");
        assert!(!debug.contains("sk-secret"));
        assert!(debug.contains(crate::redact::REDACTED));
    }

    #[test]
    fn resolve_test_api_key_uses_input_when_provided() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path(), dir.path());

        let key = resolve_test_api_key(
            &paths,
            &TestLlmInput {
                base_url: "https://api.example.com/v1".to_string(),
                model: "deepseek-v4-flash".to_string(),
                api_key: Some("sk-from-form".to_string()),
            },
        )
        .expect("key");

        assert_eq!(key, "sk-from-form");
    }

    #[test]
    fn resolve_test_api_key_falls_back_to_saved_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path(), dir.path());

        save_auth(
            &paths,
            SaveAuthInput {
                api_key: Some("sk-saved".to_string()),
                bilibili_cookie: None,
                clear_bilibili_cookie: false,
                clear_api_key: false,
            },
        )
        .expect("save");

        let key = resolve_test_api_key(
            &paths,
            &TestLlmInput {
                base_url: "https://api.example.com/v1".to_string(),
                model: "deepseek-v4-flash".to_string(),
                api_key: None,
            },
        )
        .expect("key");

        assert_eq!(key, "sk-saved");
    }

    #[test]
    fn resolve_test_api_key_requires_key_when_missing_everywhere() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = StoragePaths::from_dirs(dir.path(), dir.path());

        let err = resolve_test_api_key(
            &paths,
            &TestLlmInput {
                base_url: "https://api.example.com/v1".to_string(),
                model: "deepseek-v4-flash".to_string(),
                api_key: None,
            },
        )
        .unwrap_err();

        assert_eq!(err.code, "VALIDATION_ERROR");
    }
}
