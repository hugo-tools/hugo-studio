use std::path::PathBuf;

use tauri::State;

use crate::domain::ids::SiteId;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::theme::{self, ThemeInfo};

fn site_root(state: &State<'_, AppState>, site_id: SiteId) -> AppResult<PathBuf> {
    state
        .workspace
        .lock()
        .sites
        .iter()
        .find(|s| s.id == site_id)
        .map(|s| PathBuf::from(&s.root_path))
        .ok_or_else(|| AppError::SiteNotFound(site_id.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn theme_get(state: State<'_, AppState>, site_id: SiteId) -> AppResult<ThemeInfo> {
    let root = site_root(&state, site_id)?;
    theme::load(&root)
}

#[tauri::command]
#[specta::specta]
pub fn theme_save_params(
    state: State<'_, AppState>,
    site_id: SiteId,
    params: serde_json::Value,
) -> AppResult<ThemeInfo> {
    let root = site_root(&state, site_id)?;
    theme::save_params(&root, &params)?;
    theme::load(&root)
}
