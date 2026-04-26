//! Browse / read / write source files inside the active theme.
//!
//! Hugo themes live at `<site>/themes/<name>/` and contain Go-template
//! `layouts/`, `partials/`, SCSS / JS in `assets/`, archetypes,
//! translations, and so on. This module exposes a flat enumeration plus
//! raw-text read/write so the frontend can edit any of them with the
//! right CodeMirror language.
//!
//! Sandboxing: every path is validated to live under the resolved
//! theme directory before any IO touches disk — same pattern as
//! `assets::sandbox_check` and `data::sandbox_check`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{AppError, AppResult};
use crate::theme;

/// Coarse classification of a theme file based on extension. The
/// frontend uses this to choose the editor's language extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ThemeFileFormat {
    Html,
    Css,
    Scss,
    Js,
    Ts,
    Json,
    Yaml,
    Toml,
    Markdown,
    Other,
}

impl ThemeFileFormat {
    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("html") | Some("htm") | Some("gohtml") | Some("tmpl") => Self::Html,
            Some("css") => Self::Css,
            Some("scss") | Some("sass") => Self::Scss,
            Some("js") | Some("mjs") | Some("cjs") | Some("jsx") => Self::Js,
            Some("ts") | Some("tsx") => Self::Ts,
            Some("json") => Self::Json,
            Some("yaml") | Some("yml") => Self::Yaml,
            Some("toml") => Self::Toml,
            Some("md") | Some("markdown") => Self::Markdown,
            _ => Self::Other,
        }
    }
}

/// Maximum recursion depth into the theme tree. Real themes don't go
/// deeper than 4–5 levels; the cap is just a guardrail against
/// pathological symlinks.
const THEME_WALK_MAX_DEPTH: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFile {
    /// Path relative to the theme directory, forward slashes. Stable id.
    pub rel_path: String,
    pub name: String,
    /// Top-level subdirectory the file lives under (`layouts`, `assets`,
    /// …). Empty string for files at the theme root.
    pub category: String,
    /// Absolute path on disk.
    pub path: String,
    pub format: ThemeFileFormat,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFileContent {
    pub format: ThemeFileFormat,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFilesIndex {
    pub theme_name: Option<String>,
    pub theme_path: Option<String>,
    pub files: Vec<ThemeFile>,
}

pub fn list(site_root: &Path) -> AppResult<ThemeFilesIndex> {
    let info = theme::load(site_root)?;
    let theme_path = match info.theme_path.as_deref() {
        Some(p) => PathBuf::from(p),
        None => {
            return Ok(ThemeFilesIndex {
                theme_name: info.theme_name,
                theme_path: None,
                files: vec![],
            });
        }
    };
    let mut files = Vec::new();
    if theme_path.is_dir() {
        sandbox_check(&theme_path, &theme_path)?;
        walk(&theme_path, &theme_path, 0, &mut files)?;
        files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    }
    Ok(ThemeFilesIndex {
        theme_name: info.theme_name,
        theme_path: Some(theme_path.display().to_string()),
        files,
    })
}

pub fn read_text(site_root: &Path, rel_path: &str) -> AppResult<ThemeFileContent> {
    let abs = resolve_under_theme(site_root, rel_path)?;
    let text = std::fs::read_to_string(&abs)?;
    Ok(ThemeFileContent {
        format: ThemeFileFormat::from_path(&abs),
        text,
    })
}

pub fn write_text(site_root: &Path, rel_path: &str, text: &str) -> AppResult<ThemeFileContent> {
    let abs = resolve_under_theme(site_root, rel_path)?;
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    atomic_write(&abs, text.as_bytes())?;
    Ok(ThemeFileContent {
        format: ThemeFileFormat::from_path(&abs),
        text: text.to_string(),
    })
}

fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<ThemeFile>) -> AppResult<()> {
    if depth > THEME_WALK_MAX_DEPTH {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        // Skip vendor / build artefacts that themes commonly carry.
        if matches!(
            name_str.as_ref(),
            "node_modules" | "resources" | "public" | "package-lock.json"
        ) {
            continue;
        }
        let p = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk(root, &p, depth + 1, out)?;
            continue;
        }
        if !ft.is_file() {
            continue;
        }
        let metadata = entry.metadata()?;
        let rel_under_theme = p
            .strip_prefix(root)
            .map(|r| r.display().to_string().replace('\\', "/"))
            .unwrap_or_else(|_| name_str.to_string());
        // Top-level subdir name when the file lives nested; empty for
        // files at the theme root. Using a separate `contains('/')`
        // check (instead of `.filter` on the iterator) to keep the
        // outer-scope reference explicit and the closure capture-free.
        let category = if rel_under_theme.contains('/') {
            rel_under_theme.split('/').next().unwrap_or("").to_string()
        } else {
            String::new()
        };
        out.push(ThemeFile {
            rel_path: rel_under_theme,
            name: name_str.into_owned(),
            category,
            path: p.display().to_string(),
            format: ThemeFileFormat::from_path(&p),
            size: metadata.len(),
        });
    }
    Ok(())
}

