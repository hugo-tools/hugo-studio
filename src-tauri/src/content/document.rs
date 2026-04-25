//! Read / write a Hugo content file as a structured (FrontMatter, Body)
//! pair while preserving the on-disk format byte-for-byte for parts that
//! the user did not touch (per §6.2).

use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::config::json::JsonCodec;
use crate::config::toml::TomlCodec;
use crate::config::yaml::YamlCodec;
use crate::config::ConfigCodec;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FrontMatterFormat {
    Toml,
    Yaml,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ContentDocument {
    pub format: FrontMatterFormat,
    pub front_matter: serde_json::Value,
    pub body: String,
}

/// Internal representation of how the file is laid out on disk so we can
/// reassemble it after partial edits without dropping any bytes.
#[derive(Debug, Clone)]
struct Layout<'a> {
    leading: &'a str, // BOM / blank lines before the FM open delimiter
    open: &'a str,    // "---\n" / "---\r\n" / "+++\n" / "+++\r\n" / ""
    fm_inner: &'a str,
    close: &'a str, // "---\n" / "---\r\n" / "+++\n" / "+++\r\n" / "}\n" / ""
    body: &'a str,
    format: FrontMatterFormat,
}

pub fn read(path: &Path) -> AppResult<ContentDocument> {
    let raw = std::fs::read_to_string(path)?;
    let layout = parse_layout(&raw)?;
    let fm_value = match layout.format {
        FrontMatterFormat::Toml => TomlCodec::parse(layout.fm_inner)?,
        FrontMatterFormat::Yaml => YamlCodec::parse(layout.fm_inner)?,
        FrontMatterFormat::Json => JsonCodec::parse(layout.fm_inner)?,
    };
    Ok(ContentDocument {
        format: layout.format,
        front_matter: fm_value,
        body: layout.body.to_string(),
    })
}

/// Persist `(new_fm, new_body)` to `path` while leaving anything the user
/// did not change byte-identical with what was on disk before.
pub fn save(path: &Path, new_fm: &serde_json::Value, new_body: &str) -> AppResult<()> {
    let raw = std::fs::read_to_string(path)?;
    let layout = parse_layout(&raw)?;

    let new_fm_inner = match layout.format {
        FrontMatterFormat::Toml => TomlCodec::apply_changes(layout.fm_inner, new_fm)?,
        FrontMatterFormat::Yaml => YamlCodec::apply_changes(layout.fm_inner, new_fm)?,
        FrontMatterFormat::Json => JsonCodec::apply_changes(layout.fm_inner, new_fm)?,
    };

    // Reassemble: leading + open + fm_inner + close + body. Body keeps its
    // original surrounding whitespace by virtue of the slice layout.body
    // already including the leading newline(s) the user had.
    let mut out = String::with_capacity(raw.len() + 64);
    out.push_str(layout.leading);
    out.push_str(layout.open);
    out.push_str(&new_fm_inner);
    out.push_str(layout.close);
    out.push_str(new_body);
    if !new_body.ends_with('\n') && raw.ends_with('\n') {
        out.push('\n');
    }

    if out == raw {
        return Ok(()); // nothing actually changed; skip the write
    }
    atomic_write(path, out.as_bytes())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bak");
    let tmp = path.with_extension(format!("{ext}.tmp"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).map_err(AppError::from)
}

fn parse_layout(raw: &str) -> AppResult<Layout<'_>> {
    let bom_len = if raw.starts_with('\u{feff}') { 3 } else { 0 };
    let after_bom = &raw[bom_len..];

    // Skip leading blank lines (count by bytes since they're ASCII).
    let mut whitespace_end = 0;
    for (i, ch) in after_bom.char_indices() {
        match ch {
            '\n' | '\r' | '\t' | ' ' => whitespace_end = i + ch.len_utf8(),
            _ => break,
        }
    }
    let leading = &raw[..bom_len + whitespace_end];
    let rest_offset = bom_len + whitespace_end;
    let rest = &raw[rest_offset..];

    if let Some((open, fm_inner, close, body, fmt)) =
        try_delimited(rest, "---", FrontMatterFormat::Yaml)
    {
        return Ok(Layout {
            leading,
            open,
            fm_inner,
            close,
            body,
            format: fmt,
        });
    }
    if let Some((open, fm_inner, close, body, fmt)) =
        try_delimited(rest, "+++", FrontMatterFormat::Toml)
    {
        return Ok(Layout {
            leading,
            open,
            fm_inner,
            close,
            body,
            format: fmt,
        });
    }
    if rest.starts_with('{') {
        if let Some((open, fm_inner, close, body)) = try_json(rest) {
            return Ok(Layout {
                leading,
                open,
                fm_inner,
                close,
                body,
                format: FrontMatterFormat::Json,
            });
        }
    }

    // No front matter: treat the whole document as body, default to YAML
    // for any *new* fields we may emit.
    Ok(Layout {
        leading,
        open: "",
        fm_inner: "",
        close: "",
        body: rest,
        format: FrontMatterFormat::Yaml,
    })
}

