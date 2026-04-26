use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::config::cascade;
use crate::content::archetype::{self, Archetype};
use crate::content::create::{self as create_content, CreateOptions, CreatedContent};
use crate::content::document::{self, BodyFormat, FrontMatterFormat};
use crate::content::scan::{self, ContentScanResult};
use crate::content::schema::{infer_section_schema, FrontMatterSchema};
use crate::domain::ids::SiteId;
use crate::error::{AppError, AppResult};
use crate::hugo::detect;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn content_list(state: State<'_, AppState>, site_id: SiteId) -> AppResult<ContentScanResult> {
    let root = site_root(&state, site_id)?;
    let det = detect::detect(&root)?;
    let merged = cascade::load(&det)?.merged;
    scan::scan(&root, &merged)
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentEditPayload {
    pub format: FrontMatterFormat,
    /// Body language — drives editor mode and whether the WYSIWYG tab
    /// is offered. Inferred from extension, not from on-disk bytes.
    pub body_format: BodyFormat,
    pub front_matter: serde_json::Value,
    pub body: String,
    pub schema: FrontMatterSchema,
    /// Absolute path of the file the editor is acting on. Surfaced so the
    /// UI can show a "reveal in finder" affordance later.
    pub path: String,
}

#[tauri::command]
#[specta::specta]
pub fn content_get(
    state: State<'_, AppState>,
    site_id: SiteId,
    path: String,
) -> AppResult<ContentEditPayload> {
    let root = site_root(&state, site_id)?;
    let abs = resolve_under_root(&root, &path)?;
    let doc = document::read(&abs)?;
    let section = top_section(&root, &abs);
    let schema = infer_section_schema(&root.join("content"), section.as_deref())?.schema;
    Ok(ContentEditPayload {
        format: doc.format,
        body_format: document::body_format_for_path(&abs),
        front_matter: doc.front_matter,
        body: doc.body,
        schema,
        path: abs.display().to_string(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn content_archetypes(
    state: State<'_, AppState>,
    site_id: SiteId,
) -> AppResult<Vec<Archetype>> {
    let root = site_root(&state, site_id)?;
    archetype::list(&root)
}

#[tauri::command]
#[specta::specta]
pub fn content_create(
    state: State<'_, AppState>,
    site_id: SiteId,
    options: CreateOptions,
) -> AppResult<CreatedContent> {
    let root = site_root(&state, site_id)?;
    let det = detect::detect(&root)?;
    let merged = cascade::load(&det)?.merged;
    create_content::create(&root, &merged, &options)
}

#[tauri::command]
#[specta::specta]
pub fn content_save(
    state: State<'_, AppState>,
    site_id: SiteId,
    path: String,
    front_matter: serde_json::Value,
    body: String,
) -> AppResult<ContentEditPayload> {
    let root = site_root(&state, site_id)?;
    let abs = resolve_under_root(&root, &path)?;
    document::save(&abs, &front_matter, &body)?;
    let doc = document::read(&abs)?;
    let section = top_section(&root, &abs);
    let schema = infer_section_schema(&root.join("content"), section.as_deref())?.schema;
    Ok(ContentEditPayload {
        format: doc.format,
        body_format: document::body_format_for_path(&abs),
        front_matter: doc.front_matter,
        body: doc.body,
        schema,
        path: abs.display().to_string(),
    })
}

fn site_root(state: &State<'_, AppState>, site_id: SiteId) -> AppResult<PathBuf> {
    state
        .workspace
        .lock()
        .sites
        .iter()
        .find(|s| s.id == site_id)
        .map(|s| PathBuf::from(&s.root_path))
        .ok_or_else(|| AppError::SiteNotFound(site_id.to_string()))
}

/// Sandbox check (§6.9): the requested path must canonicalise to something
/// that lives inside the site root. Refuse traversal even when the front
/// end is the only caller — defence in depth.
fn resolve_under_root(root: &Path, requested: &str) -> AppResult<PathBuf> {
    let requested_path = PathBuf::from(requested);
    let candidate = if requested_path.is_absolute() {
        requested_path
    } else {
        root.join(requested_path)
    };
    let canonical = std::fs::canonicalize(&candidate)
        .map_err(|e| AppError::Io(format!("{}: {e}", candidate.display())))?;
    let canonical_root = std::fs::canonicalize(root)
        .map_err(|e| AppError::Io(format!("{}: {e}", root.display())))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(AppError::PathTraversal(canonical.display().to_string()));
    }
    Ok(canonical)
}

/// Return the top-level section (first path component under
/// `<root>/content/`, skipping a language subdir if the directory
/// strategy is in use).
fn top_section(root: &Path, abs: &Path) -> Option<String> {
    let content_root = root.join("content");
    let rel = abs.strip_prefix(&content_root).ok()?;
    let mut comps: Vec<String> = rel
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str().map(String::from),
            _ => None,
        })
        .collect();
    if comps.len() < 2 {
        return None;
    }
    // If the first component looks like a language subdir, drop it. We
    // can't introspect the lang list cheaply here, so the heuristic is
    // "two letters or a five-letter `xx-yy`".
    let first = &comps[0];
    let is_lang_like = (first.len() == 2 && first.chars().all(|c| c.is_ascii_alphabetic()))
        || (first.len() == 5
            && first.chars().nth(2) == Some('-')
            && first.chars().take(2).all(|c| c.is_ascii_alphabetic())
            && first.chars().skip(3).all(|c| c.is_ascii_alphabetic()));
    if is_lang_like {
        comps.remove(0);
    }
    comps.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_section_strips_language_subdir() {
        let root = Path::new("/site");
        assert_eq!(
            top_section(root, Path::new("/site/content/posts/hello.md")).as_deref(),
            Some("posts")
        );
        assert_eq!(
            top_section(root, Path::new("/site/content/en/posts/hello.md")).as_deref(),
            Some("posts")
        );
        assert_eq!(
            top_section(root, Path::new("/site/content/en-us/posts/hello.md")).as_deref(),
            Some("posts")
        );
        // Single-segment under content/ has no section.
        assert_eq!(top_section(root, Path::new("/site/content/about.md")), None);
    }
}
