use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::ids::SiteId;

/// Persistent workspace state — the user's list of registered Hugo sites and
/// which one is currently active. Saved to `app_data_dir/workspace.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub sites: Vec<SiteRef>,
    pub active_site_id: Option<SiteId>,
}

/// Lightweight site descriptor stored in the workspace; the full `Site`
/// (config, theme, languages…) is only built when a site is opened.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SiteRef {
    pub id: SiteId,
    pub name: String,
    pub root_path: String,
    pub last_opened: DateTime<Utc>,
}
