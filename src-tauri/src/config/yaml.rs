use regex::Regex;

use crate::config::ConfigCodec;
use crate::error::{AppError, AppResult};

pub struct YamlCodec;

impl ConfigCodec for YamlCodec {
    fn parse(source: &str) -> AppResult<serde_json::Value> {
        serde_yaml::from_str(source).map_err(|e| AppError::Serde(e.to_string()))
    }

    fn apply_changes(original: &str, new_data: &serde_json::Value) -> AppResult<String> {
        let original_data: serde_json::Value =
            serde_yaml::from_str(original).map_err(|e| AppError::Serde(e.to_string()))?;

        if &original_data == new_data {
            return Ok(original.to_string());
        }

        // M2 strategy (§6.2 minimum): if every change is a top-level *scalar*
        // value mutation, we patch the source text line-by-line so comments
        // and structure of the rest of the file survive byte-for-byte.
        // Anything else (added / removed keys, nested edits, type changes)
        // falls back to a clean re-serialize, accepting comment loss.
        if let Some(scalar_only_diffs) = top_level_scalar_diffs(&original_data, new_data) {
            let mut output = original.to_string();
            for (key, value) in scalar_only_diffs {
                output = patch_top_level_scalar(&output, &key, &value)?;
            }
            return Ok(output);
        }

        // Fallback.
        serde_yaml::to_string(new_data).map_err(|e| AppError::Serde(e.to_string()))
    }
}

/// Returns Some(changes) iff every difference between `old` and `new` is
/// (a) at the top level of a JSON object and (b) a scalar-to-scalar change
/// (string/number/bool/null) on a key that already existed in `old`.
fn top_level_scalar_diffs(
    old: &serde_json::Value,
    new: &serde_json::Value,
) -> Option<Vec<(String, serde_json::Value)>> {
    let old_obj = old.as_object()?;
    let new_obj = new.as_object()?;

    if old_obj.len() != new_obj.len() {
        return None;
    }
    if old_obj.keys().any(|k| !new_obj.contains_key(k)) {
        return None;
    }

    let mut diffs = Vec::new();
    for (k, new_v) in new_obj {
        let old_v = old_obj.get(k)?;
        if old_v == new_v {
            continue;
        }
        if !is_scalar(old_v) || !is_scalar(new_v) {
            return None;
        }
        diffs.push((k.clone(), new_v.clone()));
    }
    Some(diffs)
}

fn is_scalar(v: &serde_json::Value) -> bool {
    matches!(
        v,
        serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_)
    )
}

fn patch_top_level_scalar(source: &str, key: &str, value: &serde_json::Value) -> AppResult<String> {
    // Match: start of line, no indentation (top-level), key, optional spaces,
    //        colon, optional spaces, then the existing value (greedy until
    //        end of line or `#` comment).
    let pattern = format!(
        r"(?m)^({key}\s*:\s*)([^\r\n#]*?)(\s*(?:#[^\r\n]*)?)$",
        key = regex::escape(key)
    );
    let re = Regex::new(&pattern).map_err(|e| AppError::Internal(e.to_string()))?;
    let new_value = format_yaml_scalar(value);
    if !re.is_match(source) {
        return Err(AppError::Serde(format!(
            "could not locate top-level key `{key}` in YAML source"
        )));
    }
    let replaced = re
        .replace(source, |caps: &regex::Captures| {
            format!("{}{}{}", &caps[1], new_value, &caps[3])
        })
        .into_owned();
    Ok(replaced)
}

fn format_yaml_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            // Quote when the string would otherwise be misparsed as something else
            // (numbers, booleans, special tokens, leading/trailing whitespace, contains `:` / `#`).
            if needs_yaml_quoting(s) {
                format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            } else {
                s.clone()
            }
        }
        _ => unreachable!("non-scalar reached format_yaml_scalar"),
    }
}

fn needs_yaml_quoting(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if s.trim() != s {
        return true;
    }
    if s.parse::<f64>().is_ok() {
        return true;
    }
    if matches!(
        s.to_ascii_lowercase().as_str(),
        "true" | "false" | "null" | "yes" | "no" | "on" | "off" | "~"
    ) {
        return true;
    }
    s.chars()
        .any(|c| matches!(c, ':' | '#' | '&' | '*' | '!' | '|' | '>' | '%' | '@' | '`'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static str {
        "# Site identity\ntitle: My Hugo Site\nbaseURL: https://example.com/\nlanguageCode: en-us\ntheme: papermod\n# Pagination\npaginate: 10\nenableEmoji: true\n\nparams:\n  author: Jane Doe\n  description: A great site\n"
    }

    #[test]
    fn unchanged_save_is_byte_identical() {
        let src = fixture();
        let parsed = YamlCodec::parse(src).unwrap();
        let saved = YamlCodec::apply_changes(src, &parsed).unwrap();
        assert_eq!(saved, src);
    }

    #[test]
    fn changing_baseurl_only_changes_that_line() {
        let src = fixture();
        let mut parsed = YamlCodec::parse(src).unwrap();
        parsed["baseURL"] = serde_json::Value::String("https://new.example.com/".into());
        let saved = YamlCodec::apply_changes(src, &parsed).unwrap();

        let original_lines: Vec<&str> = src.lines().collect();
        let saved_lines: Vec<&str> = saved.lines().collect();
        assert_eq!(original_lines.len(), saved_lines.len());
        let differing: Vec<_> = original_lines
            .iter()
            .zip(saved_lines.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .collect();
        assert_eq!(differing.len(), 1, "expected one diff, got {differing:?}");
        assert!(saved.contains("new.example.com"));
        assert!(saved.contains("# Site identity"));
        assert!(saved.contains("# Pagination"));
    }

    #[test]
    fn quotes_strings_that_could_be_misread() {
        let src = "title: Hello\n";
        let mut parsed = YamlCodec::parse(src).unwrap();
        parsed["title"] = serde_json::Value::String("123".into());
        let saved = YamlCodec::apply_changes(src, &parsed).unwrap();
        assert!(saved.contains("\"123\""));
    }

    #[test]
    fn nested_change_falls_back_to_reserialize() {
        let src = fixture();
        let mut parsed = YamlCodec::parse(src).unwrap();
        parsed["params"]["author"] = serde_json::Value::String("Bob".into());
        let saved = YamlCodec::apply_changes(src, &parsed).unwrap();
        // The fallback re-serializes; we don't assert byte-identical, but the
        // new value must be present.
        assert!(saved.contains("Bob"));
    }
}
