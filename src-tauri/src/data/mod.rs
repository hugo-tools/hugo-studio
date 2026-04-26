//! Data file management — Hugo's `data/*` tree.
//!
//! Hugo accepts TOML / YAML / JSON / CSV here; we expose a thin
//! list / read-text / write-text API and let the frontend handle the
//! parsing/editing UX (a grid for CSV, syntax-highlighted source for
//! JSON, future formats wired in the same way).
//!
//! All paths are sandboxed under the site root before any IO, mirroring
//! what `assets::sandbox_check` does (§6.9).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{AppError, AppResult};

/// Coarse classification of a data file based on extension. The
/// frontend uses this to choose the editor (grid vs source).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DataFormat {
    Csv,
    Json,
    Yaml,
    Toml,
    Other,
}

impl DataFormat {
    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("csv") => Self::Csv,
            // GeoJSON is JSON with a specific schema — Hugo parses it as
            // JSON when loaded under data/. Surface it as Json so the
            // existing editor opens it.
            Some("json") | Some("geojson") => Self::Json,
            Some("yaml") | Some("yml") => Self::Yaml,
            Some("toml") => Self::Toml,
            _ => Self::Other,
        }
    }
}

/// Lower-cased extensions accepted by the OS-drop importer. Anything
/// else is rejected up front so users don't accidentally drag random
/// binary files into `data/` and break Hugo at build time.
pub const IMPORT_EXTENSIONS: &[&str] = &["csv", "json", "geojson", "yaml", "yml", "toml"];

