mod auth;
mod bilibili;
mod commands;
mod error;
pub mod job;
pub mod llm;
mod note;
mod paths;
mod redact;
mod settings;

pub use note::{
    generate_note_data, render_markdown, NoteData, NoteLocale, NoteProgress, NoteProgressStage,
    VideoMetadata,
};

use std::sync::Arc;

use commands::AppState;
use job::JobManager;
use paths::StoragePaths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = StoragePaths::from_env().unwrap_or_else(|err| {
        eprintln!(
            "OpenNote failed to resolve configuration directories: {}",
            err.message()
        );
        std::process::exit(1);
    });

    let jobs = Arc::new(JobManager::new());

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .manage(AppState { paths, jobs })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_auth_status,
            commands::save_auth,
            commands::fetch_bilibili_subtitles,
            commands::test_llm,
            commands::get_app_info,
            commands::start_note_job,
            commands::get_job_status,
            commands::get_job_result,
            commands::cancel_job,
            commands::export_job_markdown,
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
