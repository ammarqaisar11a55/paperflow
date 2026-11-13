//! Per-job temporary folders.
//!
//! Each job works inside its own private folder under the application's cache
//! directory. The folder is deleted when the job ends, whatever the outcome,
//! and stale folders from a crash are removed on the next start.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppResult;

const JOBS_DIR: &str = "jobs";

/// A temporary folder removed on drop.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn create(root: &Path, job_id: &str) -> AppResult<Self> {
        let path = root.join(JOBS_DIR).join(job_id);
        fs::create_dir_all(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A path inside the folder.
    pub fn join(&self, name: impl AsRef<Path>) -> PathBuf {
        self.path.join(name)
    }

    /// Creates and returns a new, empty subfolder whose name starts with `name`.
    pub fn subdir(&self, name: &str) -> AppResult<PathBuf> {
        let p = crate::filesystem::paths::unique_path(&self.path, name);
        fs::create_dir_all(&p)?;
        Ok(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Removes every job folder. Called at start-up (no job can be running) and
/// from Settings › Privacy (only when no job is running).
pub fn clear_all(root: &Path) -> AppResult<u64> {
    let dir = root.join(JOBS_DIR);
    if !dir.exists() {
        return Ok(0);
    }
    let freed = dir_size(&dir);
    fs::remove_dir_all(&dir)?;
    Ok(freed)
}

/// Total size in bytes of the temporary job folders.
pub fn usage(root: &Path) -> u64 {
    dir_size(&root.join(JOBS_DIR))
}

fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(_) => e.metadata().map(|m| m.len()).unwrap_or(0),
            Err(_) => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_on_drop() {
        let root = tempfile::tempdir().unwrap();
        let p;
        {
            let t = TempDir::create(root.path(), "job1").unwrap();
            p = t.path().to_path_buf();
            fs::write(t.join("x"), b"abc").unwrap();
            assert!(p.exists());
            assert_eq!(usage(root.path()), 3);
        }
        assert!(!p.exists());
    }

    #[test]
    fn clear_all_removes_everything() {
        let root = tempfile::tempdir().unwrap();
        let t = TempDir::create(root.path(), "job2").unwrap();
        fs::write(t.join("x"), b"abcd").unwrap();
        std::mem::forget(t);
        assert_eq!(clear_all(root.path()).unwrap(), 4);
        assert_eq!(usage(root.path()), 0);
        assert_eq!(clear_all(root.path()).unwrap(), 0);
    }
}