fn try_delimited<'a>(
    rest: &'a str,
    delim: &str,
    fmt: FrontMatterFormat,
) -> Option<(&'a str, &'a str, &'a str, &'a str, FrontMatterFormat)> {
    // Accept either LF or CRLF after the opening delimiter. The closing
    // delimiter MUST sit at the start of its own line.
    let (open, after_open) = if let Some(after) = rest.strip_prefix(&format!("{delim}\r\n")) {
        (&rest[..delim.len() + 2], after)
    } else if let Some(after) = rest.strip_prefix(&format!("{delim}\n")) {
        (&rest[..delim.len() + 1], after)
    } else {
        return None;
    };

    // Locate the close delimiter. The pattern is "\r\n<delim>" or
    // "\n<delim>" — we keep the line-ending bytes inside `fm_inner` so that
    // a parser like toml_edit (which is happy to round-trip docs that end
    // in \n) round-trips byte-identically when nothing changed.
    let (inner_len, close_offset) = if let Some(pos) = after_open.find(&format!("\r\n{delim}")) {
        (pos + 2, pos + 2) // include the trailing \r\n in fm_inner
    } else if let Some(pos) = after_open.find(&format!("\n{delim}")) {
        (pos + 1, pos + 1) // include the trailing \n
    } else {
        return None;
    };

    // close = delim + its own line ending (LF, CRLF, or EOF).
    let after_close_offset = close_offset + delim.len();
    let close_end = if let Some(rest_after) = after_open.get(after_close_offset..) {
        if let Some(stripped) = rest_after.strip_prefix("\r\n") {
            after_close_offset + 2 + (stripped.len() - stripped.len()) // 2 chars consumed
        } else if rest_after.starts_with('\n') {
            after_close_offset + 1
        } else {
            after_close_offset
        }
    } else {
        after_close_offset
    };

    let fm_inner = &after_open[..inner_len];
    let close = &after_open[close_offset..close_end];
    let body = &after_open[close_end..];

    Some((open, fm_inner, close, body, fmt))
}

