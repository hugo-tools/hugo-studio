use std::path::PathBuf;

use tauri::State;

use crate::config::cascade;
use crate::content::scan::{self, ContentScanResult};
use crate::domain::ids::SiteId;
use crate::error::{AppError, AppResult};
use crate::hugo::detect;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn content_list(state: State<'_, AppState>, site_id: SiteId) -> AppResult<ContentScanResult> {
    let root = state
        .workspace
        .lock()
        .sites
        .iter()
        .find(|s| s.id == site_id)
        .map(|s| PathBuf::from(&s.root_path))
        .ok_or_else(|| AppError::SiteNotFound(site_id.to_string()))?;

    let det = detect::detect(&root)?;
    let merged = cascade::load(&det)?.merged;
    scan::scan(&root, &merged)
}
