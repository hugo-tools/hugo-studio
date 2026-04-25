//! Live preview (M6).
//!
//! Spawns `hugo server` against the active site, captures stdout/stderr
//! line by line, and streams them to the frontend via Tauri events.
//! Lifecycle is owned by [`PreviewProcess`]; dropping it kills the child
//! (`tokio::process::Command::kill_on_drop(true)`), which is the spec's
//! "no zombie hugos when the app crashes" guarantee.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult};

pub const LOG_EVENT: &str = "preview:log";
pub const READY_EVENT: &str = "preview:ready";
pub const ERROR_EVENT: &str = "preview:error";
pub const EXITED_EVENT: &str = "preview:exited";

const LOG_TAIL_KEEP: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewHandle {
    pub url: String,
    pub port: u16,
    pub hugo_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewStatus {
    pub running: bool,
    pub url: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLogPayload {
    pub stream: String, // "stdout" | "stderr"
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReadyPayload {
    pub url: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewErrorPayload {
    pub message: String,
    /// Last lines of stdout/stderr — useful when hugo crashes during
    /// startup and we never reach `Web Server is available at`.
    pub tail: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreviewExitedPayload {
    pub reason: String, // "stopped" | "exited" | "error"
    pub code: Option<i32>,
}

/// In-memory representation of a running hugo server. Stop the process by
/// either calling [`PreviewProcess::stop`] or letting the value drop —
/// the supervisor task watches `cancel_rx` and calls `child.kill()` when
/// it fires.
pub struct PreviewProcess {
    pub handle: PreviewHandle,
    cancel_tx: mpsc::Sender<()>,
}

impl PreviewProcess {
    pub fn stop(self) {
        let _ = self.cancel_tx.try_send(());
    }
}

/// Spawn `hugo server` for `site_root`. Resolves to the chosen URL/port
/// once the OS binds the port; the actual "Hugo is ready to serve"
/// signal arrives later as the [`READY_EVENT`].
///
/// `hugo_override` is the user-configured path from app settings; when
/// `None` we fall back to the env var → PATH chain inside `locate_hugo`.
pub async fn start(
    app: AppHandle,
    site_root: PathBuf,
    hugo_override: Option<String>,
) -> AppResult<PreviewProcess> {
    let hugo_path = locate_hugo(hugo_override.as_deref())?;
    let port = pick_free_port()?;
    let url = format!("http://127.0.0.1:{port}/");

    let mut cmd = Command::new(&hugo_path);
    cmd.current_dir(&site_root)
        .arg("server")
        .arg("-D")
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--navigateToChanged")
        .arg("--disableFastRender")
        .arg("--source")
        .arg(&site_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| {
        AppError::HugoBinary(format!("failed to spawn `{}`: {e}", hugo_path.display()))
    })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::HugoBinary("hugo stdout pipe missing".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::HugoBinary("hugo stderr pipe missing".into()))?;

    let tail: Arc<Mutex<VecDeque<String>>> =
        Arc::new(Mutex::new(VecDeque::with_capacity(LOG_TAIL_KEEP)));

    spawn_pump(app.clone(), stdout, "stdout", tail.clone(), port);
    spawn_pump(app.clone(), stderr, "stderr", tail.clone(), port);

    let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
    let supervisor_app = app.clone();
    let supervisor_tail = tail.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = cancel_rx.recv() => {
                let _ = child.kill().await;
                let _ = supervisor_app.emit(EXITED_EVENT, PreviewExitedPayload {
                    reason: "stopped".into(),
                    code: None,
                });
            }
            wait = child.wait() => match wait {
                Ok(status) => {
                    let payload = PreviewExitedPayload {
                        reason: "exited".into(),
                        code: status.code(),
                    };
                    if !status.success() {
                        let _ = supervisor_app.emit(ERROR_EVENT, PreviewErrorPayload {
                            message: format!("hugo exited with status {status}"),
                            tail: supervisor_tail.lock().iter().cloned().collect(),
                        });
                    }
                    let _ = supervisor_app.emit(EXITED_EVENT, payload);
                }
                Err(err) => {
                    let _ = supervisor_app.emit(ERROR_EVENT, PreviewErrorPayload {
                        message: err.to_string(),
                        tail: supervisor_tail.lock().iter().cloned().collect(),
                    });
                }
            }
        }
    });

    Ok(PreviewProcess {
        handle: PreviewHandle {
            url,
            port,
            hugo_path: hugo_path.display().to_string(),
        },
        cancel_tx,
    })
}

fn spawn_pump<R>(
    app: AppHandle,
    reader: R,
    stream: &'static str,
    tail: Arc<Mutex<VecDeque<String>>>,
    port: u16,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    {
                        let mut buf = tail.lock();
                        if buf.len() == LOG_TAIL_KEEP {
                            buf.pop_front();
                        }
                        buf.push_back(format!("[{stream}] {line}"));
                    }
                    if let Some(detected_url) = parse_ready_line(&line) {
                        let _ = app.emit(
                            READY_EVENT,
                            PreviewReadyPayload {
                                url: detected_url,
                                port,
                            },
                        );
                    }
                    let _ = app.emit(
                        LOG_EVENT,
                        PreviewLogPayload {
                            stream: stream.to_string(),
                            line,
                        },
                    );
                }
                Ok(None) => break, // EOF
                Err(_) => break,
            }
        }
    });
}

