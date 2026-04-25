use std::path::PathBuf;

use tauri::State;

use crate::config::cascade::{self, LoadedConfig};
use crate::domain::ids::SiteId;
use crate::error::{AppError, AppResult};
use crate::hugo::detect;
use crate::state::AppState;

fn detection_for(state: &AppState, id: SiteId) -> AppResult<crate::hugo::detect::DetectionInfo> {
    let path = state
        .workspace
        .lock()
        .sites
        .iter()
        .find(|s| s.id == id)
        .map(|s| PathBuf::from(&s.root_path))
        .ok_or_else(|| AppError::SiteNotFound(id.to_string()))?;
    detect::detect(&path)
}

#[tauri::command]
#[specta::specta]
pub fn config_get(state: State<'_, AppState>, site_id: SiteId) -> AppResult<LoadedConfig> {
    let det = detection_for(&state, site_id)?;
    cascade::load(&det)
}

#[tauri::command]
#[specta::specta]
pub fn config_save(
    state: State<'_, AppState>,
    site_id: SiteId,
    merged: serde_json::Value,
) -> AppResult<LoadedConfig> {
    let det = detection_for(&state, site_id)?;
    cascade::save(&det, &merged)?;
    cascade::load(&det)
}
