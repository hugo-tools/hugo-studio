//! Site config layer.
//!
//! Reads the Hugo configuration in any of TOML / YAML / JSON, exposes the
//! merged shape as canonical `serde_json::Value`, and writes back changes
//! while preserving the original file's formatting / comments / key order
//! whenever possible. See §6.2 of the project prompt for the constraints.

pub mod cascade;
pub mod format;
pub mod json;
pub mod toml;
pub mod yaml;

use crate::error::AppResult;

/// Strategy contract every concrete parser must satisfy.
pub trait ConfigCodec {
    /// Parse `source` into the canonical merged JSON shape.
    fn parse(source: &str) -> AppResult<serde_json::Value>;

    /// Produce a new source-string for `new_data` that preserves as much of
    /// `original` (comments, key order, whitespace) as possible.
    fn apply_changes(original: &str, new_data: &serde_json::Value) -> AppResult<String>;
}
