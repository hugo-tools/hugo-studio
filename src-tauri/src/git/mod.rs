//! Git integration (M-git).
//!
//! Wraps `libgit2` (via the `git2` crate) so the user can clone a Hugo
//! site straight from the Workspace screen and stage / commit / pull /
//! push without leaving the editor.
//!
//! Authentication strategy:
//!   - SSH: tries the user's running ssh-agent first.
//!   - HTTPS: falls back to the system git credential helper. This means
//!     HTTPS auth practically requires the `git` CLI on the user's PATH;
//!     SSH does not.
//!
//! Vendored libgit2 + openssl, so end users don't need any system C
//! libraries installed.

use std::path::{Path, PathBuf};

use git2::{
    Cred, CredentialType, FetchOptions, IndexAddOption, PushOptions, RemoteCallbacks, Repository,
    ResetType, Signature, StashApplyOptions, StashFlags, StatusOptions,
};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum GitChangeStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
    Ignored,
    TypeChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GitChange {
    pub path: String,
    pub status: GitChangeStatus,
    /// True when the change is in the index (staged), false when in the
    /// working tree only.
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    /// `None` when the repository has no commits yet or is in a detached
    /// HEAD state.
    pub branch: Option<String>,
    /// Resolved upstream branch name like `origin/main`, or `None` when
    /// the current branch has no upstream configured.
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub changes: Vec<GitChange>,
    pub remotes: Vec<String>,
    /// True when the directory is a git repo at all.
    pub is_repo: bool,
    /// Convenience flag for the UI: anything to commit / push.
    pub clean: bool,
}

