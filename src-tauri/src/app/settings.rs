use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// Absolute path to a Hugo binary the user wants the preview to use
    /// instead of the one on PATH. `None` falls back to
    /// `HUGO_STUDIO_HUGO_PATH` env var → PATH lookup (see
    /// `preview::locate_hugo`).
    pub hugo_path: Option<String>,
}

pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn load(&self) -> AppResult<AppSettings> {
        if !self.path.exists() {
            return Ok(AppSettings::default());
        }
        let bytes = std::fs::read(&self.path)?;
        let settings: AppSettings = serde_json::from_slice(&bytes)?;
        Ok(settings)
    }

    pub fn save(&self, settings: &AppSettings) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(settings)?;
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &self.path).map_err(AppError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let store = SettingsStore::new(tmp.path().join("settings.json"));
        let s = store.load().unwrap();
        assert!(s.hugo_path.is_none());
    }

    #[test]
    fn round_trip_through_disk() {
        let tmp = TempDir::new().unwrap();
        let store = SettingsStore::new(tmp.path().join("settings.json"));
        let s = AppSettings {
            hugo_path: Some("/opt/hugo".into()),
        };
        store.save(&s).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.hugo_path.as_deref(), Some("/opt/hugo"));
    }
}