/// Maximum walk depth into `data/`. Hugo recursion is unbounded but
/// real-world sites don't go anywhere near this; cap protects against
/// pathological symlink loops without per-directory loop detection.
const DATA_WALK_MAX_DEPTH: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DataFile {
    /// Stable id: site-relative path with forward slashes.
    pub id: String,
    /// File name including extension.
    pub name: String,
    /// Path under `data/` (forward slashes), without the `data/`
    /// prefix — useful for grouping in the UI sidebar.
    pub rel_path: String,
    /// Absolute path on disk.
    pub path: String,
    pub format: DataFormat,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DataFileContent {
    pub format: DataFormat,
    /// Raw UTF-8 contents — the frontend parses CSV / JSON itself so
    /// the editor stays in charge of representation (and so we don't
    /// have to round-trip a structured form back through serde).
    pub text: String,
}

pub fn list(site_root: &Path) -> AppResult<Vec<DataFile>> {
    let root = site_root.join("data");
    let mut out = Vec::new();
    if !root.is_dir() {
        return Ok(out);
    }
    sandbox_check(site_root, &root)?;
    walk(site_root, &root, &root, 0, &mut out)?;
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn read_text(site_root: &Path, rel_path: &str) -> AppResult<DataFileContent> {
    let abs = resolve_under_data(site_root, rel_path)?;
    let text = std::fs::read_to_string(&abs)?;
    Ok(DataFileContent {
        format: DataFormat::from_path(&abs),
        text,
    })
}

pub fn write_text(site_root: &Path, rel_path: &str, text: &str) -> AppResult<DataFileContent> {
    let abs = resolve_under_data(site_root, rel_path)?;
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    atomic_write(&abs, text.as_bytes())?;
    Ok(DataFileContent {
        format: DataFormat::from_path(&abs),
        text: text.to_string(),
    })
}

/// Create a new empty data file. Refuses to overwrite an existing
/// file — the caller picks the path. Seeds CSV with a single empty
/// row so the grid editor opens with at least one cell to click on.
pub fn create(site_root: &Path, rel_path: &str) -> AppResult<DataFile> {
    let abs = resolve_under_data(site_root, rel_path)?;
    if abs.exists() {
        return Err(AppError::Io(format!(
            "data file already exists: {}",
            abs.display()
        )));
    }
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let format = DataFormat::from_path(&abs);
    let seed = match format {
        DataFormat::Csv => "column1,column2\n,\n",
        DataFormat::Json => "{}\n",
        DataFormat::Yaml => "",
        DataFormat::Toml => "",
        DataFormat::Other => "",
    };
    atomic_write(&abs, seed.as_bytes())?;
    let metadata = std::fs::metadata(&abs)?;
    let id = site_relative_id(site_root, &abs);
    let name = abs
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    let rel_under_data = abs
        .strip_prefix(site_root.join("data"))
        .map(|r| r.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|_| name.clone());
    Ok(DataFile {
        id,
        name,
        rel_path: rel_under_data,
        path: abs.display().to_string(),
        format,
        size: metadata.len(),
    })
}

/// Copy `source` into `<site>/data/`, suffixing the file name with
/// `-1`, `-2`, … if a file with the same name already exists so we
/// never silently overwrite the user's data. Refuses any extension
/// outside [`IMPORT_EXTENSIONS`].
pub fn import(site_root: &Path, source: &Path) -> AppResult<DataFile> {
    if !source.is_file() {
        return Err(AppError::Io(format!(
            "source is not a file: {}",
            source.display()
        )));
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !IMPORT_EXTENSIONS.contains(&ext.as_str()) {
        return Err(AppError::Io(format!(
            "unsupported data extension: {ext} (allowed: {})",
            IMPORT_EXTENSIONS.join(", ")
        )));
    }

    let data_dir = site_root.join("data");
    std::fs::create_dir_all(&data_dir)?;
    sandbox_check(site_root, &data_dir)?;

    let original_name = source
        .file_name()
        .ok_or_else(|| AppError::Io("source has no file name".into()))?;
    let dest_name = unique_name(&data_dir, original_name);
    let dest_path = data_dir.join(&dest_name);
    sandbox_check(site_root, &dest_path)?;

    std::fs::copy(source, &dest_path)?;
    let metadata = std::fs::metadata(&dest_path)?;
    let name = dest_name.to_string_lossy().to_string();
    Ok(DataFile {
        id: site_relative_id(site_root, &dest_path),
        name: name.clone(),
        rel_path: name,
        path: dest_path.display().to_string(),
        format: DataFormat::from_path(&dest_path),
        size: metadata.len(),
    })
}

/// Find a non-colliding file name under `dir` by suffixing `-1`,
/// `-2`, …  before the extension. Mirrors `assets::unique_name`.
fn unique_name(dir: &Path, name: &std::ffi::OsStr) -> std::ffi::OsString {
    if !dir.join(name).exists() {
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
    let nonce: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    std::ffi::OsString::from(format!("{stem}-{nonce}{ext}"))
}

pub fn delete(site_root: &Path, rel_path: &str) -> AppResult<()> {
    let abs = resolve_under_data(site_root, rel_path)?;
    if !abs.is_file() {
        return Err(AppError::Io(format!(
            "data file not found: {}",
            abs.display()
        )));
    }
    std::fs::remove_file(&abs)?;
    Ok(())
}

fn walk(
    site_root: &Path,
    walk_root: &Path,
    dir: &Path,
    depth: usize,
    out: &mut Vec<DataFile>,
) -> AppResult<()> {
    if depth > DATA_WALK_MAX_DEPTH {
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
            walk(site_root, walk_root, &p, depth + 1, out)?;
            continue;
        }
        if !ft.is_file() {
            continue;
        }
        let metadata = entry.metadata()?;
        let rel_under_data = p
            .strip_prefix(walk_root)
            .map(|r| r.display().to_string().replace('\\', "/"))
            .unwrap_or_else(|_| name_str.to_string());
        out.push(DataFile {
            id: site_relative_id(site_root, &p),
            name: name_str.into_owned(),
            rel_path: rel_under_data,
            path: p.display().to_string(),
            format: DataFormat::from_path(&p),
            size: metadata.len(),
        });
    }
    Ok(())
}

fn site_relative_id(site_root: &Path, abs: &Path) -> String {
    abs.strip_prefix(site_root)
        .map(|rel| rel.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|_| abs.display().to_string())
}

/// Resolve a `rel_path` (with forward slashes, no leading `data/`) to
/// an absolute path under `<site>/data/` and sandbox-check the result.
/// Refuses traversal at the path-component level — `sandbox_check`
/// would otherwise miss it because, when the target file doesn't
/// exist yet, the existing-ancestor canonicalisation reduces
/// `data/../escape.txt` back to `data/escape.txt`.
fn resolve_under_data(site_root: &Path, rel_path: &str) -> AppResult<PathBuf> {
    let cleaned = rel_path.trim_matches('/').replace('\\', "/");
    if cleaned.is_empty() {
        return Err(AppError::Io("empty data file path".into()));
    }
    if cleaned
        .split('/')
        .any(|c| c == ".." || c == "." || c.is_empty())
    {
        return Err(AppError::PathTraversal(rel_path.to_string()));
    }
    let candidate = site_root.join("data").join(&cleaned);
    sandbox_check(site_root, &candidate)?;
    Ok(candidate)
}

fn sandbox_check(site_root: &Path, candidate: &Path) -> AppResult<()> {
    let resolved = match candidate.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist yet (e.g. create target). Walk
            // upwards until something does, then verify the resolved
            // ancestor sits inside the site root.
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

    fn site() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        fs::write(root.join("hugo.toml"), "title = \"X\"\n").unwrap();
        (tmp, root)
    }

    #[test]
    fn list_walks_data_recursively() {
        let (_t, root) = site();
        let dir = root.join("data/products");
        fs::create_dir_all(&dir).unwrap();
        fs::write(root.join("data/site.json"), "{}").unwrap();
        fs::write(dir.join("inventory.csv"), "sku,qty\nA,1\n").unwrap();
        fs::write(dir.join(".DS_Store"), "junk").unwrap();
        let items = list(&root).unwrap();
        let names: Vec<_> = items.iter().map(|d| d.name.clone()).collect();
        // Sorted by id (forward-slash relative path) so site.json
        // comes before products/inventory.csv.
        assert_eq!(
            names,
            vec!["inventory.csv".to_string(), "site.json".to_string()]
        );
    }

    #[test]
    fn list_returns_empty_when_data_missing() {
        let (_t, root) = site();
        assert!(list(&root).unwrap().is_empty());
    }

    #[test]
    fn read_and_write_round_trip_csv() {
        let (_t, root) = site();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(root.join("data/team.csv"), "name,role\nAna,dev\n").unwrap();
        let read = read_text(&root, "team.csv").unwrap();
        assert_eq!(read.format, DataFormat::Csv);
        assert!(read.text.contains("Ana,dev"));
        write_text(&root, "team.csv", "name,role\nBob,pm\n").unwrap();
        let after = fs::read_to_string(root.join("data/team.csv")).unwrap();
        assert_eq!(after, "name,role\nBob,pm\n");
    }

    #[test]
    fn create_seeds_csv_with_a_minimal_row() {
        let (_t, root) = site();
        let created = create(&root, "stuff.csv").unwrap();
        assert_eq!(created.format, DataFormat::Csv);
        let text = fs::read_to_string(&created.path).unwrap();
        assert!(text.contains("column1,column2"));
    }

    #[test]
    fn create_refuses_to_overwrite() {
        let (_t, root) = site();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(root.join("data/x.json"), "{}").unwrap();
        assert!(create(&root, "x.json").is_err());
    }

    #[test]
    fn rejects_traversal_in_rel_path() {
        let (_t, root) = site();
        assert!(read_text(&root, "../escape.txt").is_err());
        assert!(write_text(&root, "../escape.txt", "x").is_err());
    }

    #[test]
    fn import_copies_csv_into_data_root() {
        let (tmp, root) = site();
        let src = tmp.path().join("ext-team.csv");
        fs::write(&src, "name,role\nAna,dev\n").unwrap();
        let dest = import(&root, &src).unwrap();
        assert_eq!(dest.format, DataFormat::Csv);
        assert!(root.join("data/ext-team.csv").is_file());
        assert_eq!(dest.rel_path, "ext-team.csv");
    }

    #[test]
    fn import_appends_suffix_on_collision() {
        let (tmp, root) = site();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(root.join("data/products.json"), "{\"a\":1}").unwrap();
        let src = tmp.path().join("products.json");
        fs::write(&src, "{\"a\":2}").unwrap();
        let dest = import(&root, &src).unwrap();
        assert_eq!(dest.rel_path, "products-1.json");
        assert!(root.join("data/products.json").is_file());
        assert!(root.join("data/products-1.json").is_file());
    }

    #[test]
    fn import_accepts_geojson_and_classifies_as_json() {
        let (tmp, root) = site();
        let src = tmp.path().join("zones.geojson");
        fs::write(&src, "{\"type\":\"FeatureCollection\"}").unwrap();
        let dest = import(&root, &src).unwrap();
        assert_eq!(dest.format, DataFormat::Json);
        assert_eq!(dest.rel_path, "zones.geojson");
    }

    #[test]
    fn import_refuses_unsupported_extension() {
        let (tmp, root) = site();
        let src = tmp.path().join("blob.exe");
        fs::write(&src, b"\x00\x01").unwrap();
        let err = import(&root, &src).unwrap_err();
        match err {
            AppError::Io(msg) => assert!(msg.contains("unsupported data extension")),
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn delete_removes_the_file() {
        let (_t, root) = site();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(root.join("data/x.json"), "{}").unwrap();
        delete(&root, "x.json").unwrap();
        assert!(!root.join("data/x.json").exists());
    }
}
