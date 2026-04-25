//! Cheap front-matter peek — extract enough to populate the content tree
//! (title, draft, date) without committing to a full deserialize round.
//!
//! M4 will replace this with a proper bidirectional FrontMatter type.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FrontMatterSummary {
    pub title: Option<String>,
    pub draft: bool,
    /// Free-form date string as it appears in the front matter; the UI
    /// formats it. Parsing here would force us to support every date
    /// shape Hugo accepts before we even render the tree.
    pub date: Option<String>,
}

/// Parse only the leading front-matter block of a content file.
/// Tolerates missing / malformed FM by returning the default summary.
pub fn peek(source: &str) -> FrontMatterSummary {
    let trimmed = source.trim_start_matches('\u{feff}'); // strip BOM
    let trimmed = trimmed.trim_start_matches(['\n', '\r']);

    if let Some(rest) = trimmed.strip_prefix("---\n") {
        return parse_block(rest, "\n---", parse_yaml);
    }
    if let Some(rest) = trimmed.strip_prefix("---\r\n") {
        return parse_block(rest, "\r\n---", parse_yaml);
    }
    if let Some(rest) = trimmed.strip_prefix("+++\n") {
        return parse_block(rest, "\n+++", parse_toml);
    }
    if let Some(rest) = trimmed.strip_prefix("+++\r\n") {
        return parse_block(rest, "\r\n+++", parse_toml);
    }
    if trimmed.starts_with('{') {
        // JSON front matter is rare but legal; the closing brace ends the block.
        return parse_json_block(trimmed);
    }
    FrontMatterSummary::default()
}

fn parse_block(
    rest: &str,
    closing: &str,
    parser: fn(&str) -> Option<serde_json::Value>,
) -> FrontMatterSummary {
    let Some(end) = rest.find(closing) else {
        return FrontMatterSummary::default();
    };
    let block = &rest[..end];
    parser(block).map(summarize).unwrap_or_default()
}

fn parse_yaml(s: &str) -> Option<serde_json::Value> {
    serde_yaml::from_str(s).ok()
}

fn parse_toml(s: &str) -> Option<serde_json::Value> {
    let v: toml::Value = toml::from_str(s).ok()?;
    serde_json::to_value(v).ok()
}

fn parse_json_block(s: &str) -> FrontMatterSummary {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut end = None;
    for (i, ch) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(end) = end else {
        return FrontMatterSummary::default();
    };
    let block = &s[..end];
    serde_json::from_str(block)
        .ok()
        .map(summarize)
        .unwrap_or_default()
}

fn summarize(v: serde_json::Value) -> FrontMatterSummary {
    let obj = match v {
        serde_json::Value::Object(o) => o,
        _ => return FrontMatterSummary::default(),
    };
    FrontMatterSummary {
        title: obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        draft: obj.get("draft").and_then(|v| v.as_bool()).unwrap_or(false),
        date: obj.get("date").and_then(stringify_date),
    }
}

fn stringify_date(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_summary() {
        let src = "---\ntitle: Hello\ndraft: true\ndate: 2026-04-25\n---\nbody";
        let sum = peek(src);
        assert_eq!(sum.title.as_deref(), Some("Hello"));
        assert!(sum.draft);
        assert_eq!(sum.date.as_deref(), Some("2026-04-25"));
    }

    #[test]
    fn toml_summary() {
        let src = "+++\ntitle = \"Hello\"\ndraft = false\n+++\nbody";
        let sum = peek(src);
        assert_eq!(sum.title.as_deref(), Some("Hello"));
        assert!(!sum.draft);
    }

    #[test]
    fn json_summary() {
        let src = "{\n  \"title\": \"Hi\",\n  \"draft\": true\n}\nbody";
        let sum = peek(src);
        assert_eq!(sum.title.as_deref(), Some("Hi"));
        assert!(sum.draft);
    }

    #[test]
    fn no_front_matter_returns_default() {
        let sum = peek("# Just a heading\n");
        assert!(sum.title.is_none());
        assert!(!sum.draft);
    }

    #[test]
    fn malformed_front_matter_returns_default() {
        let sum = peek("---\nthis: is\n  : broken yaml :\n---\n");
        assert!(sum.title.is_none());
    }

    #[test]
    fn bom_is_tolerated() {
        let src = "\u{feff}---\ntitle: With BOM\n---\n";
        assert_eq!(peek(src).title.as_deref(), Some("With BOM"));
    }
}
