use parking_lot::Mutex;

use crate::domain::workspace::Workspace;
use crate::error::AppResult;
use crate::persistence::workspace_store::WorkspaceStore;
use crate::preview::PreviewProcess;
use crate::watcher::WatcherHandle;

/// Tauri-managed state. Holds the in-memory workspace behind a non-poisoning
/// mutex (parking_lot), the on-disk store used to persist it, the file
/// watcher and the live-preview process.
pub struct AppState {
    pub workspace: Mutex<Workspace>,
    pub workspace_store: WorkspaceStore,
    pub active_watcher: Mutex<Option<WatcherHandle>>,
    pub active_preview: Mutex<Option<PreviewProcess>>,
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
            active_preview: Mutex::new(None),
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

    /// Replace the live-preview process. The previous one is gracefully
    /// killed (tokio::process::Command was spawned with kill_on_drop, so the
    /// child dies even if we forget to stop explicitly).
    pub fn replace_preview(&self, next: Option<PreviewProcess>) {
        let prev = std::mem::replace(&mut *self.active_preview.lock(), next);
        if let Some(p) = prev {
            p.stop();
        }
    }
}
