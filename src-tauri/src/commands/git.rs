use std::path::PathBuf;

use tauri::State;

use crate::domain::ids::SiteId;
use crate::error::{AppError, AppResult};
use crate::git::{self, CloneOptions, CloneResult, CommitResult, GitStatus, PullStrategy};
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
pub fn git_status(state: State<'_, AppState>, site_id: SiteId) -> AppResult<GitStatus> {
    let root = site_root(&state, site_id)?;
    git::status(&root)
}

#[tauri::command]
#[specta::specta]
pub fn git_clone(opts: CloneOptions) -> AppResult<CloneResult> {
    git::clone_repo(&opts)
}

#[tauri::command]
#[specta::specta]
pub fn git_stage(
    state: State<'_, AppState>,
    site_id: SiteId,
    paths: Vec<String>,
) -> AppResult<GitStatus> {
    let root = site_root(&state, site_id)?;
    git::stage(&root, &paths)?;
    git::status(&root)
}

#[tauri::command]
#[specta::specta]
pub fn git_unstage(
    state: State<'_, AppState>,
    site_id: SiteId,
    paths: Vec<String>,
) -> AppResult<GitStatus> {
    let root = site_root(&state, site_id)?;
    git::unstage(&root, &paths)?;
    git::status(&root)
}

#[tauri::command]
#[specta::specta]
pub fn git_commit(
    state: State<'_, AppState>,
    site_id: SiteId,
    message: String,
) -> AppResult<CommitResult> {
    let root = site_root(&state, site_id)?;
    git::commit(&root, &message)
}

#[tauri::command]
#[specta::specta]
pub fn git_pull(
    state: State<'_, AppState>,
    site_id: SiteId,
    strategy: PullStrategy,
) -> AppResult<GitStatus> {
    let root = site_root(&state, site_id)?;
    git::pull(&root, strategy)?;
    git::status(&root)
}

#[tauri::command]
#[specta::specta]
pub fn git_stash_save(
    state: State<'_, AppState>,
    site_id: SiteId,
    message: String,
) -> AppResult<GitStatus> {
    let root = site_root(&state, site_id)?;
    git::stash_save(&root, &message)?;
    git::status(&root)
}

#[tauri::command]
#[specta::specta]
pub fn git_stash_pop(state: State<'_, AppState>, site_id: SiteId) -> AppResult<GitStatus> {
    let root = site_root(&state, site_id)?;
    git::stash_pop(&root)?;
    git::status(&root)
}

#[tauri::command]
#[specta::specta]
pub fn git_push(state: State<'_, AppState>, site_id: SiteId) -> AppResult<GitStatus> {
    let root = site_root(&state, site_id)?;
    git::push(&root)?;
    git::status(&root)
}
