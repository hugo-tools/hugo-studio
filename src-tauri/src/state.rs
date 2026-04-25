use parking_lot::Mutex;

use crate::domain::workspace::Workspace;
use crate::error::AppResult;
use crate::persistence::workspace_store::WorkspaceStore;

/// Tauri-managed state. Holds the in-memory workspace behind a non-poisoning
/// mutex (parking_lot) and the on-disk store used to persist it.
pub struct AppState {
    pub workspace: Mutex<Workspace>,
    pub workspace_store: WorkspaceStore,
}

impl AppState {
    pub fn new(workspace_store: WorkspaceStore) -> Self {
        let workspace = workspace_store.load().unwrap_or_else(|err| {
            eprintln!(
                "[hugo-studio] failed to load workspace from {}: {err} — starting empty",
                workspace_store.path().display()
            );
            Workspace::default()
        });
        Self {
            workspace: Mutex::new(workspace),
            workspace_store,
        }
    }

    /// Persist the current in-memory workspace. Caller must NOT hold the
    /// `workspace` lock when invoking — this method takes its own short lock.
    pub fn save(&self) -> AppResult<()> {
        let snapshot = self.workspace.lock().clone();
        self.workspace_store.save(&snapshot)
    }
}
