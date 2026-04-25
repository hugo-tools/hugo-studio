use std::path::PathBuf;

use crate::error::AppResult;
use crate::hugo::detect::{self, DetectionInfo};

/// Inspect a path and report whether it's a valid Hugo site.
/// Used by the "Add site" flow to give immediate feedback in the picker.
#[tauri::command]
#[specta::specta]
pub fn site_detect(path: String) -> AppResult<DetectionInfo> {
    detect::detect(&PathBuf::from(path))
}
