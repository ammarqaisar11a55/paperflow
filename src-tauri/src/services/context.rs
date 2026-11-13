//! What an operation receives while it runs: a cancellation token, a progress
//! reporter and a private temporary folder. Operations know nothing about
//! Tauri, which keeps them testable in plain Rust.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::filesystem::temp::TempDir;

#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// Returns `Err(Cancelled)` once cancellation has been requested.
    pub fn check(&self) -> AppResult<()> {
        if self.is_cancelled() {
            Err(AppError::cancelled())
        } else {
            Ok(())
        }
    }
}

/// A progress update. `fraction` is `None` while the amount of work is unknown.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressUpdate {
    pub fraction: Option<f64>,
    pub message: String,
    pub current_file: Option<String>,
}

pub type ProgressSink = Arc<dyn Fn(ProgressUpdate) + Send + Sync>;

pub struct JobContext {
    pub cancel: CancelToken,
    sink: ProgressSink,
    pub temp: TempDir,
}

impl JobContext {
    pub fn new(cancel: CancelToken, sink: ProgressSink, temp: TempDir) -> Self {
        Self { cancel, sink, temp }
    }

    /// A context that discards progress, for tests.
    pub fn for_tests(root: &Path) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self::new(CancelToken::new(), Arc::new(|_| {}), TempDir::create(root, &id).expect("temp dir"))
    }

    pub fn progress(&self, fraction: f64, message: impl Into<String>) {
        (self.sink)(ProgressUpdate { fraction: Some(fraction.clamp(0.0, 1.0)), message: message.into(), current_file: None });
    }

    pub fn progress_file(&self, fraction: f64, message: impl Into<String>, file: impl Into<String>) {
        (self.sink)(ProgressUpdate {
            fraction: Some(fraction.clamp(0.0, 1.0)),
            message: message.into(),
            current_file: Some(file.into()),
        });
    }

    pub fn status(&self, message: impl Into<String>) {
        (self.sink)(ProgressUpdate { fraction: None, message: message.into(), current_file: None });
    }

    pub fn check(&self) -> AppResult<()> {
        self.cancel.check()
    }
}
