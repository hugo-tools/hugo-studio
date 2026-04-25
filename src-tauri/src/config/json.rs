use crate::config::ConfigCodec;
use crate::error::{AppError, AppResult};

pub struct JsonCodec;

impl ConfigCodec for JsonCodec {
    fn parse(source: &str) -> AppResult<serde_json::Value> {
        serde_json::from_str(source).map_err(AppError::from)
    }

    fn apply_changes(original: &str, new_data: &serde_json::Value) -> AppResult<String> {
        // serde_json with `preserve_order` already keeps key order; if the
        // new data is structurally identical we can short-circuit and emit
        // the original byte-for-byte (preserves indentation / trailing newline).
        let original_value: serde_json::Value = serde_json::from_str(original)?;
        if &original_value == new_data {
            return Ok(original.to_string());
        }

        // Otherwise pretty-print with 2-space indent — the de-facto Hugo
        // default. We don't try to recover the original indentation style;
        // JSON config is uncommon enough in Hugo land that this is fine for v1.
        let mut buf = Vec::with_capacity(original.len());
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"  ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
        new_data.serialize(&mut ser).map_err(AppError::from)?;
        let mut out = String::from_utf8(buf).map_err(|e| AppError::Internal(e.to_string()))?;
        if original.ends_with('\n') && !out.ends_with('\n') {
            out.push('\n');
        }
        Ok(out)
    }
}

use serde::Serialize;

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static str {
        r#"{
  "title": "My Hugo Site",
  "baseURL": "https://example.com/",
  "languageCode": "en-us",
  "theme": "papermod",
  "params": {
    "author": "Jane Doe"
  }
}
"#
    }

    #[test]
    fn unchanged_save_is_byte_identical() {
        let src = fixture();
        let parsed = JsonCodec::parse(src).unwrap();
        let saved = JsonCodec::apply_changes(src, &parsed).unwrap();
        assert_eq!(saved, src);
    }

    #[test]
    fn changing_baseurl_keeps_key_order() {
        let src = fixture();
        let mut parsed = JsonCodec::parse(src).unwrap();
        parsed["baseURL"] = serde_json::Value::String("https://new.example.com/".into());
        let saved = JsonCodec::apply_changes(src, &parsed).unwrap();
        // Order: title, baseURL, languageCode, theme, params.
        let title_at = saved.find("\"title\"").unwrap();
        let base_at = saved.find("\"baseURL\"").unwrap();
        let lang_at = saved.find("\"languageCode\"").unwrap();
        assert!(title_at < base_at && base_at < lang_at);
        assert!(saved.contains("new.example.com"));
    }
}
