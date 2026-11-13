//! PDF operations. Each operation reads its inputs, writes its results into
//! the job's temporary folder and returns them as a [`Staged`] result; the job
//! runner then moves them to the user's chosen location. Nothing is written
//! next to the originals until an operation has fully succeeded.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{AppError, AppResult};

pub mod compress;
pub mod convert;
pub mod metadata;
pub mod ocr;
pub mod organize;
pub mod security;
pub mod stamp;

/// The results of an operation, still in the temporary folder.
#[derive(Debug, Default)]
pub struct Staged {
    /// (temporary path, final file name)
    pub files: Vec<(PathBuf, String)>,
    /// When set, the files go into a new folder with this name.
    pub folder: Option<String>,
    pub warnings: Vec<String>,
    /// Page count of the (single) PDF output, when known.
    pub page_count: Option<usize>,
}

impl Staged {
    pub fn single(path: PathBuf, name: String) -> Self {
        Self { files: vec![(path, name)], ..Default::default() }
    }

    pub fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }
}

/// Parses an operation's options, with a friendly error.
pub fn parse_options<T: for<'de> Deserialize<'de>>(value: &serde_json::Value) -> AppResult<T> {
    let value = if value.is_null() { serde_json::Value::Object(Default::default()) } else { value.clone() };
    serde_json::from_value(value)
        .map_err(|e| AppError::invalid("The tool settings are not valid.").with_detail(e.to_string()))
}

pub fn require_one(inputs: &[PathBuf]) -> AppResult<&Path> {
    match inputs {
        [one] => Ok(one),
        [] => Err(AppError::invalid("Add a file first.")),
        _ => Err(AppError::invalid("This tool works on one file at a time.")),
    }
}
#[cfg(test)]
mod engine_tests;
