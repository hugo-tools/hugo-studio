use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::config::format::ConfigFormat;
use crate::config::json::JsonCodec;
use crate::config::toml::TomlCodec;
use crate::config::yaml::YamlCodec;
use crate::config::ConfigCodec;
use crate::error::{AppError, AppResult};
use crate::hugo::detect::{DetectionInfo, HugoConfigKind};

/// One config source on disk plus its current parsed contents.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSource {
    pub path: String,
    pub format: ConfigFormat,
    /// Top-level key under which this file's contents are merged. `null`
    /// means the file's keys live at the merged root (the canonical
    /// `hugo.{toml,yaml,json}` case).
    pub mount_key: Option<String>,
}

/// Result of [`load`] — both the canonical merged JSON for the UI to
/// render, and the per-file mapping needed to write changes back.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LoadedConfig {
    pub format: ConfigFormat,
    pub sources: Vec<ConfigSource>,
    pub merged: serde_json::Value,
}

/// Read the config according to `detection`, parse every involved file,
/// and return the merged JSON plus the source map.
pub fn load(detection: &DetectionInfo) -> AppResult<LoadedConfig> {
    let sources = enumerate_sources(detection)?;
    let mut merged = serde_json::Value::Object(serde_json::Map::new());

    for source in &sources {
        let raw = std::fs::read_to_string(&source.path)?;
        let parsed = parse_source(&source.format, &raw)?;
        match &source.mount_key {
            None => merge_into_root(&mut merged, parsed),
            Some(key) => {
                merged
                    .as_object_mut()
                    .expect("merged root is always an object")
                    .insert(key.clone(), parsed);
            }
        }
    }

    let primary_format = sources
        .first()
        .map(|s| s.format)
        .ok_or_else(|| AppError::NotAHugoSite("no config files found".into()))?;

    Ok(LoadedConfig {
        format: primary_format,
        sources,
        merged,
    })
}

/// Apply `new_merged` against the on-disk sources. Files whose extracted
/// slice of `new_merged` is unchanged are left untouched; everything else
/// is rewritten via the format-specific codec.
pub fn save(detection: &DetectionInfo, new_merged: &serde_json::Value) -> AppResult<()> {
    let current = load(detection)?;

    for source in &current.sources {
        let new_slice = match &source.mount_key {
            None => extract_root_slice(&current.merged, new_merged, &current.sources),
            Some(key) => new_merged
                .get(key)
                .cloned()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
        };
        let original = std::fs::read_to_string(&source.path)?;
        let updated = apply_changes_for(&source.format, &original, &new_slice)?;
        if updated != original {
            atomic_write(Path::new(&source.path), updated.as_bytes())?;
        }
    }

    Ok(())
}

fn enumerate_sources(detection: &DetectionInfo) -> AppResult<Vec<ConfigSource>> {
    match detection.kind {
        HugoConfigKind::HugoToml
        | HugoConfigKind::HugoYaml
        | HugoConfigKind::HugoJson
        | HugoConfigKind::ConfigToml
        | HugoConfigKind::ConfigYaml
        | HugoConfigKind::ConfigJson => {
            let path = PathBuf::from(&detection.config_path);
            let format = ConfigFormat::from_path(&path)?;
            Ok(vec![ConfigSource {
                path: detection.config_path.clone(),
                format,
                mount_key: None,
            }])
        }
        HugoConfigKind::DefaultDirectory => enumerate_default_dir(&detection.config_path),
    }
}

fn enumerate_default_dir(dir: &str) -> AppResult<Vec<ConfigSource>> {
    let mut sources = Vec::new();
    let mut hugo_root: Option<ConfigSource> = None;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        let format = match ConfigFormat::from_path(&path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let path_str = path.display().to_string();

        if matches!(stem.as_str(), "hugo" | "config") {
            hugo_root = Some(ConfigSource {
                path: path_str,
                format,
                mount_key: None,
            });
        } else {
            sources.push(ConfigSource {
                path: path_str,
                format,
                mount_key: Some(stem),
            });
        }
    }

    // Hugo root file (if any) goes first so its keys form the base of the
    // merged object before sub-files mount their nested sections.
    if let Some(root) = hugo_root {
        sources.insert(0, root);
    }

    if sources.is_empty() {
        return Err(AppError::NotAHugoSite(format!(
            "config/_default/ has no recognised hugo.* / config.* / *.toml|yaml|json files: {dir}"
        )));
    }

    sources.sort_by(|a, b| match (&a.mount_key, &b.mount_key) {
        (None, _) => std::cmp::Ordering::Less,
        (_, None) => std::cmp::Ordering::Greater,
        (Some(x), Some(y)) => x.cmp(y),
    });

    Ok(sources)
}