fn resolve_under_theme(site_root: &Path, rel_path: &str) -> AppResult<PathBuf> {
    let cleaned = rel_path.trim_matches('/').replace('\\', "/");
    if cleaned.is_empty() {
        return Err(AppError::Io("empty theme file path".into()));
    }
    if cleaned
        .split('/')
        .any(|c| c == ".." || c == "." || c.is_empty())
    {
        return Err(AppError::PathTraversal(rel_path.to_string()));
    }
    let info = theme::load(site_root)?;
    let theme_path = info
        .theme_path
        .ok_or_else(|| AppError::Io("no active theme configured".into()))?;
    let theme_path = PathBuf::from(theme_path);
    let candidate = theme_path.join(&cleaned);
    sandbox_check(&theme_path, &candidate)?;
    Ok(candidate)
}

fn sandbox_check(theme_root: &Path, candidate: &Path) -> AppResult<()> {
    let resolved = match candidate.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let mut current = candidate.parent();
            while let Some(p) = current {
                if let Ok(c) = p.canonicalize() {
                    return ensure_within(
                        theme_root,
                        &c.join(candidate.file_name().unwrap_or_default()),
                    );
                }
                current = p.parent();
            }
            return Err(AppError::PathTraversal(candidate.display().to_string()));
        }
    };
    ensure_within(theme_root, &resolved)
}

fn ensure_within(theme_root: &Path, candidate: &Path) -> AppResult<()> {
    let canonical_root = theme_root
        .canonicalize()
        .unwrap_or_else(|_| theme_root.to_path_buf());
    if candidate.starts_with(&canonical_root) {
        Ok(())
    } else {
        Err(AppError::PathTraversal(candidate.display().to_string()))
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bak");
    let tmp = path.with_extension(format!("{ext}.tmp"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn site_with_theme(name: &str) -> (TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        fs::write(root.join("hugo.toml"), format!("theme = \"{name}\"\n")).unwrap();
        let theme_dir = root.join("themes").join(name);
        fs::create_dir_all(&theme_dir).unwrap();
        (tmp, root, theme_dir)
    }

    #[test]
    fn list_walks_theme_recursively_and_skips_vendor_dirs() {
        let (_t, root, theme_dir) = site_with_theme("ace");
        fs::create_dir_all(theme_dir.join("layouts/_default")).unwrap();
        fs::create_dir_all(theme_dir.join("assets/scss")).unwrap();
        fs::create_dir_all(theme_dir.join("node_modules/foo")).unwrap();
        fs::write(theme_dir.join("layouts/_default/baseof.html"), "<html/>").unwrap();
        fs::write(theme_dir.join("assets/scss/main.scss"), "$x: 1;").unwrap();
        fs::write(theme_dir.join("theme.toml"), "name = \"ace\"\n").unwrap();
        fs::write(theme_dir.join("node_modules/foo/junk"), "x").unwrap();
        let idx = list(&root).unwrap();
        assert_eq!(idx.theme_name.as_deref(), Some("ace"));
        let names: Vec<_> = idx.files.iter().map(|f| f.rel_path.clone()).collect();
        assert!(names.contains(&"theme.toml".to_string()));
        assert!(names.contains(&"layouts/_default/baseof.html".to_string()));
        assert!(names.contains(&"assets/scss/main.scss".to_string()));
        assert!(!names.iter().any(|n| n.contains("node_modules")));
    }

    #[test]
    fn list_when_no_theme_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        // No `theme = …` in config.
        fs::write(root.join("hugo.toml"), "title = \"X\"\n").unwrap();
        let idx = list(&root).unwrap();
        assert!(idx.files.is_empty());
        assert!(idx.theme_path.is_none());
    }

    #[test]
    fn read_and_write_round_trip() {
        let (_t, root, theme_dir) = site_with_theme("ace");
        let p = theme_dir.join("layouts/_default/single.html");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "<article>{{ .Content }}</article>\n").unwrap();
        let read = read_text(&root, "layouts/_default/single.html").unwrap();
        assert_eq!(read.format, ThemeFileFormat::Html);
        assert!(read.text.contains(".Content"));
        write_text(&root, "layouts/_default/single.html", "<x/>\n").unwrap();
        let after = fs::read_to_string(&p).unwrap();
        assert_eq!(after, "<x/>\n");
    }

    #[test]
    fn rejects_traversal_in_rel_path() {
        let (_t, root, _theme_dir) = site_with_theme("ace");
        assert!(read_text(&root, "../../escape.txt").is_err());
        assert!(write_text(&root, "../../escape.txt", "x").is_err());
    }
}
