//! Engines, OCR languages, opening results, temporary files and app info.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::filesystem::temp;
use crate::pdf::ops::ocr::installed_languages;
use crate::services::context::CancelToken;
use crate::services::engines::{self, Engine};
use crate::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub engine: Engine,
    pub label: &'static str,
    pub binary: &'static str,
    pub available: bool,
    pub version: Option<String>,
}

#[tauri::command]
pub async fn engine_status() -> Vec<EngineInfo> {
    tauri::async_runtime::spawn_blocking(|| {
        Engine::ALL
            .iter()
            .map(|&e| {
                let available = engines::locate(e).is_some();
                EngineInfo {
                    engine: e,
                    label: e.label(),
                    binary: e.binary(),
                    available,
                    version: if available { engines::version(e) } else { None },
                }
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
pub async fn ocr_languages() -> AppResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(|| installed_languages(&CancelToken::new()))
        .await
        .map_err(|e| AppError::processing(e.to_string()))?
}

fn existing(path: &str) -> AppResult<PathBuf> {
    let p = PathBuf::from(path);
    if !p.is_absolute() || !p.exists() {
        return Err(
            AppError::invalid("That file no longer exists.").with_hint("It may have been moved or deleted.")
        );
    }
    Ok(p)
}

/// Opens a result with the default application.
#[tauri::command]
pub fn open_path<R: Runtime>(app: AppHandle<R>, path: String) -> AppResult<()> {
    let p = existing(&path)?;
    app.opener()
        .open_path(p.display().to_string(), None::<&str>)
        .map_err(|e| AppError::processing("The file could not be opened.").with_detail(e.to_string()))
}

/// Shows a result in the file manager.
#[tauri::command]
pub fn reveal_path<R: Runtime>(app: AppHandle<R>, path: String) -> AppResult<()> {
    let p = existing(&path)?;
    if app.opener().reveal_item_in_dir(&p).is_ok() {
        return Ok(());
    }
    // Some sandboxes allow neither the file manager's D-Bus service nor the
    // portal: open the containing folder instead.
    let folder =
        if p.is_dir() { p.clone() } else { p.parent().map(|d| d.to_path_buf()).unwrap_or(p.clone()) };
    app.opener()
        .open_path(folder.display().to_string(), None::<&str>)
        .map_err(|e| AppError::processing("The folder could not be opened.").with_detail(e.to_string()))
}

#[tauri::command]
pub fn temp_usage(state: State<'_, AppState>) -> u64 {
    temp::usage(&state.temp_root)
}

#[tauri::command]
pub fn clear_temp(state: State<'_, AppState>) -> AppResult<u64> {
    if state.jobs.has_active() {
        return Err(AppError::invalid("Temporary files are in use.")
            .with_hint("Wait for the running jobs to finish."));
    }
    temp::clear_all(&state.temp_root)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub identifier: String,
    pub repository: &'static str,
    pub license: &'static str,
    pub sandboxed: bool,
    pub tessdata: Option<String>,
}

#[tauri::command]
pub fn app_info<R: Runtime>(app: AppHandle<R>) -> AppInfo {
    let pkg = app.package_info();
    AppInfo {
        name: pkg.name.clone(),
        version: pkg.version.to_string(),
        identifier: app.config().identifier.clone(),
        repository: env!("CARGO_PKG_REPOSITORY"),
        license: env!("CARGO_PKG_LICENSE"),
        sandboxed: std::env::var_os("SNAP").is_some(),
        tessdata: engines::tessdata_dir().map(|p| p.display().to_string()),
    }
}
