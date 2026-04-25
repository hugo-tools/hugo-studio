use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::content::classify::{
    directory_kind, index_file_for, is_content_file, is_index_filename, ContentKind,
};
use crate::content::frontmatter::{peek, FrontMatterSummary};
use crate::content::language::{
    detect as detect_language, split_filename_lang, LanguageInfo, LanguageStrategy,
};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentSummary {
    /// Stable ID = path relative to the language-resolved content root,
    /// using forward slashes. Distinct translations of the same logical
    /// page share the same `id` but have different `language`.
    pub id: String,
    pub kind: ContentKind,
    /// Top-level section under content/ (e.g. "posts", "docs"). `None`
    /// for items that sit at the very top (`_index.md` of the site).
    pub section: Option<String>,
    pub language: String,
    /// Absolute filesystem path to the file the user would edit.
    /// For bundles this is the index file; for sections it's the
    /// directory itself (no editable target until M4 introduces virtual
    /// section editing).
    pub path: String,
    pub title: Option<String>,
    pub draft: bool,
    pub date: Option<String>,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentScanResult {
    pub language_info: LanguageInfo,
    pub items: Vec<ContentSummary>,
}

/// Walk `<root>/content` and produce a flat list of every page / bundle /
/// section we found, tagged with its language. Frontend rebuilds the tree
/// from the flat list so it can re-group as the user toggles languages
/// without an extra IPC roundtrip.
pub fn scan(site_root: &Path, merged_config: &serde_json::Value) -> AppResult<ContentScanResult> {
    let content_root = site_root.join("content");
    if !content_root.is_dir() {
        return Ok(ContentScanResult {
            language_info: detect_language(merged_config, None),
            items: vec![],
        });
    }

    let language_info = detect_language(merged_config, Some(&content_root));
    let lang_codes: Vec<String> = language_info
        .languages
        .iter()
        .map(|l| l.code.clone())
        .collect();

    let mut items = Vec::new();

    match (language_info.strategy, lang_codes.is_empty()) {
        (LanguageStrategy::Directory, false) => {
            for code in &lang_codes {
                let lang_root = content_root.join(code);
                if lang_root.is_dir() {
                    walk(&lang_root, &lang_root, code, &lang_codes, &mut items);
                }
            }
        }
        _ => {
            let default_lang = language_info.default_language.clone();
            walk(
                &content_root,
                &content_root,
                &default_lang,
                &lang_codes,
                &mut items,
            );
        }
    }

    Ok(ContentScanResult {
        language_info,
        items,
    })
}

fn walk(
    base: &Path,
    current: &Path,
    default_lang: &str,
    lang_codes: &[String],
    out: &mut Vec<ContentSummary>,
) {
    if current.is_dir() {
        let kind = directory_kind(current);
        // Don't emit a synthetic entry for the language root itself, only for
        // nested directories.
        if current != base {
            let summary = directory_summary(base, current, kind, default_lang);
            out.push(summary);
        }

        if kind == ContentKind::LeafBundle {
            // Leaf bundles consume their directory: assets / sub-files inside
            // are not separate pages. M7 surfaces them as assets.
            return;
        }

        let entries = match std::fs::read_dir(current) {
            Ok(it) => it,
            Err(_) => return,
        };
        let mut child_paths: Vec<PathBuf> =
            entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        child_paths.sort();
        for child in child_paths {
            walk(base, &child, default_lang, lang_codes, out);
        }
        return;
    }

    if !is_content_file(current) {
        return;
    }
    let name = match current.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return,
    };
    if is_index_filename(name) {
        return; // owned by the parent directory's bundle entry
    }

    out.push(file_summary(base, current, default_lang, lang_codes));
}

fn directory_summary(
    base: &Path,
    dir: &Path,
    kind: ContentKind,
    default_lang: &str,
) -> ContentSummary {
    let rel = dir.strip_prefix(base).unwrap_or(dir);
    let id = rel_to_id(rel);
    let depth = rel.components().count() as u32;
    let section = top_section(rel);

    let editable = index_file_for(dir, kind);
    let path = editable.as_deref().unwrap_or(dir).display().to_string();

    let summary = if let Some(idx) = editable.as_deref() {
        std::fs::read_to_string(idx)
            .map(|s| peek(&s))
            .unwrap_or_default()
    } else {
        FrontMatterSummary::default()
    };

    let title = summary.title.or_else(|| {
        dir.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    });

    ContentSummary {
        id,
        kind,
        section,
        language: default_lang.to_string(),
        path,
        title,
        draft: summary.draft,
        date: summary.date,
        depth,
    }
}

