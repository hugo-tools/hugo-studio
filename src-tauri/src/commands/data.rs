use std::path::PathBuf;

use tauri::State;

use crate::data::{self, DataFile, DataFileContent};
use crate::domain::ids::SiteId;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

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
pub fn data_list(state: State<'_, AppState>, site_id: SiteId) -> AppResult<Vec<DataFile>> {
    let root = site_root(&state, site_id)?;
    data::list(&root)
}

#[tauri::command]
#[specta::specta]
pub fn data_read(
    state: State<'_, AppState>,
    site_id: SiteId,
    rel_path: String,
) -> AppResult<DataFileContent> {
    let root = site_root(&state, site_id)?;
    data::read_text(&root, &rel_path)
}

#[tauri::command]
#[specta::specta]
pub fn data_write(
    state: State<'_, AppState>,
    site_id: SiteId,
    rel_path: String,
    text: String,
) -> AppResult<DataFileContent> {
    let root = site_root(&state, site_id)?;
    data::write_text(&root, &rel_path, &text)
}

#[tauri::command]
#[specta::specta]
pub fn data_create(
    state: State<'_, AppState>,
    site_id: SiteId,
    rel_path: String,
) -> AppResult<DataFile> {
    let root = site_root(&state, site_id)?;
    data::create(&root, &rel_path)
}

#[tauri::command]
#[specta::specta]
pub fn data_import(
    state: State<'_, AppState>,
    site_id: SiteId,
    source: String,
) -> AppResult<DataFile> {
    let root = site_root(&state, site_id)?;
    data::import(&root, std::path::Path::new(&source))
}

#[tauri::command]
#[specta::specta]
pub fn data_delete(state: State<'_, AppState>, site_id: SiteId, rel_path: String) -> AppResult<()> {
    let root = site_root(&state, site_id)?;
    data::delete(&root, &rel_path)
}
