//! Archetypes (M8).
//!
//! Hugo's `archetypes/` directory holds the templates that `hugo new
//! content` instantiates. Each archetype is either:
//!   - a single file `archetypes/<section>.md` → produces a Single Page
//!   - a directory `archetypes/<section>/index.md` (+ siblings) →
//!     produces a Leaf Bundle (with the sibling files copied verbatim)
//!
//! For v1 we support both shapes. Variable substitution is intentionally
//! minimal — covers the patterns 95% of archetypes in the wild use:
//!   - `{{ .Name }}`                                          → slug
//!   - `{{ .Title }}`                                         → title-cased slug
//!   - `{{ .Slug }}`                                          → slug
//!   - `{{ .Section }}`                                       → section
//!   - `{{ .Date }}`                                          → ISO-8601 now
//!   - `{{ replace .Name "-" " " | title }}`                  → title-cased slug
//!   - `{{ replace .Name "_" " " | title }}`                  → title-cased slug
//!
//! Anything else is left in place — the user sees `{{ ... }}` in the
//! editor and can either fix it by hand or run `hugo new` from a
//! terminal for the full template engine.

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::content::classify::{is_content_file, ContentKind, CONTENT_EXTENSIONS};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Archetype {
    /// User-facing name = stem of the archetype file or directory.
    /// "default" is reserved for the fallback applied to every section.
    pub name: String,
    /// Absolute path to the archetype's index file (for bundles) or to
    /// the archetype file itself (for single pages).
    pub path: String,
    pub kind: ContentKind,
}

/// Enumerate everything under `<site>/archetypes/`. Always returns a
/// usable `Vec` — sites without an archetypes/ dir produce an empty list,
/// the caller (UI) renders a "no archetype" hint and the create flow
/// falls back to a built-in minimal template.
pub fn list(site_root: &Path) -> AppResult<Vec<Archetype>> {
    let dir = site_root.join("archetypes");
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let p = entry.path();
        if entry.file_type()?.is_dir() {
            // Directory archetype = leaf bundle (`index.*`) or branch
            // bundle (`_index.*`) template — same `_index` /
            // `index` distinction Hugo uses for content itself.
            let mut matched = false;
            for ext in CONTENT_EXTENSIONS {
                let idx = p.join(format!("_index.{ext}"));
                if idx.is_file() {
                    out.push(Archetype {
                        name: p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string(),
                        path: idx.display().to_string(),
                        kind: ContentKind::BranchBundle,
                    });
                    matched = true;
                    break;
                }
            }
            if matched {
                continue;
            }
            for ext in CONTENT_EXTENSIONS {
                let idx = p.join(format!("index.{ext}"));
                if idx.is_file() {
                    out.push(Archetype {
                        name: p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string(),
                        path: idx.display().to_string(),
                        kind: ContentKind::LeafBundle,
                    });
                    break;
                }
            }
        } else if is_content_file(&p) {
            let stem = p
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if stem.is_empty() {
                continue;
            }
            out.push(Archetype {
                name: stem,
                path: p.display().to_string(),
                kind: ContentKind::SinglePage,
            });
        }
    }
    out.sort_by(|a, b| match (a.name.as_str(), b.name.as_str()) {
        ("default", _) => std::cmp::Ordering::Less,
        (_, "default") => std::cmp::Ordering::Greater,
        (x, y) => x.cmp(y),
    });
    Ok(out)
}

/// Pick the right archetype for `(section, requested)`:
///   1. If the user explicitly named one, use it.
///   2. Else use `<section>.md` if it exists.
///   3. Else use `default.md`.
///   4. Else produce a built-in minimal one (returned as a synthetic in-memory body).
pub fn resolve_template(
    site_root: &Path,
    section: &str,
    requested: Option<&str>,
) -> AppResult<TemplateSource> {
    let archetypes = list(site_root)?;

    if let Some(name) = requested {
        if let Some(a) = archetypes.iter().find(|a| a.name == name) {
            return Ok(TemplateSource::File(a.clone()));
        }
        return Err(AppError::Internal(format!(
            "archetype `{name}` not found in site"
        )));
    }
    if let Some(a) = archetypes.iter().find(|a| a.name == section) {
        return Ok(TemplateSource::File(a.clone()));
    }
    if let Some(a) = archetypes.iter().find(|a| a.name == "default") {
        return Ok(TemplateSource::File(a.clone()));
    }
    Ok(TemplateSource::BuiltIn)
}

#[derive(Debug, Clone)]
pub enum TemplateSource {
    File(Archetype),
    BuiltIn,
}

/// Substitution context.
#[derive(Debug, Clone)]
pub struct SubstContext {
    pub name: String,  // slug
    pub title: String, // title-cased slug
    pub slug: String,  // identical to name
    pub section: String,
    pub date: String, // ISO 8601
}

