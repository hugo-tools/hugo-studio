use std::path::PathBuf;

use tauri::State;

use crate::assets::{self, AssetContext, AssetRef};
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
pub fn asset_import(
    state: State<'_, AppState>,
    site_id: SiteId,
    source: String,
    target_context: AssetContext,
) -> AppResult<AssetRef> {
    let root = site_root(&state, site_id)?;
    assets::import(&root, &PathBuf::from(source), &target_context)
}

#[tauri::command]
#[specta::specta]
pub fn asset_list(
    state: State<'_, AppState>,
    site_id: SiteId,
    content_id: Option<String>,
) -> AppResult<Vec<AssetRef>> {
    let root = site_root(&state, site_id)?;
    assets::list(&root, content_id.as_deref())
}

#[tauri::command]
#[specta::specta]
pub fn asset_delete(
    state: State<'_, AppState>,
    site_id: SiteId,
    asset_id: String,
) -> AppResult<()> {
    let root = site_root(&state, site_id)?;
    assets::delete(&root, &asset_id)
}

#[tauri::command]
#[specta::specta]
pub fn asset_list_static(
    state: State<'_, AppState>,
    site_id: SiteId,
) -> AppResult<Vec<AssetRef>> {
    let root = site_root(&state, site_id)?;
    assets::list_static(&root)
}

#[tauri::command]
#[specta::specta]
pub fn asset_list_assets(
    state: State<'_, AppState>,
    site_id: SiteId,
) -> AppResult<Vec<AssetRef>> {
    let root = site_root(&state, site_id)?;
    assets::list_assets(&root)
}
