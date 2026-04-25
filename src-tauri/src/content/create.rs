//! `content_create` (M8).
//!
//! Resolve the archetype, pick the on-disk target path (respecting the
//! site's language strategy), substitute the basic template variables,
//! refuse if the target already exists, then write atomically.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::content::archetype::{
    self, built_in_template, resolve_template, sandbox_check, SubstContext, TemplateSource,
};
use crate::content::classify::ContentKind;
use crate::content::language::{detect as detect_language, LanguageStrategy};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateOptions {
    pub section: String,
    pub slug: String,
    /// Optional explicit archetype name (matches `Archetype.name`); when
    /// `None` the resolver walks `<section>` → `default` → built-in.
    pub archetype: Option<String>,
    /// Optional language code. When the site uses the directory
    /// strategy, this determines the `<lang>/` prefix; for the filename
    /// strategy it becomes a `.lang` suffix on the file stem.
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreatedContent {
    /// Absolute path of the created file (or its index, for bundles).
    pub path: String,
    /// Site-relative content id (`posts/hello.md` or `posts/hello/`).
    pub id: String,
    pub kind: ContentKind,
    pub language: Option<String>,
    pub archetype_used: String,
}

pub fn create(
    site_root: &Path,
    merged_config: &serde_json::Value,
    opts: &CreateOptions,
) -> AppResult<CreatedContent> {
    let section = sanitise_segment(&opts.section, "section")?;
    let slug = sanitise_segment(&opts.slug, "slug")?;

    let lang_info = detect_language(merged_config, Some(&site_root.join("content")));
    let language = match (&opts.language, lang_info.strategy) {
        (Some(c), _) if !c.is_empty() => Some(c.clone()),
        (None, LanguageStrategy::Directory) => Some(lang_info.default_language.clone()),
        _ => None,
    };

    let template = resolve_template(site_root, &section, opts.archetype.as_deref())?;
    let (archetype_kind, raw_template, archetype_used) = match &template {
        TemplateSource::File(a) => {
            let raw = std::fs::read_to_string(&a.path)?;
            (a.kind, raw, a.name.clone())
        }
        TemplateSource::BuiltIn => (
            ContentKind::SinglePage,
            built_in_template().to_string(),
            "(built-in)".to_string(),
        ),
    };

    let target = build_target_path(
        site_root,
        &section,
        &slug,
        archetype_kind,
        language.as_deref(),
        lang_info.strategy,
    );
    sandbox_check(site_root, &target.dir)?;
    if target.file.exists() {
        return Err(AppError::Internal(format!(
            "content already exists at {}",
            target.file.display()
        )));
    }

    std::fs::create_dir_all(&target.dir)?;
    let ctx = SubstContext::new(&section, &slug);
    let body = archetype::substitute(&raw_template, &ctx);
    atomic_write(&target.file, body.as_bytes())?;

    // Bundle archetype: copy sibling files (assets) untouched.
    if matches!(
        archetype_kind,
        ContentKind::LeafBundle | ContentKind::BranchBundle
    ) {
        if let TemplateSource::File(arch) = &template {
            let arch_dir = PathBuf::from(&arch.path)
                .parent()
                .ok_or_else(|| AppError::Internal("archetype path has no parent".into()))?
                .to_path_buf();
            for entry in std::fs::read_dir(&arch_dir)? {
                let entry = entry?;
                let p = entry.path();
                if !entry.file_type()?.is_file() {
                    continue;
                }
                if p.as_path() == Path::new(&arch.path) {
                    continue; // index already written
                }
                if let Some(name) = p.file_name() {
                    let dst = target.dir.join(name);
                    std::fs::copy(&p, &dst)?;
                }
            }
        }
    }

    let rel_id = relative_id(site_root, &target.file, archetype_kind, &target.dir);
    Ok(CreatedContent {
        path: target.file.display().to_string(),
        id: rel_id,
        kind: archetype_kind,
        language,
        archetype_used,
    })
}

#[derive(Debug)]
struct TargetPaths {
    /// Directory the file lives in.
    dir: PathBuf,
    /// File the user will edit.
    file: PathBuf,
}

fn build_target_path(
    site_root: &Path,
    section: &str,
    slug: &str,
    kind: ContentKind,
    language: Option<&str>,
    strategy: LanguageStrategy,
) -> TargetPaths {
    let mut content_root = site_root.join("content");
    if let (Some(lang), LanguageStrategy::Directory) = (language, strategy) {
        content_root = content_root.join(lang);
    }
    match kind {
        ContentKind::LeafBundle | ContentKind::BranchBundle => {
            let bundle_dir = content_root.join(section).join(slug);
            let stem = if matches!(kind, ContentKind::BranchBundle) {
                "_index.md"
            } else {
                "index.md"
            };
            TargetPaths {
                file: bundle_dir.join(stem),
                dir: bundle_dir,
            }
        }
        _ => {
            let dir = content_root.join(section);
            let file_name = match (language, strategy) {
                (Some(lang), LanguageStrategy::Filename) => format!("{slug}.{lang}.md"),
                _ => format!("{slug}.md"),
            };
            TargetPaths {
                file: dir.join(file_name),
                dir,
            }
        }
    }
}

fn relative_id(site_root: &Path, file: &Path, kind: ContentKind, dir: &Path) -> String {
    let content_root = site_root.join("content");
    let target = match kind {
        ContentKind::LeafBundle | ContentKind::BranchBundle => dir,
        _ => file,
    };
    target
        .strip_prefix(&content_root)
        .map(|rel| rel.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|_| target.display().to_string())
}

fn sanitise_segment(value: &str, label: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Internal(format!("{label} cannot be empty")));
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err(AppError::Internal(format!(
            "{label} `{trimmed}` contains illegal characters"
        )));
    }
    Ok(trimmed.to_string())
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
    fn create_uses_builtin_when_no_archetype_present() {
        let (_t, root) = site();
        let r = create(
            &root,
            &serde_json::json!({}),
            &CreateOptions {
                section: "posts".into(),
                slug: "hello-world".into(),
                archetype: None,
                language: None,
            },
        )
        .unwrap();
        assert_eq!(r.kind, ContentKind::SinglePage);
        assert_eq!(r.id, "posts/hello-world.md");
        assert_eq!(r.archetype_used, "(built-in)");
        let body = fs::read_to_string(&r.path).unwrap();
        assert!(body.contains("title = 'Hello World'"));
        assert!(body.contains("draft = true"));
    }

    #[test]
    fn create_substitutes_section_specific_archetype() {
        let (_t, root) = site();
        let arch_dir = root.join("archetypes");
        fs::create_dir_all(&arch_dir).unwrap();
        fs::write(
            arch_dir.join("posts.md"),
            "---\ntitle: '{{ replace .Name \"-\" \" \" | title }}'\ndate: {{ .Date }}\ndraft: true\nsection: {{ .Section }}\n---\nbody\n",
        )
        .unwrap();
        let r = create(
            &root,
            &serde_json::json!({}),
            &CreateOptions {
                section: "posts".into(),
                slug: "my-first-post".into(),
                archetype: None,
                language: None,
            },
        )
        .unwrap();
        assert_eq!(r.archetype_used, "posts");
        let body = fs::read_to_string(&r.path).unwrap();
        assert!(body.contains("title: 'My First Post'"));
        assert!(body.contains("section: posts"));
    }

    #[test]
    fn create_refuses_to_overwrite_existing_file() {
        let (_t, root) = site();
        let posts = root.join("content/posts");
        fs::create_dir_all(&posts).unwrap();
        fs::write(posts.join("hello.md"), "exists").unwrap();
        let err = create(
            &root,
            &serde_json::json!({}),
            &CreateOptions {
                section: "posts".into(),
                slug: "hello".into(),
                archetype: None,
                language: None,
            },
        )
        .unwrap_err();
        match err {
            AppError::Internal(m) => assert!(m.contains("already exists")),
            _ => panic!(),
        }
    }

    #[test]
    fn create_filename_strategy_appends_lang_suffix() {
        let (_t, root) = site();
        let cfg = serde_json::json!({ "languages": { "en": {}, "it": {} } });
        let r = create(
            &root,
            &cfg,
            &CreateOptions {
                section: "posts".into(),
                slug: "hello".into(),
                archetype: None,
                language: Some("it".into()),
            },
        )
        .unwrap();
        assert_eq!(r.id, "posts/hello.it.md");
        assert_eq!(r.language.as_deref(), Some("it"));
    }

    #[test]
    fn create_directory_strategy_prefixes_lang_dir() {
        let (_t, root) = site();
        // Pre-create the lang subdir so detect picks the directory strategy.
        fs::create_dir_all(root.join("content/en")).unwrap();
        fs::create_dir_all(root.join("content/it")).unwrap();
        let cfg = serde_json::json!({
            "languages": { "en": {}, "it": {} },
            "defaultContentLanguage": "en",
            "defaultContentLanguageInSubdir": true,
        });
        let r = create(
            &root,
            &cfg,
            &CreateOptions {
                section: "posts".into(),
                slug: "hello".into(),
                archetype: None,
                language: Some("it".into()),
            },
        )
        .unwrap();
        assert!(r.path.ends_with("content/it/posts/hello.md"));
    }

    #[test]
    fn create_rejects_section_with_separator() {
        let (_t, root) = site();
        let err = create(
            &root,
            &serde_json::json!({}),
            &CreateOptions {
                section: "../escape".into(),
                slug: "hello".into(),
                archetype: None,
                language: None,
            },
        )
        .unwrap_err();
        match err {
            AppError::Internal(m) => assert!(m.contains("illegal")),
            _ => panic!(),
        }
    }

    #[test]
    fn create_bundle_archetype_creates_leaf_bundle() {
        let (_t, root) = site();
        let arch = root.join("archetypes/post-bundle");
        fs::create_dir_all(&arch).unwrap();
        fs::write(
            arch.join("index.md"),
            "---\ntitle: '{{ .Title }}'\n---\nbody\n",
        )
        .unwrap();
        fs::write(arch.join("cover.jpg"), b"fake").unwrap();
        let r = create(
            &root,
            &serde_json::json!({}),
            &CreateOptions {
                section: "posts".into(),
                slug: "story".into(),
                archetype: Some("post-bundle".into()),
                language: None,
            },
        )
        .unwrap();
        assert_eq!(r.kind, ContentKind::LeafBundle);
        assert_eq!(r.id, "posts/story");
        assert!(root.join("content/posts/story/index.md").is_file());
        assert!(root.join("content/posts/story/cover.jpg").is_file());
    }
}
