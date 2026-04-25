use serde::Serialize;
use specta::Type;
use thiserror::Error;

/// All errors surfaced to the frontend.
///
/// Serialized with a `kind` discriminator and a `message` payload so that the
/// TypeScript side can switch on `kind` without parsing free-form strings.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum AppError {
    #[error("not a Hugo site: {0}")]
    NotAHugoSite(String),

    #[error("site not found: {0}")]
    SiteNotFound(String),

    #[error("path is not a directory: {0}")]
    NotADirectory(String),

    // Reserved for FS sandboxing in M3 (§6.9). Keep the variant so downstream
    // code can already match on it without churning the API later.
    #[allow(dead_code)]
    #[error("path traversal denied: {0}")]
    PathTraversal(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("serde error: {0}")]
    Serde(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Io(value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        AppError::Serde(value.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(value: tauri::Error) -> Self {
        AppError::Internal(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