fn file_summary(
    base: &Path,
    file: &Path,
    default_lang: &str,
    lang_codes: &[String],
) -> ContentSummary {
    let rel = file.strip_prefix(base).unwrap_or(file);
    let depth = rel.components().count() as u32;
    let section = top_section(rel);

    let summary = std::fs::read_to_string(file)
        .map(|s| peek(&s))
        .unwrap_or_default();

    let stem = file.file_stem().and_then(|n| n.to_str()).unwrap_or("");
    let (base_stem, lang_suffix) = split_filename_lang(stem, lang_codes);
    let language = lang_suffix
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_lang.to_string());

    // Build the canonical id by replacing the original stem with the
    // language-stripped one in the relative path.
    let parent_rel = rel.parent().unwrap_or_else(|| Path::new(""));
    let extension = file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    let id_path = if parent_rel.as_os_str().is_empty() {
        format!("{base_stem}{extension}")
    } else {
        let parent_str = rel_to_id(parent_rel);
        format!("{parent_str}/{base_stem}{extension}")
    };

    let title = summary.title.or_else(|| Some(base_stem.to_string()));

    ContentSummary {
        id: id_path,
        kind: ContentKind::SinglePage,
        section,
        language,
        path: file.display().to_string(),
        title,
        draft: summary.draft,
        date: summary.date,
        depth,
    }
}

fn rel_to_id(rel: &Path) -> String {
    rel.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn top_section(rel: &Path) -> Option<String> {
    rel.components().next().and_then(|c| match c {
        std::path::Component::Normal(s) => s.to_str().map(String::from),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(p: &Path, body: &str) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, body).unwrap();
    }

    #[test]
    fn no_content_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let cfg = serde_json::json!({});
        let res = scan(tmp.path(), &cfg).unwrap();
        assert!(res.items.is_empty());
    }

    #[test]
    fn classifies_section_branch_leaf_single() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        // posts/ is a section
        // posts/_index.md → branch bundle (posts becomes BranchBundle, not Section)
        touch(
            &root.join("content/posts/_index.md"),
            "---\ntitle: Posts\n---",
        );
        // posts/hello.md → single page
        touch(
            &root.join("content/posts/hello.md"),
            "---\ntitle: Hello\ndraft: true\n---\nbody",
        );
        // posts/leaf/index.md → leaf bundle
        touch(
            &root.join("content/posts/leaf/index.md"),
            "---\ntitle: Leaf\n---",
        );
        // about/ has no index → section
        touch(&root.join("content/about/team.md"), "---\ntitle: Team\n---");

        let cfg = serde_json::json!({});
        let res = scan(root, &cfg).unwrap();
        let by_id: std::collections::HashMap<_, _> =
            res.items.iter().map(|i| (i.id.clone(), i)).collect();

        assert_eq!(by_id.get("posts").unwrap().kind, ContentKind::BranchBundle);
        assert_eq!(
            by_id.get("posts/hello.md").unwrap().kind,
            ContentKind::SinglePage
        );
        assert!(by_id.get("posts/hello.md").unwrap().draft);
        assert_eq!(
            by_id.get("posts/leaf").unwrap().kind,
            ContentKind::LeafBundle
        );
        assert_eq!(by_id.get("about").unwrap().kind, ContentKind::Section);
        assert_eq!(
            by_id.get("about/team.md").unwrap().kind,
            ContentKind::SinglePage
        );
    }

    #[test]
    fn leaf_bundle_does_not_recurse_into_assets() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(
            &root.join("content/post-bundle/index.md"),
            "---\ntitle: B\n---",
        );
        touch(&root.join("content/post-bundle/cover.jpg"), "fake");
        touch(
            &root.join("content/post-bundle/notes.md"),
            "---\ntitle: notes\n---",
        );
        let res = scan(root, &serde_json::json!({})).unwrap();
        let ids: Vec<_> = res.items.iter().map(|i| &i.id).collect();
        assert!(ids.iter().any(|i| *i == "post-bundle"));
        assert!(!ids.iter().any(|i| *i == "post-bundle/notes.md"));
    }

    #[test]
    fn filename_strategy_extracts_language_suffix() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(
            &root.join("content/posts/hello.en.md"),
            "---\ntitle: Hello\n---",
        );
        touch(
            &root.join("content/posts/hello.it.md"),
            "---\ntitle: Ciao\n---",
        );
        let cfg = serde_json::json!({
            "languages": {"en": {}, "it": {}},
            "defaultContentLanguage": "en",
        });
        let res = scan(root, &cfg).unwrap();
        let mut entries: Vec<_> = res
            .items
            .iter()
            .filter(|i| i.id == "posts/hello.md")
            .collect();
        entries.sort_by_key(|i| i.language.clone());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].language, "en");
        assert_eq!(entries[1].language, "it");
        assert_eq!(entries[0].title.as_deref(), Some("Hello"));
        assert_eq!(entries[1].title.as_deref(), Some("Ciao"));
    }

    #[test]
    fn directory_strategy_walks_each_lang_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        touch(
            &root.join("content/en/posts/hello.md"),
            "---\ntitle: Hello\n---",
        );
        touch(
            &root.join("content/it/posts/hello.md"),
            "---\ntitle: Ciao\n---",
        );
        let cfg = serde_json::json!({
            "languages": {"en": {}, "it": {}},
            "defaultContentLanguageInSubdir": true,
        });
        let res = scan(root, &cfg).unwrap();
        let mut entries: Vec<_> = res
            .items
            .iter()
            .filter(|i| i.id == "posts/hello.md")
            .collect();
        entries.sort_by_key(|i| i.language.clone());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].language, "en");
        assert_eq!(entries[1].language, "it");
    }
}
