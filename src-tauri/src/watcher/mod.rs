use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use notify::{recommended_watcher, RecursiveMode, Watcher};
use serde::Serialize;
use specta::Type;
use tauri::{async_runtime, AppHandle, Emitter};
use tokio::sync::mpsc as tokio_mpsc;

use crate::error::{AppError, AppResult};

pub const SITE_CHANGED_EVENT: &str = "site:changed";

const DEBOUNCE: Duration = Duration::from_millis(200);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteChangedPayload {
    pub paths: Vec<String>,
}

/// Owns a notify watcher + the tokio task that debounces and forwards events.
/// Drop / [`stop`](Self::stop) halts both the watcher thread and the task.
pub struct WatcherHandle {
    cancel_tx: tokio_mpsc::Sender<()>,
}

impl WatcherHandle {
    pub fn stop(self) {
        // Best effort — receiver may already be gone.
        let _ = self.cancel_tx.try_send(());
    }
}

/// Spawn a recursive watcher over `root`. Filesystem events that survive
/// the ignore-list and the 200ms debounce window are emitted to the
/// frontend as the [`SITE_CHANGED_EVENT`] Tauri event.
pub fn spawn(app: AppHandle, root: PathBuf) -> AppResult<WatcherHandle> {
    let (cancel_tx, mut cancel_rx) = tokio_mpsc::channel::<()>(1);
    let (event_tx, event_rx) = std_mpsc::channel::<notify::Result<notify::Event>>();

    let mut watcher = recommended_watcher(move |res| {
        let _ = event_tx.send(res);
    })
    .map_err(|e| AppError::Internal(format!("notify init: {e}")))?;

    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| AppError::Internal(format!("notify watch: {e}")))?;

    let watch_root = root.clone();
    // Use Tauri's managed tokio runtime instead of `tokio::spawn` directly:
    // `workspace_set_active` is a *sync* command, so when Tauri invokes it
    // there is no current tokio runtime in the calling thread context. A
    // bare `tokio::spawn` would panic ("there is no reactor running") and
    // the panic handler would `abort()` the whole app — exactly what
    // happened on macOS in v0.5.0 when opening a site. `async_runtime::spawn`
    // delegates to the runtime Tauri owns, which is always available.
    async_runtime::spawn(async move {
        // Keep the watcher alive for the lifetime of this task.
        let _watcher_keep_alive = watcher;
        let mut pending: Vec<String> = Vec::new();
        let mut last_event_at: Option<tokio::time::Instant> = None;

        loop {
            tokio::select! {
                _ = cancel_rx.recv() => break,
                _ = tokio::time::sleep(POLL_INTERVAL) => {
                    while let Ok(res) = event_rx.try_recv() {
                        let Ok(event) = res else { continue };
                        for p in event.paths {
                            if is_ignored(&p, &watch_root) {
                                continue;
                            }
                            pending.push(p.display().to_string());
                            last_event_at = Some(tokio::time::Instant::now());
                        }
                    }
                    if let Some(ts) = last_event_at {
                        if ts.elapsed() >= DEBOUNCE && !pending.is_empty() {
                            let payload = SiteChangedPayload {
                                paths: std::mem::take(&mut pending),
                            };
                            if let Err(err) = app.emit(SITE_CHANGED_EVENT, payload) {
                                eprintln!("[watcher] emit failed: {err}");
                            }
                            last_event_at = None;
                        }
                    }
                }
            }
        }
    });

    Ok(WatcherHandle { cancel_tx })
}

/// Paths Hugo / the IDE generate that we never want to surface to the UI
/// (would trigger a reload storm during a `hugo` build).
const IGNORED_NAMES: &[&str] = &[
    "public",
    "resources",
    "node_modules",
    ".git",
    ".hugo_build.lock",
    ".DS_Store",
];

fn is_ignored(path: &Path, root: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    rel.components().any(|c| match c {
        std::path::Component::Normal(s) => {
            let name = s.to_string_lossy();
            IGNORED_NAMES.iter().any(|n| n.eq_ignore_ascii_case(&name))
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_public_directory() {
        let root = Path::new("/tmp/site");
        assert!(is_ignored(&root.join("public/index.html"), root));
    }

    #[test]
    fn ignores_dotgit() {
        let root = Path::new("/tmp/site");
        assert!(is_ignored(&root.join(".git/HEAD"), root));
    }

    #[test]
    fn allows_content_changes() {
        let root = Path::new("/tmp/site");
        assert!(!is_ignored(&root.join("content/posts/hello.md"), root));
    }

    #[test]
    fn ignores_hugo_build_lock() {
        let root = Path::new("/tmp/site");
        assert!(is_ignored(&root.join(".hugo_build.lock"), root));
    }
}