impl GitStatus {
    fn empty(is_repo: bool) -> Self {
        Self {
            branch: None,
            upstream: None,
            ahead: 0,
            behind: 0,
            changes: vec![],
            remotes: vec![],
            is_repo,
            clean: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CloneOptions {
    pub url: String,
    /// Absolute path of the destination directory. Must NOT exist yet.
    pub dest: String,
    /// Optional branch to check out after clone.
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CloneResult {
    pub dest: String,
    pub head: Option<String>,
}

/// Inspect the working directory and return a UI-friendly snapshot.
/// Non-repo directories return a successful result with `is_repo: false`
/// rather than an error — the panel uses that to render an "init / clone"
/// CTA instead of an alarm.
pub fn status(repo_path: &Path) -> AppResult<GitStatus> {
    let repo = match Repository::discover(repo_path) {
        Ok(r) => r,
        Err(_) => return Ok(GitStatus::empty(false)),
    };

    let branch = head_branch_name(&repo);
    let (upstream_name, ahead, behind) = upstream_state(&repo).unwrap_or((None, 0, 0));

    let mut changes = Vec::new();
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .include_ignored(false)
        .recurse_untracked_dirs(true);
    let entries = repo
        .statuses(Some(&mut opts))
        .map_err(|e| AppError::Internal(format!("git status: {e}")))?;
    for entry in entries.iter() {
        let path = match entry.path() {
            Some(p) => p.to_string(),
            None => continue,
        };
        let s = entry.status();
        // Index side
        if s.is_index_new() {
            changes.push(change(&path, GitChangeStatus::New, true));
        }
        if s.is_index_modified() {
            changes.push(change(&path, GitChangeStatus::Modified, true));
        }
        if s.is_index_deleted() {
            changes.push(change(&path, GitChangeStatus::Deleted, true));
        }
        if s.is_index_renamed() {
            changes.push(change(&path, GitChangeStatus::Renamed, true));
        }
        if s.is_index_typechange() {
            changes.push(change(&path, GitChangeStatus::TypeChange, true));
        }
        // Working-tree side
        if s.is_wt_new() {
            changes.push(change(&path, GitChangeStatus::Untracked, false));
        }
        if s.is_wt_modified() {
            changes.push(change(&path, GitChangeStatus::Modified, false));
        }
        if s.is_wt_deleted() {
            changes.push(change(&path, GitChangeStatus::Deleted, false));
        }
        if s.is_wt_renamed() {
            changes.push(change(&path, GitChangeStatus::Renamed, false));
        }
        if s.is_wt_typechange() {
            changes.push(change(&path, GitChangeStatus::TypeChange, false));
        }
        if s.is_conflicted() {
            changes.push(change(&path, GitChangeStatus::Conflicted, false));
        }
    }

    let remotes: Vec<String> = repo
        .remotes()
        .map(|arr| arr.iter().flatten().map(String::from).collect())
        .unwrap_or_default();

    let clean = changes.is_empty() && ahead == 0;

    Ok(GitStatus {
        branch,
        upstream: upstream_name,
        ahead,
        behind,
        changes,
        remotes,
        is_repo: true,
        clean,
    })
}

fn change(path: &str, status: GitChangeStatus, staged: bool) -> GitChange {
    GitChange {
        path: path.to_string(),
        status,
        staged,
    }
}

fn head_branch_name(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    head.shorthand().map(String::from)
}

fn upstream_state(repo: &Repository) -> AppResult<(Option<String>, u32, u32)> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok((None, 0, 0)),
    };
    if !head.is_branch() {
        return Ok((None, 0, 0));
    }
    let local_branch = match repo.find_branch(
        head.shorthand().unwrap_or_default(),
        git2::BranchType::Local,
    ) {
        Ok(b) => b,
        Err(_) => return Ok((None, 0, 0)),
    };
    let upstream = match local_branch.upstream() {
        Ok(u) => u,
        Err(_) => return Ok((None, 0, 0)),
    };
    let upstream_name = upstream.name().ok().flatten().map(String::from);

    let local_oid = local_branch
        .get()
        .target()
        .ok_or_else(|| AppError::Internal("local branch has no target".into()))?;
    let upstream_oid = upstream
        .get()
        .target()
        .ok_or_else(|| AppError::Internal("upstream branch has no target".into()))?;
    let (ahead, behind) = repo
        .graph_ahead_behind(local_oid, upstream_oid)
        .unwrap_or((0, 0));
    Ok((upstream_name, ahead as u32, behind as u32))
}

pub fn stage(repo_path: &Path, paths: &[String]) -> AppResult<()> {
    let repo = open(repo_path)?;
    let mut index = repo.index().map_err(map_git_err)?;
    if paths.is_empty() {
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .map_err(map_git_err)?;
    } else {
        for p in paths {
            // git2 needs paths relative to the repo root.
            index.add_path(Path::new(p)).map_err(map_git_err)?;
        }
    }
    index.write().map_err(map_git_err)
}

pub fn unstage(repo_path: &Path, paths: &[String]) -> AppResult<()> {
    let repo = open(repo_path)?;
    let head_obj = repo
        .head()
        .ok()
        .and_then(|h| h.peel(git2::ObjectType::Any).ok());
    let path_specs: Vec<&str> = paths.iter().map(String::as_str).collect();
    if let Some(obj) = head_obj {
        repo.reset_default(Some(&obj), &path_specs)
            .map_err(map_git_err)?;
    } else {
        // No HEAD yet (initial commit not made): "unstage" = remove from index
        let mut index = repo.index().map_err(map_git_err)?;
        for p in paths {
            let _ = index.remove_path(Path::new(p));
        }
        index.write().map_err(map_git_err)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CommitResult {
    pub oid: String,
    pub summary: String,
}

pub fn commit(repo_path: &Path, message: &str) -> AppResult<CommitResult> {
    if message.trim().is_empty() {
        return Err(AppError::Internal("commit message is empty".into()));
    }
    let repo = open(repo_path)?;
    let signature = build_signature(&repo)?;
    let mut index = repo.index().map_err(map_git_err)?;
    let tree_oid = index.write_tree().map_err(map_git_err)?;
    let tree = repo.find_tree(tree_oid).map_err(map_git_err)?;

    let parents: Vec<git2::Commit<'_>> = match repo.head() {
        Ok(head) => vec![head.peel_to_commit().map_err(map_git_err)?],
        Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();

    let oid = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent_refs,
        )
        .map_err(map_git_err)?;
    Ok(CommitResult {
        oid: oid.to_string(),
        summary: message.lines().next().unwrap_or("").to_string(),
    })
}

fn build_signature(repo: &Repository) -> AppResult<Signature<'static>> {
    if let Ok(sig) = repo.signature() {
        return Ok(sig.to_owned());
    }
    // Fallback: walk the global config explicitly.
    let cfg = git2::Config::open_default().map_err(map_git_err)?;
    let name = cfg
        .get_string("user.name")
        .map_err(|_| AppError::Internal("user.name not configured in git".into()))?;
    let email = cfg
        .get_string("user.email")
        .map_err(|_| AppError::Internal("user.email not configured in git".into()))?;
    Signature::now(&name, &email).map_err(map_git_err)
}

pub fn clone_repo(opts: &CloneOptions) -> AppResult<CloneResult> {
    let dest = PathBuf::from(&opts.dest);
    if dest.exists() {
        return Err(AppError::Internal(format!(
            "destination already exists: {}",
            dest.display()
        )));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_cb);

    let mut fetch = FetchOptions::new();
    fetch.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch);
    if let Some(branch) = &opts.branch {
        builder.branch(branch);
    }

    let repo = builder.clone(&opts.url, &dest).map_err(map_git_err)?;
    let head = repo
        .head()
        .ok()
        .and_then(|h| h.target().map(|oid| oid.to_string()));
    Ok(CloneResult {
        dest: dest.display().to_string(),
        head,
    })
}

/// How to resolve a non-fast-forward situation when pulling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PullStrategy {
    /// Refuse to pull anything that isn't a clean fast-forward — the
    /// safe default. The UI surfaces a button to retry with `ForceReset`
    /// after explicit confirmation.
    FastForward,
    /// `git fetch && git reset --hard <upstream>` — local commits not in
    /// the upstream are silently discarded. Combine with [`stash_save`]
    /// beforehand if you also want to preserve working-tree changes.
    ForceReset,
}