fn try_json(rest: &str) -> Option<(&str, &str, &str, &str)> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut end = None;
    for (i, ch) in rest.char_indices() {
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
    let end = end?;
    // Consume the line ending that follows the closing brace so the body
    // starts on its own line (matches the TOML/YAML conventions).
    let after_close = if rest[end..].starts_with("\r\n") {
        end + 2
    } else if rest[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };
    let fm_inner = &rest[..end];
    let close = &rest[end..after_close];
    let body = &rest[after_close..];
    // For JSON FM there's no separate open delimiter — the braces are part
    // of the FM. The "close" slice carries the trailing newline so reassembly
    // restores it without polluting the body.
    Some(("", fm_inner, close, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_and_read(content: &str) -> (TempDir, std::path::PathBuf, ContentDocument) {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("post.md");
        fs::write(&p, content).unwrap();
        let doc = read(&p).unwrap();
        (tmp, p, doc)
    }

    #[test]
    fn reads_yaml_front_matter() {
        let src = "---\ntitle: Hello\ndraft: true\n---\nbody here\n";
        let (_t, _p, doc) = write_and_read(src);
        assert_eq!(doc.format, FrontMatterFormat::Yaml);
        assert_eq!(doc.front_matter["title"], "Hello");
        assert_eq!(doc.body, "body here\n");
    }

    #[test]
    fn reads_toml_front_matter() {
        let src = "+++\ntitle = \"Hello\"\ndraft = true\n+++\nbody\n";
        let (_t, _p, doc) = write_and_read(src);
        assert_eq!(doc.format, FrontMatterFormat::Toml);
        assert_eq!(doc.front_matter["title"], "Hello");
        assert_eq!(doc.body, "body\n");
    }

    #[test]
    fn reads_json_front_matter() {
        let src = "{\n  \"title\": \"Hello\",\n  \"draft\": true\n}\nbody\n";
        let (_t, _p, doc) = write_and_read(src);
        assert_eq!(doc.format, FrontMatterFormat::Json);
        assert_eq!(doc.front_matter["title"], "Hello");
        assert_eq!(doc.body, "body\n");
    }

    #[test]
    fn no_front_matter_treats_whole_file_as_body() {
        let src = "# Heading\nNo FM here\n";
        let (_t, _p, doc) = write_and_read(src);
        assert_eq!(doc.body, src);
        assert!(doc.front_matter.is_null() || doc.front_matter == serde_json::json!({}));
    }

    #[test]
    fn roundtrip_yaml_no_changes_is_byte_identical() {
        let src = "---\n# top comment\ntitle: Hello\ndraft: false\n---\nbody\n";
        let (_t, p, doc) = write_and_read(src);
        save(&p, &doc.front_matter, &doc.body).unwrap();
        let after = fs::read_to_string(&p).unwrap();
        assert_eq!(after, src);
    }

    #[test]
    fn roundtrip_toml_change_title_only_diff_is_one_line() {
        let src = "+++\n# header comment\ntitle = \"Hello\"\ndraft = true\ndate = 2026-04-25\n+++\nbody preserved\n";
        let (_t, p, doc) = write_and_read(src);
        let mut fm = doc.front_matter.clone();
        fm["title"] = serde_json::Value::String("Renamed".into());
        save(&p, &fm, &doc.body).unwrap();
        let after = fs::read_to_string(&p).unwrap();
        assert!(after.contains("# header comment"));
        let differing: Vec<_> = src
            .lines()
            .zip(after.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(
            differing.len(),
            1,
            "expected exactly one line diff: {:?}",
            differing
        );
        assert!(differing[0].1.contains("Renamed"));
    }

    #[test]
    fn roundtrip_yaml_change_title_only_diff_is_one_line() {
        let src = "---\n# header comment\ntitle: Hello\ndraft: true\n---\nbody\n";
        let (_t, p, doc) = write_and_read(src);
        let mut fm = doc.front_matter.clone();
        fm["title"] = serde_json::Value::String("Renamed".into());
        save(&p, &fm, &doc.body).unwrap();
        let after = fs::read_to_string(&p).unwrap();
        assert!(after.contains("# header comment"));
        let differing: Vec<_> = src
            .lines()
            .zip(after.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(differing.len(), 1);
    }

    #[test]
    fn body_only_change_does_not_touch_front_matter() {
        let src = "---\n# header\ntitle: Hello\n---\noriginal body\n";
        let (_t, p, doc) = write_and_read(src);
        save(&p, &doc.front_matter, "rewritten body\n").unwrap();
        let after = fs::read_to_string(&p).unwrap();
        assert!(after.contains("# header"));
        assert!(after.contains("title: Hello"));
        assert!(after.contains("rewritten body"));
        assert!(!after.contains("original body"));
    }

    #[test]
    fn crlf_line_endings_are_preserved_for_unmodified_content() {
        let src = "---\r\ntitle: Hello\r\n---\r\nbody\r\n";
        let (_t, p, doc) = write_and_read(src);
        save(&p, &doc.front_matter, &doc.body).unwrap();
        let after = fs::read_to_string(&p).unwrap();
        assert_eq!(after, src);
    }

    #[test]
    fn bom_is_preserved_through_round_trip() {
        let src = "\u{feff}---\ntitle: Hello\n---\nbody\n";
        let (_t, p, doc) = write_and_read(src);
        save(&p, &doc.front_matter, &doc.body).unwrap();
        let after = fs::read_to_string(&p).unwrap();
        assert_eq!(after, src);
    }
}
