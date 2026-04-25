use std::path::PathBuf;

use crate::domain::workspace::Workspace;
use crate::error::{AppError, AppResult};

/// Reads/writes the workspace JSON in the OS-specific app data dir.
/// Writes are atomic (write to `*.tmp` then rename) so a crash mid-save
/// can never leave the user with a half-written workspace file.
pub struct WorkspaceStore {
    path: PathBuf,
}

impl WorkspaceStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn load(&self) -> AppResult<Workspace> {
        if !self.path.exists() {
            return Ok(Workspace::default());
        }
        let bytes = std::fs::read(&self.path)?;
        let workspace: Workspace = serde_json::from_slice(&bytes)?;
        Ok(workspace)
    }

    pub fn save(&self, workspace: &Workspace) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(workspace)?;
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &self.path).map_err(AppError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::SiteId;
    use crate::domain::workspace::SiteRef;
    use chrono::Utc;
    use tempfile::TempDir;

    fn ws_with_one_site() -> Workspace {
        Workspace {
            sites: vec![SiteRef {
                id: SiteId::new(),
                name: "Example".into(),
                root_path: "/tmp/example".into(),
                last_opened: Utc::now(),
            }],
            active_site_id: None,
        }
    }

    #[test]
    fn round_trip_through_disk() {
        let tmp = TempDir::new().unwrap();
        let store = WorkspaceStore::new(tmp.path().join("workspace.json"));
        let original = ws_with_one_site();
        store.save(&original).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.sites.len(), 1);
        assert_eq!(loaded.sites[0].id, original.sites[0].id);
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let store = WorkspaceStore::new(tmp.path().join("nope.json"));
        let loaded = store.load().unwrap();
        assert!(loaded.sites.is_empty());
        assert!(loaded.active_site_id.is_none());
    }

    #[test]
    fn save_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("a/b/c/workspace.json");
        let store = WorkspaceStore::new(nested.clone());
        store.save(&Workspace::default()).unwrap();
        assert!(nested.exists());
    }
}
