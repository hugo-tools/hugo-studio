use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::domain::ids::SiteId;
use crate::error::{AppError, AppResult};
use crate::preview::{self, PreviewHandle, PreviewStatus};
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
pub async fn preview_start(
    app: AppHandle,
    state: State<'_, AppState>,
    site_id: SiteId,
) -> AppResult<PreviewHandle> {
    if state.active_preview.lock().is_some() {
        return Err(AppError::PreviewAlreadyRunning(site_id.to_string()));
    }
    let root = site_root(&state, site_id)?;
    let process = preview::start(app, root).await?;
    let handle = process.handle.clone();
    state.replace_preview(Some(process));
    Ok(handle)
}

#[tauri::command]
#[specta::specta]
pub fn preview_stop(state: State<'_, AppState>) -> AppResult<()> {
    if state.active_preview.lock().is_none() {
        return Err(AppError::NoPreviewRunning);
    }
    state.replace_preview(None);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn preview_status(state: State<'_, AppState>) -> AppResult<PreviewStatus> {
    let snapshot = state
        .active_preview
        .lock()
        .as_ref()
        .map(|p| p.handle.clone());
    Ok(preview::status_from(snapshot.as_ref()))
}
