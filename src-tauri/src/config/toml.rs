use toml_edit::{value as tv, DocumentMut, Item};

use crate::config::ConfigCodec;
use crate::error::{AppError, AppResult};

pub struct TomlCodec;

impl ConfigCodec for TomlCodec {
    fn parse(source: &str) -> AppResult<serde_json::Value> {
        let doc: DocumentMut = source
            .parse()
            .map_err(|e: toml_edit::TomlError| AppError::Serde(e.to_string()))?;
        toml_to_json(&doc)
    }

    fn apply_changes(original: &str, new_data: &serde_json::Value) -> AppResult<String> {
        let mut doc: DocumentMut = original
            .parse()
            .map_err(|e: toml_edit::TomlError| AppError::Serde(e.to_string()))?;
        let new_obj = new_data
            .as_object()
            .ok_or_else(|| AppError::Serde("config root must be a JSON object".into()))?;

        let original_data = toml_to_json(&doc)?;
        let original_obj = original_data.as_object().cloned().unwrap_or_default();

        // Update / insert.
        for (k, v) in new_obj {
            let unchanged = original_obj.get(k).is_some_and(|prev| prev == v);
            if unchanged {
                continue; // preserve byte-identical formatting
            }
            doc[k] = json_value_to_toml_item(v)?;
        }

        // Remove keys present in original but absent from new_data.
        let removed: Vec<String> = original_obj
            .keys()
            .filter(|k| !new_obj.contains_key(*k))
            .cloned()
            .collect();
        for k in removed {
            doc.remove(&k);
        }

        Ok(doc.to_string())
    }
}

fn toml_to_json(doc: &DocumentMut) -> AppResult<serde_json::Value> {
    let s = doc.to_string();
    // toml_edit serializes back to TOML; round-trip via the regular `toml`
    // parser into JSON. This is fine because we use `toml_edit` only for the
    // *write* side — the *read* side just needs the canonical shape.
    let parsed: toml::Value = toml::from_str(&s).map_err(|e| AppError::Serde(e.to_string()))?;
    serde_json::to_value(parsed).map_err(AppError::from)
}

fn json_value_to_toml_item(v: &serde_json::Value) -> AppResult<Item> {
    match v {
        serde_json::Value::Null => Ok(tv("")), // TOML has no null; emit empty string
        serde_json::Value::Bool(b) => Ok(tv(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(tv(i))
            } else if let Some(f) = n.as_f64() {
                Ok(tv(f))
            } else {
                Err(AppError::Serde(format!("unsupported numeric value: {n}")))
            }
        }
        serde_json::Value::String(s) => Ok(tv(s)),
        serde_json::Value::Array(items) => {
            let mut arr = toml_edit::Array::new();
            for item in items {
                arr.push(json_value_to_toml_value(item)?);
            }
            Ok(Item::Value(toml_edit::Value::Array(arr)))
        }
        serde_json::Value::Object(map) => {
            let mut table = toml_edit::Table::new();
            for (k, v) in map {
                table.insert(k, json_value_to_toml_item(v)?);
            }
            Ok(Item::Table(table))
        }
    }
}

fn json_value_to_toml_value(v: &serde_json::Value) -> AppResult<toml_edit::Value> {
    match json_value_to_toml_item(v)? {
        Item::Value(val) => Ok(val),
        Item::Table(t) => {
            let mut inline = toml_edit::InlineTable::new();
            for (k, item) in t.iter() {
                if let Item::Value(val) = item {
                    inline.insert(k, val.clone());
                }
            }
            Ok(toml_edit::Value::InlineTable(inline))
        }
        _ => Err(AppError::Serde("unrepresentable nested array".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static str {
        r#"# Top-level config for the site
title = "My Hugo Site"
baseURL = "https://example.com/"
languageCode = "en-us"
theme = "papermod"

# Pagination
paginate = 10

[params]
description = "A great site"
author = "Jane Doe"
"#
    }

    #[test]
    fn unchanged_save_is_byte_identical() {
        let src = fixture();
        let parsed = TomlCodec::parse(src).unwrap();
        let saved = TomlCodec::apply_changes(src, &parsed).unwrap();
        assert_eq!(
            saved, src,
            "round-trip must be byte-identical when unchanged"
        );
    }

    #[test]
    fn changing_baseurl_only_changes_that_line() {
        let src = fixture();
        let mut parsed = TomlCodec::parse(src).unwrap();
        parsed["baseURL"] = serde_json::Value::String("https://new.example.com/".into());
        let saved = TomlCodec::apply_changes(src, &parsed).unwrap();

        let original_lines: Vec<&str> = src.lines().collect();
        let saved_lines: Vec<&str> = saved.lines().collect();
        assert_eq!(original_lines.len(), saved_lines.len());
        let mut differing = vec![];
        for (i, (a, b)) in original_lines.iter().zip(saved_lines.iter()).enumerate() {
            if a != b {
                differing.push((i, *a, *b));
            }
        }
        assert_eq!(
            differing.len(),
            1,
            "exactly one line should change: {:?}",
            differing
        );
        assert!(differing[0].2.contains("new.example.com"));
    }

    #[test]
    fn comments_survive_a_top_level_change() {
        let src = fixture();
        let mut parsed = TomlCodec::parse(src).unwrap();
        parsed["title"] = serde_json::Value::String("Renamed".into());
        let saved = TomlCodec::apply_changes(src, &parsed).unwrap();
        assert!(saved.contains("# Top-level config for the site"));
        assert!(saved.contains("# Pagination"));
    }

    #[test]
    fn removing_a_key_drops_only_that_key() {
        let src = fixture();
        let mut parsed = TomlCodec::parse(src).unwrap();
        parsed.as_object_mut().unwrap().remove("paginate");
        let saved = TomlCodec::apply_changes(src, &parsed).unwrap();
        assert!(!saved.contains("paginate"));
        assert!(saved.contains("baseURL"));
    }
}
