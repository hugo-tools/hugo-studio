use std::path::PathBuf;

use tauri::State;

use crate::domain::ids::SiteId;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::theme_files::{self, ThemeFileContent, ThemeFilesIndex};

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
pub fn theme_files_list(state: State<'_, AppState>, site_id: SiteId) -> AppResult<ThemeFilesIndex> {
    let root = site_root(&state, site_id)?;
    theme_files::list(&root)
}

#[tauri::command]
#[specta::specta]
pub fn theme_file_read(
    state: State<'_, AppState>,
    site_id: SiteId,
    rel_path: String,
) -> AppResult<ThemeFileContent> {
    let root = site_root(&state, site_id)?;
    theme_files::read_text(&root, &rel_path)
}

#[tauri::command]
#[specta::specta]
pub fn theme_file_write(
    state: State<'_, AppState>,
    site_id: SiteId,
    rel_path: String,
    text: String,
) -> AppResult<ThemeFileContent> {
    let root = site_root(&state, site_id)?;
    theme_files::write_text(&root, &rel_path, &text)
}