pub fn pull(repo_path: &Path, strategy: PullStrategy) -> AppResult<()> {
    let repo = open(repo_path)?;
    let head = repo.head().map_err(map_git_err)?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| AppError::Internal("HEAD has no shorthand".into()))?;
    let mut local_branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .map_err(map_git_err)?;
    let upstream = local_branch
        .upstream()
        .map_err(|_| AppError::Internal(format!("`{branch_name}` has no upstream")))?;
    let upstream_ref_name = upstream
        .get()
        .name()
        .ok_or_else(|| AppError::Internal("upstream ref name missing".into()))?
        .to_string();

    let upstream_full = upstream_ref_name.clone();
    let upstream_short = upstream
        .name()
        .ok()
        .flatten()
        .ok_or_else(|| AppError::Internal("upstream short name missing".into()))?
        .to_string();
    let (remote_name, _) = upstream_short
        .split_once('/')
        .ok_or_else(|| AppError::Internal(format!("malformed upstream `{upstream_short}`")))?;
    let mut remote = repo.find_remote(remote_name).map_err(map_git_err)?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_cb);
    let mut fetch = FetchOptions::new();
    fetch.remote_callbacks(callbacks);
    remote
        .fetch::<&str>(&[], Some(&mut fetch), None)
        .map_err(map_git_err)?;

    let upstream_ann = repo
        .find_reference(&upstream_full)
        .map_err(map_git_err)?
        .peel(git2::ObjectType::Commit)
        .map_err(map_git_err)?;
    let upstream_oid = upstream_ann.id();

    if strategy == PullStrategy::ForceReset {
        // Hard-reset to upstream. Working tree + index are wiped along
        // with any local-only commits — the caller is expected to have
        // stashed first if they wanted to keep uncommitted changes.
        let upstream_obj = repo.find_object(upstream_oid, None).map_err(map_git_err)?;
        repo.reset(&upstream_obj, ResetType::Hard, None)
            .map_err(map_git_err)?;
        return Ok(());
    }

    let analysis = repo
        .merge_analysis(&[&repo
            .find_annotated_commit(upstream_oid)
            .map_err(map_git_err)?])
        .map_err(map_git_err)?;
    if analysis.0.is_up_to_date() {
        return Ok(());
    }
    if analysis.0.is_fast_forward() {
        let local_ref = local_branch.get_mut();
        local_ref
            .set_target(upstream_oid, "fast-forward")
            .map_err(map_git_err)?;
        repo.set_head(local_ref.name().unwrap_or("HEAD"))
            .map_err(map_git_err)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(map_git_err)?;
        return Ok(());
    }
    Err(AppError::Internal(
        "non-fast-forward pull — try Force pull (will reset to upstream and discard local commits)"
            .into(),
    ))
}

/// Stash both staged and unstaged changes. Returns the stash OID.
/// `message` is recorded in the stash log so the user can identify it
/// later from the terminal if they want to inspect it.
pub fn stash_save(repo_path: &Path, message: &str) -> AppResult<String> {
    let mut repo = open(repo_path)?;
    let signature = build_signature(&repo)?;
    let oid = repo
        .stash_save2(&signature, Some(message), Some(StashFlags::DEFAULT))
        .map_err(map_git_err)?;
    Ok(oid.to_string())
}

/// Pop the most recent stash entry (index 0). Returns an Err with a
/// readable message if there's nothing on the stash stack.
pub fn stash_pop(repo_path: &Path) -> AppResult<()> {
    let mut repo = open(repo_path)?;
    let mut opts = StashApplyOptions::default();
    repo.stash_pop(0, Some(&mut opts)).map_err(map_git_err)
}

