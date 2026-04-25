use std::path::PathBuf;

use chrono::Utc;
use tauri::State;

use crate::domain::ids::SiteId;
use crate::domain::site::Site;
use crate::domain::workspace::SiteRef;
use crate::error::{AppError, AppResult};
use crate::hugo::detect;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn workspace_list_sites(state: State<'_, AppState>) -> AppResult<Vec<SiteRef>> {
    Ok(state.workspace.lock().sites.clone())
}

#[tauri::command]
#[specta::specta]
pub fn workspace_active_site_id(state: State<'_, AppState>) -> AppResult<Option<SiteId>> {
    Ok(state.workspace.lock().active_site_id)
}

#[tauri::command]
#[specta::specta]
pub fn workspace_add_site(
    state: State<'_, AppState>,
    path: String,
    name: Option<String>,
) -> AppResult<SiteRef> {
    let path_buf = PathBuf::from(&path);
    // Reject non-Hugo directories before storing them.
    detect::detect(&path_buf)?;

    let canonical = std::fs::canonicalize(&path_buf)
        .unwrap_or(path_buf.clone())
        .display()
        .to_string();

    // Idempotency: if the same canonical path is already registered, return the
    // existing entry instead of duplicating it.
    if let Some(existing) = state
        .workspace
        .lock()
        .sites
        .iter()
        .find(|s| s.root_path == canonical)
        .cloned()
    {
        return Ok(existing);
    }

    let inferred_name = name
        .or_else(|| {
            path_buf
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| canonical.clone());

    let site_ref = SiteRef {
        id: SiteId::new(),
        name: inferred_name,
        root_path: canonical,
        last_opened: Utc::now(),
    };

    state.workspace.lock().sites.push(site_ref.clone());
    state.save()?;
    Ok(site_ref)
}

#[tauri::command]
#[specta::specta]
pub fn workspace_remove_site(state: State<'_, AppState>, id: SiteId) -> AppResult<()> {
    {
        let mut ws = state.workspace.lock();
        let before = ws.sites.len();
        ws.sites.retain(|s| s.id != id);
        if ws.sites.len() == before {
            return Err(AppError::SiteNotFound(id.to_string()));
        }
        if ws.active_site_id == Some(id) {
            ws.active_site_id = None;
        }
    }
    state.save()
}

#[tauri::command]
#[specta::specta]
pub fn workspace_rename_site(
    state: State<'_, AppState>,
    id: SiteId,
    name: String,
) -> AppResult<SiteRef> {
    let updated = {
        let mut ws = state.workspace.lock();
        let site = ws
            .sites
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| AppError::SiteNotFound(id.to_string()))?;
        site.name = name;
        site.clone()
    };
    state.save()?;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub fn workspace_set_active(state: State<'_, AppState>, id: SiteId) -> AppResult<Site> {
    let site_ref = {
        let mut ws = state.workspace.lock();
        let site_clone = {
            let site = ws
                .sites
                .iter_mut()
                .find(|s| s.id == id)
                .ok_or_else(|| AppError::SiteNotFound(id.to_string()))?;
            site.last_opened = Utc::now();
            site.clone()
        };
        ws.active_site_id = Some(id);
        site_clone
    };
    state.save()?;

    let detection = detect::detect(&PathBuf::from(&site_ref.root_path))?;
    Ok(Site::from_ref_with_detection(&site_ref, detection))
}

#[tauri::command]
#[specta::specta]
pub fn workspace_clear_active(state: State<'_, AppState>) -> AppResult<()> {
    state.workspace.lock().active_site_id = None;
    state.save()
}
