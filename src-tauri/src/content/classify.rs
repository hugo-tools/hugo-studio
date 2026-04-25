use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

/// Hugo's three structural content kinds, plus the synthetic `Section`
/// for directories that contain content but no `_index.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Section,
    BranchBundle,
    LeafBundle,
    SinglePage,
}

/// File extensions Hugo treats as page content. Order is informational —
/// the matcher is a `contains` check.
pub const CONTENT_EXTENSIONS: &[&str] = &["md", "markdown", "html", "htm"];

/// True when `path` is a regular file with one of the recognised content
/// extensions. Cheap — no file IO beyond `is_file()`.
pub fn is_content_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            CONTENT_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

/// Classify a directory based on which special index file (if any) it
/// contains. Caller is expected to pass an actual directory path.
pub fn directory_kind(dir: &Path) -> ContentKind {
    for ext in CONTENT_EXTENSIONS {
        if dir.join(format!("_index.{ext}")).is_file() {
            return ContentKind::BranchBundle;
        }
    }
    for ext in CONTENT_EXTENSIONS {
        if dir.join(format!("index.{ext}")).is_file() {
            return ContentKind::LeafBundle;
        }
    }
    ContentKind::Section
}

/// Returns the name of the special index file for branch / leaf bundles
/// — used by the scanner to attribute the bundle's metadata to the file
/// the user would actually edit.
pub fn index_file_for(dir: &Path, kind: ContentKind) -> Option<std::path::PathBuf> {
    let prefix = match kind {
        ContentKind::BranchBundle => "_index",
        ContentKind::LeafBundle => "index",
        _ => return None,
    };
    for ext in CONTENT_EXTENSIONS {
        let candidate = dir.join(format!("{prefix}.{ext}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// True when the file *name* (not the full path) marks it as a bundle
/// index — used by the scanner to skip enumerating index files as their
/// own SinglePage entries.
pub fn is_index_filename(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    for ext in CONTENT_EXTENSIONS {
        if lower == format!("_index.{ext}") || lower == format!("index.{ext}") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn empty_directory_is_a_section() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(directory_kind(tmp.path()), ContentKind::Section);
    }

    #[test]
    fn underscore_index_marks_branch_bundle() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("_index.md"), "").unwrap();
        assert_eq!(directory_kind(tmp.path()), ContentKind::BranchBundle);
    }

    #[test]
    fn plain_index_marks_leaf_bundle() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("index.md"), "").unwrap();
        assert_eq!(directory_kind(tmp.path()), ContentKind::LeafBundle);
    }

    #[test]
    fn underscore_wins_over_plain_index_when_both_present() {
        // Hugo would never normally emit both, but be defensive.
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("_index.md"), "").unwrap();
        fs::write(tmp.path().join("index.md"), "").unwrap();
        assert_eq!(directory_kind(tmp.path()), ContentKind::BranchBundle);
    }

    #[test]
    fn html_index_also_counts() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("_index.html"), "").unwrap();
        assert_eq!(directory_kind(tmp.path()), ContentKind::BranchBundle);
    }

    #[test]
    fn index_filename_detector() {
        assert!(is_index_filename("_index.md"));
        assert!(is_index_filename("index.html"));
        assert!(!is_index_filename("hello.md"));
        assert!(!is_index_filename("indexes.md"));
    }
}
