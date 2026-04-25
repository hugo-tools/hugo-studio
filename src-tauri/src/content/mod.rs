//! Content layer: scan `content/`, classify pages and bundles, peek front
//! matter for tree summaries. Multi-language aware (§6.6). The full
//! ContentItem with body + form-renderable front-matter schema lands in M4.

pub mod archetype;
pub mod classify;
pub mod create;
pub mod document;
pub mod frontmatter;
pub mod language;
pub mod scan;
pub mod schema;