fn parse_source(fmt: &ConfigFormat, raw: &str) -> AppResult<serde_json::Value> {
    match fmt {
        ConfigFormat::Toml => TomlCodec::parse(raw),
        ConfigFormat::Yaml => YamlCodec::parse(raw),
        ConfigFormat::Json => JsonCodec::parse(raw),
    }
}

fn apply_changes_for(
    fmt: &ConfigFormat,
    original: &str,
    new_data: &serde_json::Value,
) -> AppResult<String> {
    match fmt {
        ConfigFormat::Toml => TomlCodec::apply_changes(original, new_data),
        ConfigFormat::Yaml => YamlCodec::apply_changes(original, new_data),
        ConfigFormat::Json => JsonCodec::apply_changes(original, new_data),
    }
}

fn merge_into_root(target: &mut serde_json::Value, src: serde_json::Value) {
    if let (Some(t_obj), Some(s_obj)) = (target.as_object_mut(), src.as_object()) {
        for (k, v) in s_obj {
            t_obj.insert(k.clone(), v.clone());
        }
    }
}

/// Compute the slice of the new merged JSON that belongs to a root-mounted
/// file: everything except the keys claimed by other (mount_key-bearing)
/// sources. This way the `params:` key stays in `params.toml` instead of
/// being duplicated into `hugo.toml`.
fn extract_root_slice(
    _original_merged: &serde_json::Value,
    new_merged: &serde_json::Value,
    sources: &[ConfigSource],
) -> serde_json::Value {
    let claimed: Vec<&str> = sources
        .iter()
        .filter_map(|s| s.mount_key.as_deref())
        .collect();
    let mut out = serde_json::Map::new();
    if let Some(obj) = new_merged.as_object() {
        for (k, v) in obj {
            if !claimed.contains(&k.as_str()) {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    serde_json::Value::Object(out)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("bak")
    ));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn detection(path: &Path, kind: HugoConfigKind) -> DetectionInfo {
        DetectionInfo {
            kind,
            config_path: path.display().to_string(),
        }
    }

    #[test]
    fn loads_single_toml_file() {
        let tmp = TempDir::new().unwrap();
        let cfg = tmp.path().join("hugo.toml");
        fs::write(&cfg, "title = \"Hi\"\nbaseURL = \"https://x\"\n").unwrap();
        let loaded = load(&detection(&cfg, HugoConfigKind::HugoToml)).unwrap();
        assert_eq!(loaded.merged["title"], "Hi");
        assert_eq!(loaded.merged["baseURL"], "https://x");
    }

    #[test]
    fn round_trip_baseurl_change_one_line_diff_toml() {
        let tmp = TempDir::new().unwrap();
        let cfg = tmp.path().join("hugo.toml");
        let original = "# top comment\ntitle = \"Hi\"\nbaseURL = \"https://x\"\n";
        fs::write(&cfg, original).unwrap();
        let det = detection(&cfg, HugoConfigKind::HugoToml);

        let mut loaded = load(&det).unwrap();
        loaded.merged["baseURL"] = serde_json::Value::String("https://new".into());
        save(&det, &loaded.merged).unwrap();

        let written = fs::read_to_string(&cfg).unwrap();
        assert!(written.contains("# top comment"));
        assert!(written.contains("https://new"));
        assert_eq!(
            written.matches('\n').count(),
            original.matches('\n').count()
        );
    }

    #[test]
    fn enumerates_default_directory_with_params_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("config").join("_default");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("hugo.toml"), "title = \"X\"\n").unwrap();
        fs::write(dir.join("params.toml"), "author = \"Jane\"\n").unwrap();
        let loaded = load(&detection(&dir, HugoConfigKind::DefaultDirectory)).unwrap();
        assert_eq!(loaded.merged["title"], "X");
        assert_eq!(loaded.merged["params"]["author"], "Jane");
    }

    #[test]
    fn save_to_default_directory_only_touches_changed_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("config").join("_default");
        fs::create_dir_all(&dir).unwrap();
        let hugo = dir.join("hugo.toml");
        let params = dir.join("params.toml");
        let hugo_orig = "title = \"X\"\nbaseURL = \"https://a\"\n";
        let params_orig = "author = \"Jane\"\n";
        fs::write(&hugo, hugo_orig).unwrap();
        fs::write(&params, params_orig).unwrap();
        let det = detection(&dir, HugoConfigKind::DefaultDirectory);

        let mut loaded = load(&det).unwrap();
        loaded.merged["baseURL"] = serde_json::Value::String("https://b".into());
        save(&det, &loaded.merged).unwrap();

        let new_hugo = fs::read_to_string(&hugo).unwrap();
        let new_params = fs::read_to_string(&params).unwrap();
        assert!(new_hugo.contains("https://b"));
        assert_eq!(
            new_params, params_orig,
            "params.toml must not have been touched"
        );
    }
}
