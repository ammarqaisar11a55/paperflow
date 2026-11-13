//! File inspection and reading for the interface.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::{display_name, validate_image_input, validate_pdf_input, IMAGE_EXTENSIONS};
use crate::pdf::ops::metadata::{self, Metadata};

/// Largest file the viewer will load into memory.
const MAX_READ_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    /// "pdf", "image" or "unsupported".
    pub kind: &'static str,
    pub page_count: Option<u32>,
    pub encrypted: bool,
    pub error: Option<String>,
}

fn is_image_path(path: &Path) -> bool {
    path.extension()
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn inspect(path: &Path) -> FileInfo {
    let name = display_name(path);
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut info = FileInfo {
        path: path.display().to_string(),
        name,
        size,
        kind: "unsupported",
        page_count: None,
        encrypted: false,
        error: None,
    };
    if is_image_path(path) {
        info.kind = "image";
        if let Err(e) = validate_image_input(path) {
            info.error = Some(e.message);
        }
        return info;
    }
    match validate_pdf_input(path) {
        Ok(_) => {
            info.kind = "pdf";
            match lopdf::Document::load_metadata(path) {
                Ok(meta) => {
                    info.encrypted = meta.encrypted;
                    info.page_count = Some(meta.page_count);
                }
                Err(lopdf::Error::InvalidPassword) | Err(lopdf::Error::Decryption(_)) => {
                    info.encrypted = true
                }
                Err(e) => info.error = Some(AppError::from(e).message),
            }
        }
        Err(e) => info.error = Some(e.message),
    }
    info
}

#[tauri::command]
pub async fn inspect_files(paths: Vec<String>) -> Vec<FileInfo> {
    tauri::async_runtime::spawn_blocking(move || paths.iter().map(|p| inspect(Path::new(p))).collect())
        .await
        .unwrap_or_default()
}

/// Raw bytes of a PDF or image, for rendering previews in the webview.
#[tauri::command]
pub async fn read_file_bytes(path: String) -> AppResult<tauri::ipc::Response> {
    let p = PathBuf::from(&path);
    if !p.is_absolute() {
        return Err(AppError::invalid("Invalid file path."));
    }
    if is_image_path(&p) {
        validate_image_input(&p)?;
    } else {
        validate_pdf_input(&p)?;
    }
    if fs::metadata(&p)?.len() > MAX_READ_BYTES {
        return Err(AppError::invalid("This file is too large to preview."));
    }
    let bytes = tauri::async_runtime::spawn_blocking(move || fs::read(p))
        .await
        .map_err(|e| AppError::processing(e.to_string()))??;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub async fn read_metadata(path: String) -> AppResult<Metadata> {
    tauri::async_runtime::spawn_blocking(move || metadata::read(Path::new(&path)))
        .await
        .map_err(|e| AppError::processing(e.to_string()))?
}

/// The user's real Documents folder, also inside the Snap sandbox (where
/// `$HOME` points at the snap's private folder).
pub fn documents_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    if let Some(real) = std::env::var_os("SNAP_REAL_HOME") {
        let real = PathBuf::from(real);
        let docs = real.join("Documents");
        return if docs.is_dir() { docs } else { real };
    }
    app.path()
        .document_dir()
        .ok()
        .filter(|d| d.is_dir())
        .or_else(|| app.path().home_dir().ok())
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn is_writable_dir(dir: &Path) -> bool {
    if !dir.is_dir() || dir.starts_with("/run/user") {
        // /run/user/<uid>/doc/... is the document portal: one file, no siblings.
        return false;
    }
    let probe = dir.join(format!(".paperflow-{}", uuid::Uuid::new_v4().simple()));
    match fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Where results go by default: next to the input when that folder is
/// writable, otherwise Documents/PaperFlow.
#[tauri::command]
pub fn suggest_output_dir<R: Runtime>(app: AppHandle<R>, input: Option<String>) -> AppResult<String> {
    if let Some(parent) = input.as_deref().map(Path::new).and_then(Path::parent) {
        if is_writable_dir(parent) {
            return Ok(parent.display().to_string());
        }
    }
    let fallback = documents_dir(&app).join("PaperFlow");
    fs::create_dir_all(&fallback)?;
    Ok(fallback.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspects_pdfs_images_and_other_files() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("a.pdf");
        crate::pdf::assemble::tests::sample_doc(4, "A").save(&pdf).unwrap();
        let info = inspect(&pdf);
        assert_eq!((info.kind, info.page_count, info.encrypted), ("pdf", Some(4), false));

        let png = dir.path().join("b.png");
        image::RgbImage::new(2, 2).save(&png).unwrap();
        assert_eq!(inspect(&png).kind, "image");

        let txt = dir.path().join("c.txt");
        fs::write(&txt, b"hello").unwrap();
        let info = inspect(&txt);
        assert_eq!(info.kind, "unsupported");
        assert!(info.error.is_some());
    }

    #[test]
    fn detects_writable_dirs() {
        let dir = tempfile::tempdir().unwrap();
        assert!(is_writable_dir(dir.path()));
        assert!(!is_writable_dir(&dir.path().join("missing")));
        assert!(!is_writable_dir(Path::new("/run/user/1000/doc/abc")));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0, "probe file removed");
    }
}
