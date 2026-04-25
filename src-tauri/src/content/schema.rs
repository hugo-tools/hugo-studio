//! Per-section front-matter schema: a list of typed [`FieldDef`]s that the
//! frontend renders as form widgets. Built by combining the curated
//! standard-Hugo set with values inferred from the section's existing
//! content.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FieldType {
    String,
    Text,
    Number,
    Boolean,
    Date,
    DateTime,
    /// `Array<String>` rendered as a chip-style input. Combined with
    /// `enum_values` for autocomplete.
    Tags,
    /// `Array<String>` without autocomplete (e.g. `aliases`).
    StringArray,
    /// Catch-all for nested objects, arrays of objects, mixed-type
    /// arrays, and unknown shapes — edited as raw JSON.
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum UnknownFieldsPolicy {
    Preserve,
    Warn,
    Strip,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    pub key: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub enum_values: Option<Vec<String>>,
    pub group: Option<String>,
    pub hidden: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FrontMatterSchema {
    pub fields: Vec<FieldDef>,
    pub unknown_fields_policy: UnknownFieldsPolicy,
}

fn field(key: &str, label: &str, t: FieldType, group: &str) -> FieldDef {
    FieldDef {
        key: key.into(),
        label: label.into(),
        field_type: t,
        required: false,
        default: None,
        enum_values: None,
        group: Some(group.into()),
        hidden: false,
        description: None,
    }
}

/// Curated set of Hugo's standard top-level front-matter fields, grouped
/// for sensible form layout. Marked `required: false` everywhere because
/// Hugo is happy with any subset.
pub fn standard_fields() -> Vec<FieldDef> {
    vec![
        field("title", "Title", FieldType::String, "Basic"),
        field("date", "Date", FieldType::DateTime, "Basic"),
        field("draft", "Draft", FieldType::Boolean, "Basic"),
        field("description", "Description", FieldType::Text, "Basic"),
        field("summary", "Summary", FieldType::Text, "Basic"),
        field("tags", "Tags", FieldType::Tags, "Taxonomy"),
        field("categories", "Categories", FieldType::Tags, "Taxonomy"),
        field("keywords", "Keywords", FieldType::StringArray, "Taxonomy"),
        field("series", "Series", FieldType::Tags, "Taxonomy"),
        field("slug", "Slug", FieldType::String, "Routing"),
        field("url", "URL", FieldType::String, "Routing"),
        field("aliases", "Aliases", FieldType::StringArray, "Routing"),
        field("linkTitle", "Link title", FieldType::String, "Routing"),
        field("lastmod", "Last modified", FieldType::DateTime, "Schedule"),
        field(
            "publishDate",
            "Publish date",
            FieldType::DateTime,
            "Schedule",
        ),
        field("expiryDate", "Expiry date", FieldType::DateTime, "Schedule"),
        field("weight", "Weight", FieldType::Number, "Order"),
        field("layout", "Layout", FieldType::String, "Rendering"),
        field("type", "Type", FieldType::String, "Rendering"),
        field("outputs", "Outputs", FieldType::StringArray, "Rendering"),
        field("headless", "Headless", FieldType::Boolean, "Rendering"),
    ]
}

const STANDARD_KEYS: &[&str] = &[
    "title",
    "date",
    "draft",
    "description",
    "summary",
    "tags",
    "categories",
    "keywords",
    "series",
    "slug",
    "url",
    "aliases",
    "linkTitle",
    "lastmod",
    "publishDate",
    "expiryDate",
    "weight",
    "layout",
    "type",
    "outputs",
    "headless",
];

/// Pick a [`FieldType`] for a value. Returns `None` for `null` so that
/// inference can disambiguate from neighbours that have a real value.
pub(crate) fn classify_value(v: &serde_json::Value) -> Option<FieldType> {
    match v {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(_) => Some(FieldType::Boolean),
        serde_json::Value::Number(_) => Some(FieldType::Number),
        serde_json::Value::String(s) => {
            if looks_like_datetime(s) {
                Some(FieldType::DateTime)
            } else if looks_like_date(s) {
                Some(FieldType::Date)
            } else if s.contains('\n') || s.len() > 120 {
                Some(FieldType::Text)
            } else {
                Some(FieldType::String)
            }
        }
        serde_json::Value::Array(items) => {
            if items.iter().all(|i| i.is_string()) {
                Some(FieldType::StringArray)
            } else {
                Some(FieldType::Json)
            }
        }
        serde_json::Value::Object(_) => Some(FieldType::Json),
    }
}

fn looks_like_date(s: &str) -> bool {
    s.len() >= 10
        && s[..4].chars().filter(|c| c.is_ascii_digit()).count() == 4
        && s.as_bytes()[4] == b'-'
        && s.as_bytes()[7] == b'-'
}

fn looks_like_datetime(s: &str) -> bool {
    looks_like_date(s) && (s.contains('T') || s.contains(' '))
}

/// Result of [`infer_section_schema`] — both the schema itself plus the
/// bag of distinct values per array<string> field, useful as autocomplete
/// hints for the form.
#[derive(Debug, Clone)]
pub struct InferredSchema {
    pub schema: FrontMatterSchema,
}

/// Infer the schema for a section by reading every page's front matter.
/// Files that fail to parse are silently skipped — the schema is a hint,
/// not an authority.
pub fn infer_section_schema(
    content_root: &Path,
    section: Option<&str>,
) -> AppResult<InferredSchema> {
    let mut type_votes: BTreeMap<String, BTreeMap<FieldType, u32>> = BTreeMap::new();
    let mut tag_values: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    let mut seen_keys: std::collections::BTreeSet<String> = Default::default();

    let scan_root = match section {
        Some(s) => content_root.join(s),
        None => content_root.to_path_buf(),
    };
    if scan_root.is_dir() {
        walk_section_files(&scan_root, &mut |path| {
            let Ok(raw) = std::fs::read_to_string(path) else {
                return;
            };
            let Ok(doc) = crate::content::document::read(path) else {
                let _ = raw; // keep variable used
                return;
            };
            let serde_json::Value::Object(map) = doc.front_matter else {
                return;
            };
            for (key, value) in map {
                seen_keys.insert(key.clone());
                if let Some(ft) = classify_value(&value) {
                    let votes = type_votes.entry(key.clone()).or_default();
                    *votes.entry(ft).or_insert(0) += 1;
                }
                if let serde_json::Value::Array(items) = &value {
                    let bucket = tag_values.entry(key.clone()).or_default();
                    for item in items {
                        if let Some(s) = item.as_str() {
                            bucket.insert(s.to_string());
                        }
                    }
                }
            }
        });
    }

    // Start from the curated standard set, then enrich with inferred enum
    // values (so e.g. `tags`/`categories` get autocomplete from existing
    // posts).
    let mut fields: Vec<FieldDef> = standard_fields()
        .into_iter()
        .map(|mut f| {
            if matches!(f.field_type, FieldType::Tags | FieldType::StringArray) {
                if let Some(values) = tag_values.get(&f.key) {
                    if !values.is_empty() {
                        f.enum_values = Some(values.iter().cloned().collect());
                    }
                }
            }
            f
        })
        .collect();

    // Append custom fields the user/theme has introduced.
    for key in &seen_keys {
        if STANDARD_KEYS.contains(&key.as_str()) {
            continue;
        }
        let votes = type_votes.get(key);
        let majority = votes
            .and_then(|v| v.iter().max_by_key(|(_, c)| **c).map(|(t, _)| *t))
            .unwrap_or(FieldType::Json);

        let mut f = FieldDef {
            key: key.clone(),
            label: humanise(key),
            field_type: majority,
            required: false,
            default: None,
            enum_values: None,
            group: Some("Custom".into()),
            hidden: false,
            description: None,
        };
        if matches!(f.field_type, FieldType::Tags | FieldType::StringArray) {
            if let Some(values) = tag_values.get(key) {
                if !values.is_empty() {
                    f.enum_values = Some(values.iter().cloned().collect());
                }
            }
        }
        fields.push(f);
    }

    Ok(InferredSchema {
        schema: FrontMatterSchema {
            fields,
            unknown_fields_policy: UnknownFieldsPolicy::Preserve,
        },
    })
}

pub(crate) fn humanise(key: &str) -> String {
    // very small helper: split camelCase / snake_case and capitalise
    let mut out = String::with_capacity(key.len() + 4);
    let mut prev_lower = false;
    for ch in key.chars() {
        if ch == '_' || ch == '-' {
            out.push(' ');
            prev_lower = false;
            continue;
        }
        if ch.is_uppercase() && prev_lower {
            out.push(' ');
        }
        out.push(ch);
        prev_lower = ch.is_lowercase();
    }
    let mut chars = out.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => out,
    }
}

