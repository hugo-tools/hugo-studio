use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LanguageStrategy {
    /// No `languages` block in config — site is monolingual.
    Mono,
    /// Translations live as `posts/hello.en.md`, `posts/hello.it.md`.
    Filename,
    /// Translations live under `content/<lang>/posts/hello.md`.
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    pub code: String,
    pub name: String,
    pub weight: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LanguageInfo {
    pub strategy: LanguageStrategy,
    pub languages: Vec<Language>,
    pub default_language: String,
}

/// Decide how the site stores translations.
///
/// Inputs:
/// - `merged` — the loaded config JSON (root level).
/// - `content_root` — `<site>/content`. Optional because in tests we may
///   not have a real filesystem; if `None`, only the config informs the
///   decision.
pub fn detect(merged: &serde_json::Value, content_root: Option<&Path>) -> LanguageInfo {
    let langs_obj = merged
        .get("languages")
        .and_then(|v| v.as_object())
        .filter(|o| !o.is_empty());

    let Some(langs_obj) = langs_obj else {
        return LanguageInfo {
            strategy: LanguageStrategy::Mono,
            languages: vec![],
            default_language: merged
                .get("defaultContentLanguage")
                .and_then(|v| v.as_str())
                .unwrap_or("en")
                .to_string(),
        };
    };

    let default_language = merged
        .get("defaultContentLanguage")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| langs_obj.keys().next().map(String::as_str).unwrap_or("en"))
        .to_string();

    let in_subdir = merged
        .get("defaultContentLanguageInSubdir")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let lang_codes: Vec<String> = langs_obj.keys().cloned().collect();

    let strategy = if in_subdir {
        LanguageStrategy::Directory
    } else if let Some(root) = content_root {
        let any_lang_dir = std::fs::read_dir(root)
            .ok()
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|entry| {
                    entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                        && lang_codes
                            .iter()
                            .any(|c| c.eq_ignore_ascii_case(&entry.file_name().to_string_lossy()))
                })
            })
            .unwrap_or(false);
        if any_lang_dir {
            LanguageStrategy::Directory
        } else {
            LanguageStrategy::Filename
        }
    } else {
        LanguageStrategy::Filename
    };

    let mut languages: Vec<Language> = lang_codes
        .iter()
        .map(|code| {
            let info = langs_obj.get(code).and_then(|v| v.as_object());
            let name = info
                .and_then(|m| m.get("languageName"))
                .and_then(|v| v.as_str())
                .unwrap_or(code)
                .to_string();
            let weight = info
                .and_then(|m| m.get("weight"))
                .and_then(|v| v.as_i64())
                .map(|i| i as i32)
                .unwrap_or(0);
            Language {
                code: code.clone(),
                name,
                weight,
            }
        })
        .collect();
    languages.sort_by(|a, b| a.weight.cmp(&b.weight).then_with(|| a.code.cmp(&b.code)));

    LanguageInfo {
        strategy,
        languages,
        default_language,
    }
}

/// Strip a `.<lang>` suffix from a file stem, returning `(base_stem, lang)`
/// when matched. If no recognised language suffix is found, returns
/// `(stem, None)`.
pub fn split_filename_lang<'a>(stem: &'a str, languages: &[String]) -> (&'a str, Option<&'a str>) {
    if let Some(dot_pos) = stem.rfind('.') {
        let candidate = &stem[dot_pos + 1..];
        if languages.iter().any(|l| l.eq_ignore_ascii_case(candidate)) {
            return (&stem[..dot_pos], Some(candidate));
        }
    }
    (stem, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_languages_block_is_mono() {
        let cfg = json!({"title": "x"});
        let info = detect(&cfg, None);
        assert_eq!(info.strategy, LanguageStrategy::Mono);
        assert!(info.languages.is_empty());
    }

    #[test]
    fn empty_languages_block_is_mono() {
        let cfg = json!({"languages": {}});
        assert_eq!(detect(&cfg, None).strategy, LanguageStrategy::Mono);
    }

    #[test]
    fn default_content_language_in_subdir_forces_directory() {
        let cfg = json!({
            "languages": {"en": {}, "it": {}},
            "defaultContentLanguageInSubdir": true,
        });
        assert_eq!(detect(&cfg, None).strategy, LanguageStrategy::Directory);
    }

    #[test]
    fn no_subdir_flag_falls_back_to_filename_when_no_lang_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("post.md"), "").unwrap();
        let cfg = json!({"languages": {"en": {}, "it": {}}});
        assert_eq!(
            detect(&cfg, Some(tmp.path())).strategy,
            LanguageStrategy::Filename
        );
    }

    #[test]
    fn presence_of_lang_dir_picks_directory_strategy() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("en")).unwrap();
        std::fs::create_dir_all(tmp.path().join("it")).unwrap();
        let cfg = json!({"languages": {"en": {}, "it": {}}});
        assert_eq!(
            detect(&cfg, Some(tmp.path())).strategy,
            LanguageStrategy::Directory
        );
    }

    #[test]
    fn languages_are_sorted_by_weight_then_code() {
        let cfg = json!({
            "languages": {
                "it": {"weight": 2, "languageName": "Italiano"},
                "en": {"weight": 1, "languageName": "English"},
            }
        });
        let info = detect(&cfg, None);
        assert_eq!(info.languages[0].code, "en");
        assert_eq!(info.languages[1].code, "it");
        assert_eq!(info.languages[0].name, "English");
    }

    #[test]
    fn split_filename_lang_strips_known_suffix() {
        let langs = vec!["en".to_string(), "it".to_string()];
        assert_eq!(
            split_filename_lang("hello.en", &langs),
            ("hello", Some("en"))
        );
        assert_eq!(split_filename_lang("hello", &langs), ("hello", None));
        assert_eq!(split_filename_lang("hello.fr", &langs), ("hello.fr", None));
    }
}
