use parking_lot::Mutex;

use crate::domain::workspace::Workspace;
use crate::error::AppResult;
use crate::persistence::workspace_store::WorkspaceStore;
use crate::watcher::WatcherHandle;

/// Tauri-managed state. Holds the in-memory workspace behind a non-poisoning
/// mutex (parking_lot), the on-disk store used to persist it, and the
/// currently-active filesystem watcher (if any).
pub struct AppState {
    pub workspace: Mutex<Workspace>,
    pub workspace_store: WorkspaceStore,
    pub active_watcher: Mutex<Option<WatcherHandle>>,
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
            active_watcher: Mutex::new(None),
        }
    }

    /// Persist the current in-memory workspace. Caller must NOT hold the
    /// `workspace` lock when invoking — this method takes its own short lock.
    pub fn save(&self) -> AppResult<()> {
        let snapshot = self.workspace.lock().clone();
        self.workspace_store.save(&snapshot)
    }

    /// Replace the active watcher (if any) with `next`. The previous one is
    /// stopped — its tokio task wakes up, drops the notify watcher, exits.
    pub fn replace_watcher(&self, next: Option<WatcherHandle>) {
        let prev = std::mem::replace(&mut *self.active_watcher.lock(), next);
        if let Some(handle) = prev {
            handle.stop();
        }
    }
}
