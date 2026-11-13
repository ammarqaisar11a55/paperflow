//! PaperFlow: an offline PDF toolkit. Every document is processed on this
//! computer; nothing is uploaded and no network access is needed.

pub mod commands;
pub mod error;
pub mod filesystem;
pub mod pdf;
pub mod services;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{Manager, Runtime};

use crate::services::jobs::JobManager;
use crate::services::store::Store;

/// State shared by all commands.
pub struct AppState {
    pub jobs: JobManager,
    pub store: Arc<Store>,
    pub temp_root: PathBuf,
}

/// How many jobs run at the same time. Engines such as Ghostscript and
/// Tesseract are heavy, so two keeps the desktop responsive.
const MAX_CONCURRENT_JOBS: usize = 2;

/// Where job folders live: a per-user folder under the system temporary
/// directory. Engines' security profiles (AppArmor on Ubuntu) allow /tmp but
/// not hidden folders such as ~/.cache, and inside the Snap /tmp is private
/// to PaperFlow.
pub fn job_temp_root() -> PathBuf {
    #[cfg(unix)]
    let user = {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata("/proc/self").map(|m| m.uid().to_string()).unwrap_or_else(|_| "user".into())
    };
    #[cfg(not(unix))]
    let user = "user".to_string();
    std::env::temp_dir().join(format!("paperflow-{user}"))
}

/// Registers PaperFlow's plugins and commands on a builder. Shared by the
/// application and the IPC tests, which run it on Tauri's mock runtime.
pub fn with_commands<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_opener::init()).invoke_handler(
        tauri::generate_handler![
            commands::files::inspect_files,
            commands::files::read_file_bytes,
            commands::files::read_metadata,
            commands::files::suggest_output_dir,
            commands::jobs::start_job,
            commands::jobs::cancel_job,
            commands::jobs::list_jobs,
            commands::jobs::clear_finished_jobs,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_activity,
            commands::settings::remove_activity,
            commands::settings::clear_activity,
            commands::system::engine_status,
            commands::system::ocr_languages,
            commands::system::open_path,
            commands::system::reveal_path,
            commands::system::temp_usage,
            commands::system::clear_temp,
            commands::system::app_info,
        ],
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    with_commands(tauri::Builder::default())
        .setup(|app| {
            let paths = app.path();
            let cache = job_temp_root();
            let config = paths.app_config_dir()?;
            let data = paths.app_data_dir()?;
            // Leftovers from an interrupted session: no job can be running yet.
            let _ = filesystem::temp::clear_all(&cache);
            app.manage(AppState {
                jobs: JobManager::new(cache.clone(), MAX_CONCURRENT_JOBS),
                store: Arc::new(Store::open(&config, &data)),
                temp_root: cache,
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running PaperFlow");
}

#[cfg(test)]
mod ipc_tests;
