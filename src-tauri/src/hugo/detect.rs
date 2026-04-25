use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{AppError, AppResult};

/// Which kind of config layout was found in the site root.
///
/// Order matches the priority in [`detect`]: hugo.* > config.* > config/_default/.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HugoConfigKind {
    HugoToml,
    HugoYaml,
    HugoJson,
    ConfigToml,
    ConfigYaml,
    ConfigJson,
    DefaultDirectory,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DetectionInfo {
    pub kind: HugoConfigKind,
    /// Absolute path to the discovered file (single-file kinds) or to the
    /// `config/_default/` directory (DefaultDirectory).
    pub config_path: String,
}

const SINGLE_FILE_CANDIDATES: &[(&str, HugoConfigKind)] = &[
    ("hugo.toml", HugoConfigKind::HugoToml),
    ("hugo.yaml", HugoConfigKind::HugoYaml),
    ("hugo.yml", HugoConfigKind::HugoYaml),
    ("hugo.json", HugoConfigKind::HugoJson),
    ("config.toml", HugoConfigKind::ConfigToml),
    ("config.yaml", HugoConfigKind::ConfigYaml),
    ("config.yml", HugoConfigKind::ConfigYaml),
    ("config.json", HugoConfigKind::ConfigJson),
];

/// Inspect `root` and decide whether it looks like a Hugo site, per §6.1.
///
/// A path qualifies if at least one of:
/// - a `hugo.{toml,yaml,yml,json}` or `config.{toml,yaml,yml,json}` file exists in root
/// - a `config/_default/` directory exists and contains `hugo.*` or `config.*`
///
/// The first matching candidate wins; nothing here parses the actual config
/// content — that's the job of M2.
pub fn detect(root: &Path) -> AppResult<DetectionInfo> {
    if !root.exists() {
        return Err(AppError::NotAHugoSite(root.display().to_string()));
    }
    if !root.is_dir() {
        return Err(AppError::NotADirectory(root.display().to_string()));
    }

    for (name, kind) in SINGLE_FILE_CANDIDATES {
        let candidate = root.join(name);
        if candidate.is_file() {
            return Ok(DetectionInfo {
                kind: *kind,
                config_path: candidate.display().to_string(),
            });
        }
    }

    let default_dir = root.join("config").join("_default");
    if default_dir.is_dir() {
        for entry in std::fs::read_dir(&default_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if (name_str.starts_with("hugo.") || name_str.starts_with("config."))
                && has_known_extension(&name_str)
            {
                return Ok(DetectionInfo {
                    kind: HugoConfigKind::DefaultDirectory,
                    config_path: default_dir.display().to_string(),
                });
            }
        }
    }

    Err(AppError::NotAHugoSite(root.display().to_string()))
}

fn has_known_extension(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".toml", ".yaml", ".yml", ".json"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(dir: &Path, name: &str) {
        fs::write(dir.join(name), b"").expect("write fixture file");
    }

    #[test]
    fn detects_hugo_toml() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "hugo.toml");
        let info = detect(tmp.path()).unwrap();
        assert_eq!(info.kind, HugoConfigKind::HugoToml);
        assert!(info.config_path.ends_with("hugo.toml"));
    }

    #[test]
    fn detects_config_yaml() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "config.yaml");
        let info = detect(tmp.path()).unwrap();
        assert_eq!(info.kind, HugoConfigKind::ConfigYaml);
    }

    #[test]
    fn detects_config_yml_alias() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "config.yml");
        let info = detect(tmp.path()).unwrap();
        assert_eq!(info.kind, HugoConfigKind::ConfigYaml);
    }

    #[test]
    fn detects_default_directory() {
        let tmp = TempDir::new().unwrap();
        let default_dir = tmp.path().join("config").join("_default");
        fs::create_dir_all(&default_dir).unwrap();
        touch(&default_dir, "hugo.toml");
        let info = detect(tmp.path()).unwrap();
        assert_eq!(info.kind, HugoConfigKind::DefaultDirectory);
        assert!(info.config_path.ends_with("_default"));
    }

    #[test]
    fn prefers_root_hugo_toml_over_default_dir() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "hugo.toml");
        let default_dir = tmp.path().join("config").join("_default");
        fs::create_dir_all(&default_dir).unwrap();
        touch(&default_dir, "hugo.toml");
        let info = detect(tmp.path()).unwrap();
        assert_eq!(info.kind, HugoConfigKind::HugoToml);
    }

    #[test]
    fn rejects_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let err = detect(tmp.path()).unwrap_err();
        assert!(matches!(err, AppError::NotAHugoSite(_)));
    }

    #[test]
    fn rejects_default_dir_with_only_unrelated_files() {
        let tmp = TempDir::new().unwrap();
        let default_dir = tmp.path().join("config").join("_default");
        fs::create_dir_all(&default_dir).unwrap();
        touch(&default_dir, "menus.yaml"); // not hugo.* / config.*
        let err = detect(tmp.path()).unwrap_err();
        assert!(matches!(err, AppError::NotAHugoSite(_)));
    }

    #[test]
    fn rejects_nonexistent_path() {
        let err = detect(Path::new("/this/does/not/exist")).unwrap_err();
        assert!(matches!(err, AppError::NotAHugoSite(_)));
    }
}
