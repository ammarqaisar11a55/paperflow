//! Settings and recent activity, persisted as small JSON files in the
//! application's config and data folders.
//!
//! Activity records only file names, the tool used, the outcome and where the
//! result was saved. Document contents and passwords are never stored.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::filesystem::paths::OverwritePolicy;
use crate::services::jobs::{JobSnapshot, JobStatus};
use crate::services::operations::OperationType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum OutputLocation {
    /// Next to the first input file (falls back to Documents/PaperFlow).
    #[default]
    SameAsSource,
    /// Always the folder in `output_dir`.
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: Theme,
    pub output_location: OutputLocation,
    pub output_dir: Option<String>,
    pub compression_preset: String,
    pub image_dpi: u32,
    pub image_format: String,
    pub overwrite: OverwritePolicy,
    pub ocr_language: String,
    pub favorites: Vec<String>,
    pub sidebar_collapsed: bool,
    pub record_activity: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            output_location: OutputLocation::SameAsSource,
            output_dir: None,
            compression_preset: "recommended".into(),
            image_dpi: 150,
            image_format: "png".into(),
            overwrite: OverwritePolicy::Rename,
            ocr_language: "eng".into(),
            favorites: Vec::new(),
            sidebar_collapsed: false,
            record_activity: true,
        }
    }
}

impl Settings {
    /// Clamps values that came from the interface into sensible ranges.
    pub fn sanitized(mut self) -> Self {
        self.image_dpi = self.image_dpi.clamp(36, 1200);
        if !["extreme", "recommended", "light"].contains(&self.compression_preset.as_str()) {
            self.compression_preset = "recommended".into();
        }
        if !["png", "jpeg"].contains(&self.image_format.as_str()) {
            self.image_format = "png".into();
        }
        self.favorites.truncate(64);
        self.favorites.dedup();
        if self.output_dir.as_deref().is_some_and(|d| d.trim().is_empty()) {
            self.output_dir = None;
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub op_type: OperationType,
    pub title: String,
    pub file_names: Vec<String>,
    /// "completed", "failed" or "cancelled".
    pub status: String,
    /// Milliseconds since the Unix epoch.
    pub timestamp: i64,
    pub output_files: Vec<String>,
    pub output_folder: Option<String>,
    pub error: Option<String>,
}

impl ActivityEntry {
    pub fn from_job(job: &JobSnapshot) -> Self {
        let status = match job.status {
            JobStatus::Completed => "completed",
            JobStatus::Cancelled => "cancelled",
            _ => "failed",
        };
        Self {
            id: job.id.clone(),
            op_type: job.op_type,
            title: job.title.clone(),
            file_names: job.input_names.clone(),
            status: status.into(),
            timestamp: job.finished_at.unwrap_or(job.created_at),
            output_files: job.result.as_ref().map(|r| r.output_files.clone()).unwrap_or_default(),
            output_folder: job.result.as_ref().and_then(|r| r.output_folder.clone()),
            error: job.error.as_ref().map(|e| e.message.clone()),
        }
    }
}

const MAX_ACTIVITY: usize = 200;

pub struct Store {
    settings_path: PathBuf,
    activity_path: PathBuf,
    settings: Mutex<Settings>,
    activity: Mutex<Vec<ActivityEntry>>,
}

fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Writes via a temporary file and rename, so a crash never leaves a
/// half-written settings file.
fn write_json<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(value).map_err(|e| AppError::output("Settings could not be saved.").with_detail(e.to_string()))?;
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

impl Store {
    pub fn open(config_dir: &Path, data_dir: &Path) -> Self {
        let settings_path = config_dir.join("settings.json");
        let activity_path = data_dir.join("activity.json");
        let settings = read_json::<Settings>(&settings_path).sanitized();
        let activity = read_json::<Vec<ActivityEntry>>(&activity_path);
        Self { settings_path, activity_path, settings: Mutex::new(settings), activity: Mutex::new(activity) }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn save_settings(&self, settings: Settings) -> AppResult<Settings> {
        let settings = settings.sanitized();
        write_json(&self.settings_path, &settings)?;
        *self.settings.lock().unwrap() = settings.clone();
        Ok(settings)
    }

    pub fn activity(&self) -> Vec<ActivityEntry> {
        self.activity.lock().unwrap().clone()
    }

    pub fn record(&self, entry: ActivityEntry) -> AppResult<()> {
        if !self.settings().record_activity {
            return Ok(());
        }
        let mut list = self.activity.lock().unwrap();
        list.retain(|e| e.id != entry.id);
        list.insert(0, entry);
        list.truncate(MAX_ACTIVITY);
        write_json(&self.activity_path, &*list)
    }

    pub fn remove_activity(&self, id: &str) -> AppResult<()> {
        let mut list = self.activity.lock().unwrap();
        list.retain(|e| e.id != id);
        write_json(&self.activity_path, &*list)
    }

    pub fn clear_activity(&self) -> AppResult<()> {
        let mut list = self.activity.lock().unwrap();
        list.clear();
        write_json(&self.activity_path, &*list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> ActivityEntry {
        ActivityEntry {
            id: id.into(),
            op_type: OperationType::Merge,
            title: "Merge PDF".into(),
            file_names: vec!["a.pdf".into()],
            status: "completed".into(),
            timestamp: 1,
            output_files: vec![],
            output_folder: None,
            error: None,
        }
    }

    #[test]
    fn settings_persist_across_restarts() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path(), dir.path());
        assert_eq!(store.settings(), Settings::default());
        let s = Settings { theme: Theme::Dark, image_dpi: 5000, compression_preset: "bogus".into(), ..Default::default() };
        let saved = store.save_settings(s).unwrap();
        assert_eq!(saved.image_dpi, 1200);
        assert_eq!(saved.compression_preset, "recommended");
        let reopened = Store::open(dir.path(), dir.path());
        assert_eq!(reopened.settings().theme, Theme::Dark);
    }

    #[test]
    fn tolerates_corrupt_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("settings.json"), b"{ not json").unwrap();
        fs::write(dir.path().join("activity.json"), b"[1,2").unwrap();
        let store = Store::open(dir.path(), dir.path());
        assert_eq!(store.settings(), Settings::default());
        assert!(store.activity().is_empty());
    }

    #[test]
    fn activity_is_recorded_newest_first_and_clearable() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path(), dir.path());
        store.record(entry("1")).unwrap();
        store.record(entry("2")).unwrap();
        assert_eq!(store.activity().iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), vec!["2", "1"]);
        store.remove_activity("2").unwrap();
        assert_eq!(Store::open(dir.path(), dir.path()).activity().len(), 1);
        store.clear_activity().unwrap();
        assert!(Store::open(dir.path(), dir.path()).activity().is_empty());
    }

    #[test]
    fn activity_can_be_turned_off() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path(), dir.path());
        store.save_settings(Settings { record_activity: false, ..Default::default() }).unwrap();
        store.record(entry("1")).unwrap();
        assert!(store.activity().is_empty());
    }
}