/// Find the Hugo binary. Order:
/// 1. `override_path` (set by the user from the Settings dialog)
/// 2. `HUGO_STUDIO_HUGO_PATH` env var (absolute path to a binary)
/// 3. `which hugo` on the user's `PATH`
pub fn locate_hugo(override_path: Option<&str>) -> AppResult<PathBuf> {
    if let Some(p) = override_path.filter(|s| !s.is_empty()) {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Ok(pb);
        }
        return Err(AppError::HugoBinary(format!(
            "configured Hugo path {p} is not a file"
        )));
    }
    if let Ok(env_path) = std::env::var("HUGO_STUDIO_HUGO_PATH") {
        let p = PathBuf::from(env_path);
        if p.is_file() {
            return Ok(p);
        }
        return Err(AppError::HugoBinary(format!(
            "HUGO_STUDIO_HUGO_PATH points to {} which is not a file",
            p.display()
        )));
    }
    which::which("hugo").map_err(|e| {
        AppError::HugoBinary(format!(
            "`hugo` not found on PATH ({e}); install Hugo, set HUGO_STUDIO_HUGO_PATH, or pick a binary from the Settings dialog"
        ))
    })
}

/// Ask the OS for an unused port; bind+drop creates a brief TOCTOU
/// window before hugo binds, but in practice it's fine.
fn pick_free_port() -> AppResult<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::Internal(e.to_string()))?
        .port();
    drop(listener);
    Ok(port)
}

/// Hugo prints e.g. `Web Server is available at http://127.0.0.1:1313/ (...)`.
/// Newer versions sometimes prefix with the bind address line. The simplest
/// robust extraction is "look for the URL right after `available at `".
fn parse_ready_line(line: &str) -> Option<String> {
    let needle = "Web Server is available at ";
    let pos = line.find(needle)?;
    let rest = &line[pos + needle.len()..];
    let url_end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    let candidate = &rest[..url_end];
    if candidate.starts_with("http://") || candidate.starts_with("https://") {
        return Some(candidate.to_string());
    }
    // Some Hugo builds drop the scheme.
    candidate
        .strip_prefix("//")
        .map(|rest| format!("http://{rest}"))
}

/// Resolve [`PreviewStatus`] from an optional handle. Used by the
/// `preview_status` command.
pub fn status_from(handle: Option<&PreviewHandle>) -> PreviewStatus {
    match handle {
        Some(h) => PreviewStatus {
            running: true,
            url: Some(h.url.clone()),
            port: Some(h.port),
        },
        None => PreviewStatus {
            running: false,
            url: None,
            port: None,
        },
    }
}

#[allow(dead_code)]
fn _path_kept(_p: &Path) {} // silence unused-import lint when only locate_hugo runs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_hugo_ready_line() {
        let line =
            "Web Server is available at http://127.0.0.1:1313/ (bind address 127.0.0.1) (Press Ctrl+C to stop)";
        assert_eq!(
            parse_ready_line(line).as_deref(),
            Some("http://127.0.0.1:1313/")
        );
    }

    #[test]
    fn parses_scheme_less_ready_line() {
        let line = "Web Server is available at //localhost:1313/ (...)";
        assert_eq!(
            parse_ready_line(line).as_deref(),
            Some("http://localhost:1313/")
        );
    }

    #[test]
    fn ignores_unrelated_lines() {
        assert!(parse_ready_line("Built in 12 ms").is_none());
        assert!(parse_ready_line("ERROR: something").is_none());
    }

    #[test]
    fn pick_free_port_returns_a_usable_port() {
        let p = pick_free_port().unwrap();
        assert!(p > 1024);
        // Must be re-bindable since we dropped the listener.
        let l = std::net::TcpListener::bind(("127.0.0.1", p));
        assert!(l.is_ok());
    }

    #[test]
    fn locate_hugo_via_env_var_rejects_missing_file() {
        let _g = EnvGuard::set("HUGO_STUDIO_HUGO_PATH", "/definitely/not/here");
        let err = locate_hugo(None).unwrap_err();
        match err {
            AppError::HugoBinary(msg) => assert!(msg.contains("not a file")),
            other => panic!("expected HugoBinary, got {other:?}"),
        }
    }

    /// Test helper that scopes an env var change so it doesn't leak between
    /// tests. Cargo runs tests on a thread pool — keep it serial via the
    /// `std::env` global lock by choice of API; the guard restores on Drop.
    struct EnvGuard {
        key: &'static str,
        prior: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prior = std::env::var(key).ok();
            // SAFETY: cargo test serializes env access in our test set; this
            // is a small project, OK for now.
            unsafe { std::env::set_var(key, value) };
            Self { key, prior }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.prior {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }
}
