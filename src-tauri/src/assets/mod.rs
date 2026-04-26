//! Asset management (M7).
//!
//! `import`, `list` and `delete` files that live alongside content. The
//! frontend uses [`AssetContext`] to tell us *where* a dropped file
//! should land:
//!
//! - `Bundle { content_id }` → next to the index file of a leaf / branch
//!   bundle, so Hugo's `Resources` API can reach them and the markdown
//!   body can use a relative `![](image.jpg)` link.
//! - `Static { subpath }`     → `<site>/static/<subpath>/`. Served as-is.
//! - `Assets { subpath }`     → `<site>/assets/<subpath>/`. Routed through
//!   Hugo Pipes (SCSS, JS bundling, image processing).
//!
//! All paths are sandboxed under the site root before any IO touches
//! disk (§6.9).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::content::classify::{is_index_filename, ContentKind};
use crate::error::{AppError, AppResult};

/// Coarse classification used by the UI to pick an icon and decide
/// whether to show an inline thumbnail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Image,
    Script,
    Style,
    Document,
    Other,
}

impl AssetKind {
    pub fn from_path(path: &Path) -> Self {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "avif" | "ico" | "bmp" | "tif"
            | "tiff" => Self::Image,
            "js" | "ts" | "mjs" | "cjs" | "tsx" | "jsx" => Self::Script,
            "css" | "scss" | "sass" | "less" => Self::Style,
            "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "csv" => Self::Document,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum AssetContext {
    /// Co-locate the file inside the bundle directory of `content_id`.
    /// `content_id` is the absolute path to the bundle's index file
    /// (matches `ContentSummary.path` for a Branch / Leaf bundle).
    Bundle {
        #[serde(rename = "contentId")]
        #[specta(rename = "contentId")]
        content_id: String,
    },
    /// `<site>/static/<subpath>`. Empty subpath = static root.
    Static { subpath: String },
    /// `<site>/assets/<subpath>`. Empty subpath = assets root.
    Assets { subpath: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AssetRef {
    /// Stable id: site-relative path with forward slashes.
    pub id: String,
    pub name: String,
    /// Absolute path on disk.
    pub path: String,
    /// Relative path from the editor's reference point — for a Bundle
    /// import that's just the file name (suitable for `![]()` markdown);
    /// for Static / Assets it's the path the file would have when Hugo
    /// renders the page (`/<subpath>/<name>` for static).
    pub relative_link: String,
    pub kind: AssetKind,
    pub size: u64,
    /// Friendly label for the UI ("posts/hello/", "static/img/", …).
    pub context_label: String,
}

/// Copy `source` into the location implied by `context`. The destination
/// directory is created on demand. If a file with the same name already
/// exists, the imported file gets a `-1`, `-2`, … suffix so we never
/// silently overwrite the user's data.
pub fn import(site_root: &Path, source: &Path, context: &AssetContext) -> AppResult<AssetRef> {
    if !source.is_file() {
        return Err(AppError::Io(format!(
            "source is not a file: {}",
            source.display()
        )));
    }

    let (dest_dir, link_prefix, label) = resolve_context(site_root, context)?;
    sandbox_check(site_root, &dest_dir)?;

    std::fs::create_dir_all(&dest_dir)?;
    let original_name = source
        .file_name()
        .ok_or_else(|| AppError::Io("source has no file name".into()))?;
    let dest_name = unique_name(&dest_dir, original_name);
    let dest_path = dest_dir.join(&dest_name);
    sandbox_check(site_root, &dest_path)?;

    std::fs::copy(source, &dest_path)?;

    let metadata = std::fs::metadata(&dest_path)?;

    let lp = link_prefix.trim_start_matches('/');
    let rel_link = match context {
        AssetContext::Bundle { .. } => dest_name.to_string_lossy().to_string(),
        AssetContext::Static { .. } => {
            if lp.is_empty() {
                format!("/{}", dest_name.to_string_lossy())
            } else {
                format!("/{}/{}", lp, dest_name.to_string_lossy())
            }
        }
        AssetContext::Assets { .. } => {
            // assets/ is a Hugo Pipes input, not a public URL — surface
            // the source-tree path so the user can hand-edit a `resources.Get`
            // call if needed.
            if lp.is_empty() {
                dest_name.to_string_lossy().to_string()
            } else {
                format!("{}/{}", lp, dest_name.to_string_lossy())
            }
        }
    };

    Ok(AssetRef {
        id: site_relative_id(site_root, &dest_path),
        name: dest_name.to_string_lossy().to_string(),
        path: dest_path.display().to_string(),
        relative_link: rel_link,
        kind: AssetKind::from_path(&dest_path),
        size: metadata.len(),
        context_label: label,
    })
}

/// Maximum directory depth walked by [`list_static`] / [`list_assets`].
/// Hugo sites with deep nested asset trees (16+) are extremely rare, and
/// capping protects against pathological symlink loops without needing
/// per-directory loop detection.
const MEDIA_WALK_MAX_DEPTH: usize = 16;

/// Enumerate assets associated with `content_id` (the bundle's index
/// path) — only non-content sibling files. With `content_id == None`
/// this is intentionally empty for v1; a global "all assets" view is
/// future work.
pub fn list(site_root: &Path, content_id: Option<&str>) -> AppResult<Vec<AssetRef>> {
    let Some(content_id) = content_id else {
        return Ok(vec![]);
    };
    let bundle_dir = bundle_dir_for(site_root, content_id)?;
    sandbox_check(site_root, &bundle_dir)?;

    let mut out = Vec::new();
    if !bundle_dir.is_dir() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(&bundle_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let p = entry.path();
        let name = match p.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if is_index_filename(&name) {
            continue;
        }
        if name.starts_with('.') {
            continue;
        }
        let metadata = std::fs::metadata(&p)?;
        out.push(AssetRef {
            id: site_relative_id(site_root, &p),
            name: name.clone(),
            path: p.display().to_string(),
            relative_link: name,
            kind: AssetKind::from_path(&p),
            size: metadata.len(),
            context_label: bundle_label(&bundle_dir, site_root),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Recursively walk `<site>/static/` and return every regular file as an
/// [`AssetRef`] suitable for the media library. The `relative_link`
/// uses Hugo's URL convention (`/<rel>` from the static root) so the
/// frontend can drop the string straight into markdown.
pub fn list_static(site_root: &Path) -> AppResult<Vec<AssetRef>> {
    let root = site_root.join("static");
    list_under(site_root, &root, |rel| format!("/{rel}"))
}

/// Recursively walk `<site>/assets/`. The `relative_link` is the path
/// relative to `assets/` — what `resources.Get` takes. Surfacing it as
/// a media item is intentional even though there's no auto-link the
/// editor can insert (assets need a Hugo Pipes call); browsing /
/// previewing / deleting is still useful.
pub fn list_assets(site_root: &Path) -> AppResult<Vec<AssetRef>> {
    let root = site_root.join("assets");
    list_under(site_root, &root, |rel| rel.to_string())
}

fn list_under(
    site_root: &Path,
    root: &Path,
    rel_to_link: impl Fn(&str) -> String,
) -> AppResult<Vec<AssetRef>> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return Ok(out);
    }
    sandbox_check(site_root, root)?;
    walk_collect(site_root, root, root, 0, &rel_to_link, &mut out)?;
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn walk_collect(
    site_root: &Path,
    walk_root: &Path,
    dir: &Path,
    depth: usize,
    rel_to_link: &dyn Fn(&str) -> String,
    out: &mut Vec<AssetRef>,
) -> AppResult<()> {
    if depth > MEDIA_WALK_MAX_DEPTH {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let p = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk_collect(site_root, walk_root, &p, depth + 1, rel_to_link, out)?;
            continue;
        }
        if !ft.is_file() {
            continue;
        }
        let metadata = entry.metadata()?;
        let rel_under_root = p
            .strip_prefix(walk_root)
            .map(|r| r.display().to_string().replace('\\', "/"))
            .unwrap_or_else(|_| name_str.to_string());
        let context_label = match p
            .parent()
            .and_then(|d| d.strip_prefix(walk_root).ok())
            .map(|r| r.display().to_string().replace('\\', "/"))
        {
            Some(s) if s.is_empty() => format!(
                "{}/",
                walk_root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
            Some(s) => format!(
                "{}/{}/",
                walk_root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                s
            ),
            None => "".into(),
        };
        out.push(AssetRef {
            id: site_relative_id(site_root, &p),
            name: name_str.into_owned(),
            path: p.display().to_string(),
            relative_link: rel_to_link(&rel_under_root),
            kind: AssetKind::from_path(&p),
            size: metadata.len(),
            context_label,
        });
    }
    Ok(())
}

pub fn delete(site_root: &Path, asset_id: &str) -> AppResult<()> {
    let abs = site_root.join(asset_id);
    sandbox_check(site_root, &abs)?;
    if !abs.is_file() {
        return Err(AppError::Io(format!("asset not found: {}", abs.display())));
    }
    // Refuse to delete content files even if the caller asks.
    let name = abs.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    if is_index_filename(name) {
        return Err(AppError::Internal(format!(
            "refusing to delete a content index file: {name}"
        )));
    }
    std::fs::remove_file(&abs)?;
    Ok(())
}

fn resolve_context(
    site_root: &Path,
    context: &AssetContext,
) -> AppResult<(PathBuf, String, String)> {
    match context {
        AssetContext::Bundle { content_id } => {
            let dir = bundle_dir_for(site_root, content_id)?;
            let label = bundle_label(&dir, site_root);
            Ok((dir, String::new(), label))
        }
        AssetContext::Static { subpath } => {
            let cleaned = clean_subpath(subpath);
            let dir = site_root.join("static").join(&cleaned);
            let label = if cleaned.is_empty() {
                "static/".into()
            } else {
                format!("static/{cleaned}/")
            };
            Ok((dir, cleaned, label))
        }
        AssetContext::Assets { subpath } => {
            let cleaned = clean_subpath(subpath);
            let dir = site_root.join("assets").join(&cleaned);
            let label = if cleaned.is_empty() {
                "assets/".into()
            } else {
                format!("assets/{cleaned}/")
            };
            Ok((dir, cleaned, label))
        }
    }
}

fn clean_subpath(s: &str) -> String {
    s.trim_matches('/').replace('\\', "/").to_string()
}

fn bundle_dir_for(site_root: &Path, content_id: &str) -> AppResult<PathBuf> {
    let p = PathBuf::from(content_id);
    // content_id may be either the bundle dir or the bundle's index file.
    let dir = if p.is_dir() {
        p
    } else {
        p.parent()
            .ok_or_else(|| AppError::Io(format!("bundle path has no parent: {content_id}")))?
            .to_path_buf()
    };
    sandbox_check(site_root, &dir)?;
    Ok(dir)
}

fn bundle_label(dir: &Path, site_root: &Path) -> String {
    let content_root = site_root.join("content");
    let label = match dir.strip_prefix(&content_root) {
        Ok(rel) => format!("content/{}/", rel.display()),
        Err(_) => dir.display().to_string(),
    };
    label.replace('\\', "/")
}

fn site_relative_id(site_root: &Path, abs: &Path) -> String {
    abs.strip_prefix(site_root)
        .map(|rel| rel.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|_| abs.display().to_string())
}

fn unique_name(dir: &Path, name: &std::ffi::OsStr) -> std::ffi::OsString {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return name.to_os_string();
    }
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    for n in 1..=999 {
        let probe = format!("{stem}-{n}{ext}");
        if !dir.join(&probe).exists() {
            return std::ffi::OsString::from(probe);
        }
    }
    // Pathological collision: append a random nonce.
    let nonce: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    std::ffi::OsString::from(format!("{stem}-{nonce}{ext}"))
}

fn sandbox_check(site_root: &Path, candidate: &Path) -> AppResult<()> {
    let resolved = match candidate.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist yet (e.g. import target). Walk parents.
            let mut current = candidate.parent();
            while let Some(p) = current {
                if let Ok(c) = p.canonicalize() {
                    return ensure_within(
                        site_root,
                        &c.join(candidate.file_name().unwrap_or_default()),
                    );
                }
                current = p.parent();
            }
            return Err(AppError::PathTraversal(candidate.display().to_string()));
        }
    };
    ensure_within(site_root, &resolved)
}

fn ensure_within(site_root: &Path, candidate: &Path) -> AppResult<()> {
    let canonical_root = site_root
        .canonicalize()
        .unwrap_or_else(|_| site_root.to_path_buf());
    if candidate.starts_with(&canonical_root) {
        Ok(())
    } else {
        Err(AppError::PathTraversal(candidate.display().to_string()))
    }
}

/// Return the bundle kind (Branch / Leaf / something else) of a path.
/// Surfaced so the frontend can mark "drop here" as legal only for
/// bundle contexts. Currently unused inside this module — exposed for
/// the command layer.
#[allow(dead_code)]
pub fn bundle_kind(path: &Path) -> Option<ContentKind> {
    let dir = if path.is_dir() {
        Some(path.to_path_buf())
    } else {
        path.parent().map(Path::to_path_buf)
    };
    dir.map(|d| crate::content::classify::directory_kind(&d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn site() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        fs::write(root.join("hugo.toml"), "title = \"X\"\n").unwrap();
        (tmp, root)
    }

    fn write_source(tmp: &TempDir, name: &str, body: &[u8]) -> PathBuf {
        let p = tmp.path().join(name);
        fs::write(&p, body).unwrap();
        p
    }

    #[test]
    fn import_to_bundle_copies_and_returns_relative_link() {
        let (tmp, root) = site();
        let bundle = root.join("content/posts/hello");
        fs::create_dir_all(&bundle).unwrap();
        let index = bundle.join("index.md");
        fs::write(&index, "---\ntitle: Hello\n---\n").unwrap();
        let src = write_source(&tmp, "cover.jpg", b"binary");
        let r = import(
            &root,
            &src,
            &AssetContext::Bundle {
                content_id: index.display().to_string(),
            },
        )
        .unwrap();
        assert_eq!(r.name, "cover.jpg");
        assert_eq!(r.relative_link, "cover.jpg");
        assert_eq!(r.kind, AssetKind::Image);
        assert!(bundle.join("cover.jpg").is_file());
    }

    #[test]
    fn import_collision_appends_suffix() {
        let (tmp, root) = site();
        let bundle = root.join("content/posts/hello");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("index.md"), "---\ntitle: X\n---\n").unwrap();
        fs::write(bundle.join("cover.jpg"), b"existing").unwrap();
        let src = write_source(&tmp, "cover.jpg", b"new");
        let r = import(
            &root,
            &src,
            &AssetContext::Bundle {
                content_id: bundle.join("index.md").display().to_string(),
            },
        )
        .unwrap();
        assert_eq!(r.name, "cover-1.jpg");
        assert!(bundle.join("cover.jpg").is_file());
        assert!(bundle.join("cover-1.jpg").is_file());
    }

    #[test]
    fn import_to_static_uses_url_style_link() {
        let (tmp, root) = site();
        let src = write_source(&tmp, "logo.png", b"x");
        let r = import(
            &root,
            &src,
            &AssetContext::Static {
                subpath: "img".into(),
            },
        )
        .unwrap();
        assert_eq!(r.relative_link, "/img/logo.png");
        assert!(root.join("static/img/logo.png").is_file());
    }

    #[test]
    fn import_to_assets_creates_pipeable_path() {
        let (tmp, root) = site();
        let src = write_source(&tmp, "main.scss", b"$color: red;");
        let r = import(
            &root,
            &src,
            &AssetContext::Assets {
                subpath: "scss".into(),
            },
        )
        .unwrap();
        assert_eq!(r.kind, AssetKind::Style);
        assert_eq!(r.relative_link, "scss/main.scss");
        assert!(root.join("assets/scss/main.scss").is_file());
    }

    #[test]
    fn import_rejects_traversal_in_subpath() {
        let (tmp, root) = site();
        let src = write_source(&tmp, "leak.txt", b"x");
        let r = import(
            &root,
            &src,
            &AssetContext::Static {
                subpath: "../../escape".into(),
            },
        );
        assert!(matches!(r, Err(AppError::PathTraversal(_))));
    }

    #[test]
    fn list_returns_only_non_index_siblings() {
        let (_t, root) = site();
        let bundle = root.join("content/posts/hello");
        fs::create_dir_all(&bundle).unwrap();
        let index = bundle.join("index.md");
        fs::write(&index, "---\ntitle: X\n---\n").unwrap();
        fs::write(bundle.join("cover.jpg"), b"x").unwrap();
        fs::write(bundle.join("notes.txt"), b"x").unwrap();
        fs::write(bundle.join(".DS_Store"), b"x").unwrap();
        let assets = list(&root, Some(&index.display().to_string())).unwrap();
        let names: Vec<_> = assets.iter().map(|a| a.name.clone()).collect();
        assert_eq!(
            names,
            vec!["cover.jpg".to_string(), "notes.txt".to_string()]
        );
    }

    #[test]
    fn delete_refuses_index_files() {
        let (_t, root) = site();
        let bundle = root.join("content/posts/hello");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("index.md"), "x").unwrap();
        let err = delete(&root, "content/posts/hello/index.md").unwrap_err();
        match err {
            AppError::Internal(msg) => assert!(msg.contains("index")),
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn list_static_walks_recursively_and_emits_url_links() {
        let (_t, root) = site();
        let img_dir = root.join("static/img/posts");
        fs::create_dir_all(&img_dir).unwrap();
        fs::write(root.join("static/favicon.ico"), b"x").unwrap();
        fs::write(img_dir.join("hero.jpg"), b"x").unwrap();
        fs::write(img_dir.join(".DS_Store"), b"junk").unwrap();
        let items = list_static(&root).unwrap();
        let names: Vec<_> = items.iter().map(|a| a.name.clone()).collect();
        // Order is by id (forward-slash site-relative).
        assert_eq!(names, vec!["favicon.ico".to_string(), "hero.jpg".to_string()]);
        let hero = items.iter().find(|a| a.name == "hero.jpg").unwrap();
        assert_eq!(hero.relative_link, "/img/posts/hero.jpg");
        assert_eq!(hero.context_label, "static/img/posts/");
        let fav = items.iter().find(|a| a.name == "favicon.ico").unwrap();
        assert_eq!(fav.relative_link, "/favicon.ico");
        assert_eq!(fav.context_label, "static/");
    }

    #[test]
    fn list_static_returns_empty_when_directory_missing() {
        let (_t, root) = site();
        let items = list_static(&root).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn list_assets_uses_pipeable_relative_path() {
        let (_t, root) = site();
        let dir = root.join("assets/scss");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("main.scss"), b"$x: 1;").unwrap();
        let items = list_assets(&root).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].relative_link, "scss/main.scss");
    }

    #[test]
    fn delete_removes_the_file() {
        let (_t, root) = site();
        let bundle = root.join("content/posts/hello");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("index.md"), "x").unwrap();
        fs::write(bundle.join("cover.jpg"), b"x").unwrap();
        delete(&root, "content/posts/hello/cover.jpg").unwrap();
        assert!(!bundle.join("cover.jpg").exists());
    }
}
