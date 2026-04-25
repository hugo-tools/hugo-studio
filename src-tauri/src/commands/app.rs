use tauri::State;

use crate::app::settings::AppSettings;
use crate::error::AppResult;
use crate::preview;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn app_settings_get(state: State<'_, AppState>) -> AppResult<AppSettings> {
    Ok(state.settings.lock().clone())
}

#[tauri::command]
#[specta::specta]
pub fn app_settings_save(state: State<'_, AppState>, next: AppSettings) -> AppResult<AppSettings> {
    {
        let mut current = state.settings.lock();
        *current = next;
    }
    state.save_settings()?;
    Ok(state.settings.lock().clone())
}

/// Echo back the path Hugo Studio would actually use right now (resolved
/// override → env var → PATH). Returns `None` when nothing is reachable —
/// the SettingsDialog uses this to render a green/red status hint.
#[tauri::command]
#[specta::specta]
pub fn app_settings_resolve_hugo(state: State<'_, AppState>) -> AppResult<Option<String>> {
    let override_path = state.settings.lock().hugo_path.clone();
    match preview::locate_hugo(override_path.as_deref()) {
        Ok(p) => Ok(Some(p.display().to_string())),
        Err(_) => Ok(None),
    }
}