fn walk_section_files(dir: &Path, visit: &mut dyn FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_section_files(&path, visit);
            continue;
        }
        if !crate::content::classify::is_content_file(&path) {
            continue;
        }
        visit(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn standard_fields_are_present_and_optional() {
        let fields = standard_fields();
        assert!(fields.iter().any(|f| f.key == "title"));
        assert!(fields.iter().any(|f| f.key == "draft"));
        assert!(fields.iter().any(|f| f.key == "tags"));
        assert!(fields.iter().all(|f| !f.required));
    }

    #[test]
    fn classify_picks_native_types() {
        assert_eq!(
            classify_value(&serde_json::json!(42)),
            Some(FieldType::Number)
        );
        assert_eq!(
            classify_value(&serde_json::json!(true)),
            Some(FieldType::Boolean)
        );
        assert_eq!(
            classify_value(&serde_json::json!("2026-04-25")),
            Some(FieldType::Date)
        );
        assert_eq!(
            classify_value(&serde_json::json!("2026-04-25T10:00:00Z")),
            Some(FieldType::DateTime)
        );
        assert_eq!(
            classify_value(&serde_json::json!(["a", "b"])),
            Some(FieldType::StringArray)
        );
        assert_eq!(
            classify_value(&serde_json::json!([1, "x"])),
            Some(FieldType::Json)
        );
    }

    #[test]
    fn humanise_camel_and_snake() {
        // camelCase becomes "Word Word" (each capital letter is a word boundary
        // — we don't try to lowercase the second word; "Base URL" stays
        // uppercase because URL is already uppercase in source).
        assert_eq!(humanise("baseURL"), "Base URL");
        assert_eq!(humanise("publish_date"), "Publish date");
        assert_eq!(humanise("authorName"), "Author Name");
    }

    #[test]
    fn infer_collects_custom_field_and_tag_values() {
        let tmp = TempDir::new().unwrap();
        let posts = tmp.path().join("posts");
        fs::create_dir_all(&posts).unwrap();
        fs::write(
            posts.join("a.md"),
            "---\ntitle: A\ntags: [rust, hugo]\nauthor: Jane\n---\nbody\n",
        )
        .unwrap();
        fs::write(
            posts.join("b.md"),
            "---\ntitle: B\ntags: [rust, tauri]\nauthor: Bob\n---\nbody\n",
        )
        .unwrap();

        let inferred = infer_section_schema(tmp.path(), Some("posts")).unwrap();
        let by_key: std::collections::HashMap<_, _> = inferred
            .schema
            .fields
            .iter()
            .map(|f| (f.key.clone(), f))
            .collect();

        // Tags from standard list now have enum values populated.
        let tags = by_key.get("tags").expect("tags field present");
        let tag_values = tags.enum_values.as_ref().expect("tag enum values");
        assert!(tag_values.contains(&"rust".to_string()));
        assert!(tag_values.contains(&"hugo".to_string()));
        assert!(tag_values.contains(&"tauri".to_string()));

        // Custom field discovered.
        let author = by_key.get("author").expect("author field inferred");
        assert_eq!(author.field_type, FieldType::String);
        assert_eq!(author.group.as_deref(), Some("Custom"));
    }
}
