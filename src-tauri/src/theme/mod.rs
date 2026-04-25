//! Theme params editor (M5).
//!
//! Resolves the active theme's parameter schema by walking the cascade
//! described in §6.4:
//!   1. `themes/<name>/.hugoeditor/theme-schema.json` (opt-in manifest)
//!   2. `themes/<name>/{config.{yaml,yml,toml,json},theme.toml}` with a
//!      `[params]` section (treated as defaults)
//!   3. Inference from the site's current `params`
//!
//! Either way the result is a [`FrontMatterSchema`] the existing form
//! renderer can already display, plus a [`SchemaSource`] badge so the UI
//! is honest about where the schema came from.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::config::cascade;
use crate::content::schema::{
    classify_value, humanise, FieldDef, FieldType, FrontMatterSchema, UnknownFieldsPolicy,
};
use crate::error::{AppError, AppResult};
use crate::hugo::detect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SchemaSource {
    /// Authoritative schema shipped by the theme author at
    /// `themes/<name>/.hugoeditor/theme-schema.json`.
    Manifest,
    /// Inferred from the theme's `[params]` defaults (`config.*` /
    /// `theme.toml`).
    Defaults,
    /// Last resort — derived from the values currently set in the site's
    /// config.
    Inferred,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThemeInfo {
    pub theme_name: Option<String>,
    pub theme_path: Option<String>,
    pub schema: FrontMatterSchema,
    pub source: SchemaSource,
    pub params: serde_json::Value,
}

/// Read the active config, find the theme, and produce a [`ThemeInfo`]
/// the UI can render. Returns a usable result even when no theme is
/// configured (Inferred from whatever is in `params`).
pub fn load(site_root: &Path) -> AppResult<ThemeInfo> {
    let det = detect::detect(site_root)?;
    let merged = cascade::load(&det)?.merged;

    let theme_name = merged.get("theme").and_then(|v| match v {
        serde_json::Value::String(s) => Some(s.clone()),
        // Hugo accepts an array of themes; pick the first as the
        // "primary". Component themes layered on top of it are out of
        // scope for v1 — note in DECISIONS once we hit that case.
        serde_json::Value::Array(arr) => arr.iter().find_map(|v| v.as_str().map(String::from)),
        _ => None,
    });

    let theme_path = theme_name
        .as_ref()
        .map(|n| site_root.join("themes").join(n));

    let current_params = merged
        .get("params")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    // 1. Manifest
    if let Some(p) = theme_path
        .as_ref()
        .map(|p| p.join(".hugoeditor").join("theme-schema.json"))
    {
        if p.is_file() {
            match read_manifest(&p) {
                Ok(schema) => {
                    return Ok(ThemeInfo {
                        theme_name,
                        theme_path: theme_path.map(stringify_path),
                        schema,
                        source: SchemaSource::Manifest,
                        params: current_params,
                    });
                }
                Err(err) => {
                    eprintln!("[theme] manifest at {} unreadable: {err}", p.display());
                }
            }
        }
    }

    // 2. Theme defaults
    if let Some(theme_dir) = theme_path.as_ref() {
        if let Some(defaults) = read_theme_defaults(theme_dir)? {
            let defaults_params = extract_params(&defaults);
            let mut effective = defaults_params.clone();
            // Merge: site's current values win (so the form opens with the
            // user's edited values, not the theme defaults).
            if let serde_json::Value::Object(current_obj) = &current_params {
                if let serde_json::Value::Object(eff_obj) = &mut effective {
                    for (k, v) in current_obj {
                        eff_obj.insert(k.clone(), v.clone());
                    }
                }
            }
            let schema = infer_from_object(&defaults_params, "Theme params");
            return Ok(ThemeInfo {
                theme_name,
                theme_path: Some(stringify_path(theme_dir.clone())),
                schema,
                source: SchemaSource::Defaults,
                params: effective,
            });
        }
    }

    // 3. Inference from current values
    let schema = infer_from_object(&current_params, "Theme params");
    Ok(ThemeInfo {
        theme_name,
        theme_path: theme_path.map(stringify_path),
        schema,
        source: SchemaSource::Inferred,
        params: current_params,
    })
}

/// Persist `new_params` back into the site config. Reuses the cascade
/// writer so a `params.toml` cascade entry stays where it was.
pub fn save_params(site_root: &Path, new_params: &serde_json::Value) -> AppResult<()> {
    let det = detect::detect(site_root)?;
    let mut merged = cascade::load(&det)?.merged;
    if let serde_json::Value::Object(obj) = &mut merged {
        let is_empty_object = matches!(
            new_params,
            serde_json::Value::Object(m) if m.is_empty()
        );
        if new_params.is_null() || is_empty_object {
            obj.remove("params");
        } else {
            obj.insert("params".into(), new_params.clone());
        }
    }
    cascade::save(&det, &merged)
}

fn stringify_path(p: PathBuf) -> String {
    p.display().to_string()
}

fn read_manifest(path: &Path) -> AppResult<FrontMatterSchema> {
    let raw = std::fs::read_to_string(path)?;
    let schema: FrontMatterSchema =
        serde_json::from_str(&raw).map_err(|e| AppError::Serde(e.to_string()))?;
    Ok(schema)
}

fn read_theme_defaults(theme_dir: &Path) -> AppResult<Option<serde_json::Value>> {
    for name in &[
        "config.toml",
        "config.yaml",
        "config.yml",
        "config.json",
        "theme.toml",
    ] {
        let p = theme_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let raw = std::fs::read_to_string(&p)?;
        let parsed: serde_json::Value = match p.extension().and_then(|e| e.to_str()) {
            Some("toml") => {
                let v: toml::Value =
                    toml::from_str(&raw).map_err(|e| AppError::Serde(e.to_string()))?;
                serde_json::to_value(v)?
            }
            Some("yaml" | "yml") => {
                serde_yaml::from_str(&raw).map_err(|e| AppError::Serde(e.to_string()))?
            }
            Some("json") => serde_json::from_str(&raw)?,
            _ => continue,
        };
        return Ok(Some(parsed));
    }
    Ok(None)
}

/// Pull the `[params]` section out of a theme config, returning an empty
/// object if none is present. Some theme authors hoist params straight
/// to the root — we honour that fallback so they're not invisible.
fn extract_params(theme_config: &serde_json::Value) -> serde_json::Value {
    match theme_config.get("params") {
        Some(p) if p.is_object() => p.clone(),
        _ => theme_config
            .as_object()
            .map(|m| {
                let mut owned = serde_json::Map::new();
                for (k, v) in m {
                    if !matches!(k.as_str(), "baseURL" | "title" | "languageCode" | "theme") {
                        owned.insert(k.clone(), v.clone());
                    }
                }
                serde_json::Value::Object(owned)
            })
            .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
    }
}

fn infer_from_object(value: &serde_json::Value, group: &str) -> FrontMatterSchema {
    let empty = serde_json::Map::new();
    let map = value.as_object().unwrap_or(&empty);

    let mut fields = Vec::with_capacity(map.len());
    for (key, val) in map {
        let ft = classify_value(val).unwrap_or(FieldType::String);
        fields.push(FieldDef {
            key: key.clone(),
            label: humanise(key),
            field_type: ft,
            required: false,
            default: Some(val.clone()),
            enum_values: None,
            group: Some(group.to_string()),
            hidden: false,
            description: None,
        });
    }

    fields.sort_by(|a, b| a.key.cmp(&b.key));

    FrontMatterSchema {
        fields,
        unknown_fields_policy: UnknownFieldsPolicy::Preserve,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn site_with(cfg_toml: &str) -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("hugo.toml"), cfg_toml).unwrap();
        let root = tmp.path().to_path_buf();
        (tmp, root)
    }

    #[test]
    fn no_theme_falls_back_to_inferred() {
        let (_t, root) =
            site_with("title = \"X\"\n\n[params]\nauthor = \"Jane\"\nshowToc = true\n");
        let info = load(&root).unwrap();
        assert!(info.theme_name.is_none());
        assert_eq!(info.source, SchemaSource::Inferred);
        let by_key: std::collections::HashMap<_, _> = info
            .schema
            .fields
            .iter()
            .map(|f| (f.key.clone(), f))
            .collect();
        assert_eq!(by_key.get("author").unwrap().field_type, FieldType::String);
        assert_eq!(
            by_key.get("showToc").unwrap().field_type,
            FieldType::Boolean
        );
    }

    #[test]
    fn missing_theme_dir_still_works() {
        let (_t, root) =
            site_with("title = \"X\"\ntheme = \"papermod\"\n\n[params]\nauthor = \"Jane\"\n");
        let info = load(&root).unwrap();
        assert_eq!(info.theme_name.as_deref(), Some("papermod"));
        assert_eq!(info.source, SchemaSource::Inferred);
        assert!(info.schema.fields.iter().any(|f| f.key == "author"));
    }

    #[test]
    fn theme_defaults_drive_schema_when_no_manifest() {
        let (_t, root) =
            site_with("title = \"X\"\ntheme = \"papermod\"\n\n[params]\nauthor = \"Jane\"\n");
        let theme_dir = root.join("themes").join("papermod");
        fs::create_dir_all(&theme_dir).unwrap();
        fs::write(
            theme_dir.join("theme.toml"),
            "name = \"PaperMod\"\n[params]\nauthor = \"Default\"\nshowReadingTime = true\nlogo = \"/logo.png\"\n",
        )
        .unwrap();
        let info = load(&root).unwrap();
        assert_eq!(info.source, SchemaSource::Defaults);
        let by_key: std::collections::HashMap<_, _> = info
            .schema
            .fields
            .iter()
            .map(|f| (f.key.clone(), f))
            .collect();
        assert!(by_key.contains_key("author"));
        assert!(by_key.contains_key("showReadingTime"));
        assert_eq!(
            by_key.get("showReadingTime").unwrap().field_type,
            FieldType::Boolean
        );
        // current value (Jane) wins over the theme default.
        assert_eq!(info.params["author"], "Jane");
        // theme defaults that the site hasn't set yet are surfaced too.
        assert_eq!(info.params["showReadingTime"], true);
    }

    #[test]
    fn manifest_wins_over_defaults() {
        let (_t, root) =
            site_with("title = \"X\"\ntheme = \"papermod\"\n\n[params]\nauthor = \"Jane\"\n");
        let theme_dir = root.join("themes").join("papermod");
        let editor_dir = theme_dir.join(".hugoeditor");
        fs::create_dir_all(&editor_dir).unwrap();
        fs::write(
            editor_dir.join("theme-schema.json"),
            r#"{
              "fields": [
                {
                  "key": "author",
                  "label": "Author name",
                  "fieldType": "string",
                  "required": true,
                  "default": null,
                  "enumValues": null,
                  "group": "Identity",
                  "hidden": false,
                  "description": "Visible byline below each post"
                }
              ],
              "unknownFieldsPolicy": "preserve"
            }"#,
        )
        .unwrap();
        // Defaults file present too — should be ignored.
        fs::write(
            theme_dir.join("theme.toml"),
            "[params]\nshowReadingTime = true\n",
        )
        .unwrap();

        let info = load(&root).unwrap();
        assert_eq!(info.source, SchemaSource::Manifest);
        let f = info
            .schema
            .fields
            .iter()
            .find(|f| f.key == "author")
            .unwrap();
        assert!(f.required);
        assert_eq!(f.group.as_deref(), Some("Identity"));
        assert_eq!(
            f.description.as_deref(),
            Some("Visible byline below each post")
        );
    }

    #[test]
    fn save_params_writes_back_to_hugo_toml() {
        let (_t, root) = site_with(
            "# header\ntitle = \"X\"\ntheme = \"papermod\"\n\n[params]\nauthor = \"Jane\"\n",
        );
        let new_params = serde_json::json!({"author": "Bob"});
        save_params(&root, &new_params).unwrap();
        let after = std::fs::read_to_string(root.join("hugo.toml")).unwrap();
        assert!(after.contains("# header"));
        assert!(after.contains("\"Bob\""));
        assert!(!after.contains("\"Jane\""));
    }

    #[test]
    fn save_params_to_default_directory_targets_params_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("config").join("_default");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("hugo.toml"),
            "title = \"X\"\ntheme = \"papermod\"\n",
        )
        .unwrap();
        let params_path = dir.join("params.toml");
        fs::write(&params_path, "author = \"Jane\"\n").unwrap();

        let new_params = serde_json::json!({"author": "Bob"});
        save_params(tmp.path(), &new_params).unwrap();

        let hugo_after = fs::read_to_string(dir.join("hugo.toml")).unwrap();
        let params_after = fs::read_to_string(&params_path).unwrap();
        // hugo.toml untouched (no params edit landed there)
        assert!(!hugo_after.contains("Bob"));
        // params.toml took the edit
        assert!(params_after.contains("Bob"));
        assert!(!params_after.contains("Jane"));
    }
}
