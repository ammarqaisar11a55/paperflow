//! Errors that cross the IPC boundary.
//!
//! Every error carries a plain-language `message` for the interface, an optional
//! `hint` that suggests what to do next, and an optional technical `detail`
//! that the interface shows only on request. Document contents and passwords
//! never appear in any of them.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// The input is unusable (missing, not a PDF, damaged, wrong type).
    InvalidInput,
    /// The PDF is password-protected and must be unlocked first.
    Encrypted,
    /// A password was supplied but is not correct.
    WrongPassword,
    /// A processing engine (Ghostscript, qpdf, ...) is not installed.
    MissingEngine,
    /// The output could not be written.
    Output,
    /// The user cancelled the job.
    Cancelled,
    /// Anything else.
    Processing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
    pub hint: Option<String>,
    pub detail: Option<String>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into(), hint: None, detail: None }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidInput, message)
    }

    pub fn processing(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Processing, message)
    }

    pub fn output(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Output, message)
    }

    pub fn cancelled() -> Self {
        Self::new(ErrorKind::Cancelled, "The job was cancelled.")
    }

    pub fn encrypted(file_name: &str) -> Self {
        Self::new(ErrorKind::Encrypted, format!("“{file_name}” is password-protected."))
            .with_hint("Open it with Unlock PDF and your password first, then try again.")
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn is_cancelled(&self) -> bool {
        self.kind == ErrorKind::Cancelled
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(detail) = &self.detail {
            write!(f, " ({detail})")?;
        }
        Ok(())
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind as Io;
        let (kind, message, hint) = match err.kind() {
            Io::NotFound => (ErrorKind::InvalidInput, "A file could not be found.", Some("It may have been moved or deleted. Add it again.")),
            Io::PermissionDenied => (
                ErrorKind::Output,
                "PaperFlow is not allowed to access this location.",
                Some("Choose a folder in your home directory or on a removable drive."),
            ),
            Io::StorageFull => (ErrorKind::Output, "The disk is full.", Some("Free up some space and try again.")),
            _ => (ErrorKind::Processing, "A file could not be read or written.", None),
        };
        let mut e = AppError::new(kind, message).with_detail(err.to_string());
        e.hint = hint.map(str::to_string);
        e
    }
}

impl From<lopdf::Error> for AppError {
    fn from(err: lopdf::Error) -> Self {
        match err {
            lopdf::Error::InvalidPassword | lopdf::Error::Decryption(_) => {
                AppError::new(ErrorKind::Encrypted, "This PDF is password-protected.")
                    .with_hint("Open it with Unlock PDF and your password first, then try again.")
            }
            other => AppError::invalid("This PDF appears to be damaged or uses features PaperFlow cannot read.")
                .with_hint("Try opening and re-saving it in another PDF viewer, then try again.")
                .with_detail(other.to_string()),
        }
    }
}

impl From<image::ImageError> for AppError {
    fn from(err: image::ImageError) -> Self {
        AppError::invalid("An image could not be read.")
            .with_hint("Use PNG, JPEG, WebP, BMP, TIFF or GIF images.")
            .with_detail(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_not_found_is_friendly() {
        let e: AppError = std::io::Error::new(std::io::ErrorKind::NotFound, "x").into();
        assert_eq!(e.kind, ErrorKind::InvalidInput);
        assert!(e.hint.is_some());
        assert!(e.detail.is_some());
    }

    #[test]
    fn serializes_camel_case() {
        let e = AppError::encrypted("a.pdf");
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["kind"], "encrypted");
        assert!(json["message"].as_str().unwrap().contains("a.pdf"));
    }
}
