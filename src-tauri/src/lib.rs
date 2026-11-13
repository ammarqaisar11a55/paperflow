//! PaperFlow: an offline PDF toolkit. Every document is processed on this
//! computer; nothing is uploaded and no network access is needed.

pub mod commands;
pub mod error;
pub mod filesystem;
pub mod pdf;
pub mod services;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = app.path();
            let cache = paths.app_cache_dir()?;
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
        .invoke_handler(tauri::generate_handler![
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running PaperFlow");
}