pub fn push(repo_path: &Path) -> AppResult<()> {
    let repo = open(repo_path)?;
    let head = repo.head().map_err(map_git_err)?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| AppError::Internal("HEAD has no shorthand".into()))?
        .to_string();
    let local_branch = repo
        .find_branch(&branch_name, git2::BranchType::Local)
        .map_err(map_git_err)?;
    let upstream = local_branch
        .upstream()
        .map_err(|_| AppError::Internal(format!("`{branch_name}` has no upstream")))?;
    let upstream_short = upstream
        .name()
        .ok()
        .flatten()
        .ok_or_else(|| AppError::Internal("upstream short name missing".into()))?
        .to_string();
    let (remote_name, remote_branch) = upstream_short
        .split_once('/')
        .ok_or_else(|| AppError::Internal(format!("malformed upstream `{upstream_short}`")))?;
    let mut remote = repo.find_remote(remote_name).map_err(map_git_err)?;

    let refspec = format!("refs/heads/{branch_name}:refs/heads/{remote_branch}");
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(credentials_cb);
    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(callbacks);
    remote
        .push(&[refspec.as_str()], Some(&mut push_opts))
        .map_err(map_git_err)
}

fn open(path: &Path) -> AppResult<Repository> {
    Repository::discover(path).map_err(|e| {
        AppError::Internal(format!(
            "not a git repo at {}: {}",
            path.display(),
            e.message()
        ))
    })
}

fn credentials_cb(
    url: &str,
    username_from_url: Option<&str>,
    allowed: CredentialType,
) -> Result<Cred, git2::Error> {
    if allowed.contains(CredentialType::SSH_KEY) {
        if let Some(username) = username_from_url {
            return Cred::ssh_key_from_agent(username);
        }
        return Cred::ssh_key_from_agent("git");
    }
    if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
        if let Ok(cfg) = git2::Config::open_default() {
            if let Ok(cred) = Cred::credential_helper(&cfg, url, username_from_url) {
                return Ok(cred);
            }
        }
    }
    if allowed.contains(CredentialType::DEFAULT) {
        return Cred::default();
    }
    Err(git2::Error::from_str(
        "no credentials available — set up an ssh-agent or the system git credential helper",
    ))
}

fn map_git_err(e: git2::Error) -> AppError {
    AppError::Internal(format!("git: {}", e.message()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use tempfile::TempDir;

    fn init_repo() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        // Set identity locally so tests don't depend on the host's gitconfig.
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();
        let path = tmp.path().to_path_buf();
        (tmp, path)
    }

    #[test]
    fn status_reports_not_a_repo_for_plain_dir() {
        let tmp = TempDir::new().unwrap();
        let s = status(tmp.path()).unwrap();
        assert!(!s.is_repo);
        assert!(s.changes.is_empty());
        assert!(s.clean);
    }

    #[test]
    fn status_lists_untracked_files() {
        let (_t, root) = init_repo();
        fs::write(root.join("README.md"), "hi").unwrap();
        let s = status(&root).unwrap();
        assert!(s.is_repo);
        assert!(s.changes.iter().any(|c| {
            c.path == "README.md" && c.status == GitChangeStatus::Untracked && !c.staged
        }));
    }

    #[test]
    fn stage_then_status_marks_index_new() {
        let (_t, root) = init_repo();
        fs::write(root.join("a.txt"), "x").unwrap();
        stage(&root, &["a.txt".into()]).unwrap();
        let s = status(&root).unwrap();
        assert!(s
            .changes
            .iter()
            .any(|c| { c.path == "a.txt" && c.status == GitChangeStatus::New && c.staged }));
    }

    #[test]
    fn commit_creates_initial_commit_then_status_is_clean() {
        let (_t, root) = init_repo();
        fs::write(root.join("a.txt"), "x").unwrap();
        stage(&root, &["a.txt".into()]).unwrap();
        let r = commit(&root, "first").unwrap();
        assert_eq!(r.summary, "first");
        let s = status(&root).unwrap();
        assert!(s.clean);
        // Default branch name depends on init.defaultBranch in the host
        // gitconfig — accept either of the two common choices.
        assert!(matches!(s.branch.as_deref(), Some("main") | Some("master")));
    }

    #[test]
    fn commit_refuses_empty_message() {
        let (_t, root) = init_repo();
        fs::write(root.join("a.txt"), "x").unwrap();
        stage(&root, &["a.txt".into()]).unwrap();
        let err = commit(&root, "   ").unwrap_err();
        match err {
            AppError::Internal(m) => assert!(m.contains("empty")),
            _ => panic!("expected Internal"),
        }
    }

    #[test]
    fn unstage_removes_from_index_when_no_head() {
        let (_t, root) = init_repo();
        fs::write(root.join("a.txt"), "x").unwrap();
        stage(&root, &["a.txt".into()]).unwrap();
        unstage(&root, &["a.txt".into()]).unwrap();
        let s = status(&root).unwrap();
        assert!(s
            .changes
            .iter()
            .any(|c| { c.path == "a.txt" && c.status == GitChangeStatus::Untracked && !c.staged }));
        assert!(!s.changes.iter().any(|c| c.staged));
    }
}