impl SubstContext {
    pub fn new(section: &str, slug: &str) -> Self {
        Self {
            name: slug.to_string(),
            title: title_case_from_slug(slug),
            slug: slug.to_string(),
            section: section.to_string(),
            date: Utc::now().to_rfc3339(),
        }
    }
}

pub fn substitute(template: &str, ctx: &SubstContext) -> String {
    let mut out = template.to_string();
    // Order matters: handle the more specific compound patterns before
    // the bare `.Name` / `.Slug` ones.
    let pairs = [
        (
            "{{ replace .Name \"-\" \" \" | title }}",
            ctx.title.as_str(),
        ),
        (
            "{{ replace .Name \"_\" \" \" | title }}",
            ctx.title.as_str(),
        ),
        ("{{ .Title }}", ctx.title.as_str()),
        ("{{ .Name }}", ctx.name.as_str()),
        ("{{ .Slug }}", ctx.slug.as_str()),
        ("{{ .Section }}", ctx.section.as_str()),
        ("{{ .Date }}", ctx.date.as_str()),
    ];
    for (needle, value) in pairs {
        out = out.replace(needle, value);
    }
    out
}

pub fn built_in_template() -> &'static str {
    "+++\ntitle = '{{ .Title }}'\ndate = {{ .Date }}\ndraft = true\n+++\n"
}

fn title_case_from_slug(slug: &str) -> String {
    let cleaned = slug.replace(['-', '_'], " ");
    let mut out = String::with_capacity(cleaned.len());
    let mut new_word = true;
    for ch in cleaned.chars() {
        if ch.is_whitespace() {
            out.push(ch);
            new_word = true;
        } else if new_word {
            for u in ch.to_uppercase() {
                out.push(u);
            }
            new_word = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Kind picked when the user creates a new archetype. Mirrors the
/// content kinds Hugo cares about for templates: a single file, a
/// leaf bundle (`<name>/index.md`) or a branch bundle
/// (`<name>/_index.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ArchetypeKind {
    Single,
    LeafBundle,
    BranchBundle,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ArchetypeContent {
    pub name: String,
    pub kind: ContentKind,
    pub path: String,
    pub text: String,
}

/// Read the raw template text of an existing archetype. The `name`
/// argument is the user-facing stem (e.g. `"posts"` or `"default"`),
/// matching `Archetype.name` from [`list`].
pub fn read(site_root: &Path, name: &str) -> AppResult<ArchetypeContent> {
    let arch = find(site_root, name)?;
    let text = std::fs::read_to_string(&arch.path)?;
    Ok(ArchetypeContent {
        name: arch.name,
        kind: arch.kind,
        path: arch.path,
        text,
    })
}

/// Overwrite an existing archetype's body. Refuses to write if the
/// archetype hasn't been seen by [`list`] (so a stray path can't
/// land somewhere unexpected). Atomic tmp+rename.
pub fn write(site_root: &Path, name: &str, text: &str) -> AppResult<ArchetypeContent> {
    let arch = find(site_root, name)?;
    let path = std::path::PathBuf::from(&arch.path);
    sandbox_check(site_root, &path)?;
    atomic_write(&path, text.as_bytes())?;
    Ok(ArchetypeContent {
        name: arch.name,
        kind: arch.kind,
        path: arch.path,
        text: text.to_string(),
    })
}

/// Create a new archetype. `name` becomes the file stem (single) or
/// directory name (bundle); refuses to overwrite an existing one so
/// the caller has to delete first if they really mean it. Seeds the
/// body with [`built_in_template`] when `text` is `None`.
pub fn create(
    site_root: &Path,
    name: &str,
    kind: ArchetypeKind,
    text: Option<&str>,
) -> AppResult<ArchetypeContent> {
    let cleaned = name.trim();
    if cleaned.is_empty() {
        return Err(AppError::Io("archetype name cannot be empty".into()));
    }
    if cleaned.contains(['/', '\\']) || cleaned == "." || cleaned == ".." {
        return Err(AppError::PathTraversal(name.to_string()));
    }
    let dir = site_root.join("archetypes");
    std::fs::create_dir_all(&dir)?;
    sandbox_check(site_root, &dir)?;

    let path = match kind {
        ArchetypeKind::Single => dir.join(format!("{cleaned}.md")),
        ArchetypeKind::LeafBundle => {
            let bundle = dir.join(cleaned);
            std::fs::create_dir_all(&bundle)?;
            bundle.join("index.md")
        }
        ArchetypeKind::BranchBundle => {
            let bundle = dir.join(cleaned);
            std::fs::create_dir_all(&bundle)?;
            bundle.join("_index.md")
        }
    };
    sandbox_check(site_root, &path)?;
    if path.exists() {
        return Err(AppError::Io(format!(
            "archetype already exists: {}",
            path.display()
        )));
    }
    let body = text.unwrap_or_else(|| built_in_template());
    atomic_write(&path, body.as_bytes())?;
    let content_kind = match kind {
        ArchetypeKind::Single => ContentKind::SinglePage,
        ArchetypeKind::LeafBundle => ContentKind::LeafBundle,
        ArchetypeKind::BranchBundle => ContentKind::BranchBundle,
    };
    Ok(ArchetypeContent {
        name: cleaned.to_string(),
        kind: content_kind,
        path: path.display().to_string(),
        text: body.to_string(),
    })
}

/// Delete an archetype (single file or whole bundle directory).
/// Refuses if the archetype isn't listed.
pub fn delete(site_root: &Path, name: &str) -> AppResult<()> {
    let arch = find(site_root, name)?;
    let path = std::path::PathBuf::from(&arch.path);
    sandbox_check(site_root, &path)?;
    match arch.kind {
        ContentKind::SinglePage => {
            std::fs::remove_file(&path)?;
        }
        ContentKind::BranchBundle | ContentKind::LeafBundle => {
            let bundle_dir = path
                .parent()
                .ok_or_else(|| AppError::Io("bundle archetype has no parent".into()))?;
            sandbox_check(site_root, bundle_dir)?;
            std::fs::remove_dir_all(bundle_dir)?;
        }
        ContentKind::Section => {
            return Err(AppError::Internal("section archetypes don't exist".into()));
        }
    }
    Ok(())
}

fn find(site_root: &Path, name: &str) -> AppResult<Archetype> {
    list(site_root)?
        .into_iter()
        .find(|a| a.name == name)
        .ok_or_else(|| AppError::Internal(format!("archetype `{name}` not found")))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bak");
    let tmp = path.with_extension(format!("{ext}.tmp"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).map_err(AppError::from)
}

/// Sandbox the path ascending a chain of canonicalisable parents.
/// Mirrors the helper in `assets::mod` — duplicated to keep the modules
/// independent.
pub fn sandbox_check(site_root: &Path, candidate: &Path) -> AppResult<()> {
    let canonical_root = site_root
        .canonicalize()
        .unwrap_or_else(|_| site_root.to_path_buf());
    let resolved = candidate
        .canonicalize()
        .unwrap_or_else(|_| candidate.to_path_buf());
    if resolved.starts_with(&canonical_root) {
        Ok(())
    } else {
        Err(AppError::PathTraversal(resolved.display().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn site() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        fs::write(root.join("hugo.toml"), "title = \"X\"\n").unwrap();
        (tmp, root)
    }

    #[test]
    fn list_returns_empty_when_no_archetypes_dir() {
        let (_t, root) = site();
        let archetypes = list(&root).unwrap();
        assert!(archetypes.is_empty());
    }

    #[test]
    fn list_orders_default_first() {
        let (_t, root) = site();
        let dir = root.join("archetypes");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("posts.md"), "x").unwrap();
        fs::write(dir.join("default.md"), "x").unwrap();
        fs::write(dir.join("docs.md"), "x").unwrap();
        let archetypes = list(&root).unwrap();
        let names: Vec<_> = archetypes.iter().map(|a| a.name.clone()).collect();
        assert_eq!(names, vec!["default", "docs", "posts"]);
    }

    #[test]
    fn list_picks_up_directory_bundle_archetype() {
        let (_t, root) = site();
        let dir = root.join("archetypes/post-bundle");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("index.md"), "x").unwrap();
        let archetypes = list(&root).unwrap();
        assert!(archetypes
            .iter()
            .any(|a| a.name == "post-bundle" && a.kind == ContentKind::LeafBundle));
    }

    #[test]
    fn list_distinguishes_branch_from_leaf_bundle_archetypes() {
        let (_t, root) = site();
        let leaf = root.join("archetypes/post-bundle");
        let branch = root.join("archetypes/section-bundle");
        fs::create_dir_all(&leaf).unwrap();
        fs::create_dir_all(&branch).unwrap();
        fs::write(leaf.join("index.md"), "x").unwrap();
        fs::write(branch.join("_index.md"), "x").unwrap();
        let archetypes = list(&root).unwrap();
        let by_name: std::collections::HashMap<_, _> = archetypes
            .iter()
            .map(|a| (a.name.clone(), a.kind))
            .collect();
        assert_eq!(by_name.get("post-bundle"), Some(&ContentKind::LeafBundle));
        assert_eq!(
            by_name.get("section-bundle"),
            Some(&ContentKind::BranchBundle)
        );
    }

    #[test]
    fn create_writes_single_archetype_with_default_template() {
        let (_t, root) = site();
        let arch = create(&root, "posts", ArchetypeKind::Single, None).unwrap();
        assert_eq!(arch.kind, ContentKind::SinglePage);
        let on_disk = fs::read_to_string(&arch.path).unwrap();
        assert!(on_disk.contains("title = '{{ .Title }}'"));
    }

    #[test]
    fn create_writes_branch_bundle_archetype() {
        let (_t, root) = site();
        let arch = create(&root, "section", ArchetypeKind::BranchBundle, Some("hi")).unwrap();
        assert_eq!(arch.kind, ContentKind::BranchBundle);
        assert!(root.join("archetypes/section/_index.md").is_file());
        assert_eq!(
            fs::read_to_string(root.join("archetypes/section/_index.md")).unwrap(),
            "hi"
        );
    }

    #[test]
    fn create_refuses_to_overwrite() {
        let (_t, root) = site();
        create(&root, "posts", ArchetypeKind::Single, None).unwrap();
        let err = create(&root, "posts", ArchetypeKind::Single, None).unwrap_err();
        match err {
            AppError::Io(msg) => assert!(msg.contains("already exists")),
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn create_rejects_path_traversal_in_name() {
        let (_t, root) = site();
        assert!(create(&root, "../escape", ArchetypeKind::Single, None).is_err());
        assert!(create(&root, "nested/name", ArchetypeKind::Single, None).is_err());
    }

    #[test]
    fn read_then_write_round_trip() {
        let (_t, root) = site();
        create(&root, "posts", ArchetypeKind::Single, Some("v1\n")).unwrap();
        let read1 = read(&root, "posts").unwrap();
        assert_eq!(read1.text, "v1\n");
        write(&root, "posts", "v2\n").unwrap();
        let read2 = read(&root, "posts").unwrap();
        assert_eq!(read2.text, "v2\n");
    }

    #[test]
    fn delete_removes_single_file_archetype() {
        let (_t, root) = site();
        create(&root, "posts", ArchetypeKind::Single, None).unwrap();
        delete(&root, "posts").unwrap();
        assert!(list(&root).unwrap().iter().all(|a| a.name != "posts"));
    }

    #[test]
    fn delete_removes_bundle_archetype_directory() {
        let (_t, root) = site();
        create(&root, "story", ArchetypeKind::LeafBundle, None).unwrap();
        delete(&root, "story").unwrap();
        assert!(!root.join("archetypes/story").exists());
    }

    #[test]
    fn resolve_template_prefers_explicit_then_section_then_default_then_builtin() {
        let (_t, root) = site();
        // empty: built-in
        let r = resolve_template(&root, "posts", None).unwrap();
        assert!(matches!(r, TemplateSource::BuiltIn));
        // add default
        let dir = root.join("archetypes");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("default.md"), "default").unwrap();
        let r = resolve_template(&root, "posts", None).unwrap();
        match r {
            TemplateSource::File(a) => assert_eq!(a.name, "default"),
            _ => panic!("expected file"),
        }
        // add section-specific
        fs::write(dir.join("posts.md"), "posts-specific").unwrap();
        let r = resolve_template(&root, "posts", None).unwrap();
        match r {
            TemplateSource::File(a) => assert_eq!(a.name, "posts"),
            _ => panic!(),
        }
        // explicit name wins
        fs::write(dir.join("custom.md"), "custom").unwrap();
        let r = resolve_template(&root, "posts", Some("custom")).unwrap();
        match r {
            TemplateSource::File(a) => assert_eq!(a.name, "custom"),
            _ => panic!(),
        }
    }

    #[test]
    fn substitute_replaces_known_patterns() {
        let ctx = SubstContext {
            name: "hello-world".into(),
            title: "Hello World".into(),
            slug: "hello-world".into(),
            section: "posts".into(),
            date: "2026-04-25T10:00:00Z".into(),
        };
        let out = substitute(
            "title: '{{ replace .Name \"-\" \" \" | title }}'\ndate: {{ .Date }}\ndraft: true\nsection: {{ .Section }}\n",
            &ctx,
        );
        assert!(out.contains("title: 'Hello World'"));
        assert!(out.contains("date: 2026-04-25T10:00:00Z"));
        assert!(out.contains("section: posts"));
    }

    #[test]
    fn title_case_from_slug_handles_dashes_and_underscores() {
        assert_eq!(title_case_from_slug("hello-world"), "Hello World");
        assert_eq!(title_case_from_slug("a_b-c"), "A B C");
        assert_eq!(title_case_from_slug("single"), "Single");
    }

    #[test]
    fn built_in_template_has_required_keys() {
        let t = built_in_template();
        assert!(t.contains("title"));
        assert!(t.contains("date"));
        assert!(t.contains("draft"));
    }
}
