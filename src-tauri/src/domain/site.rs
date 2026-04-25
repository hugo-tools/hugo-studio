use serde::{Deserialize, Serialize};
use specta::Type;

use super::ids::SiteId;
use super::workspace::SiteRef;
use crate::hugo::detect::DetectionInfo;

/// Runtime view of an opened site. M1 only fills the structural pieces
/// (id, paths, detection info); config / theme / languages arrive in M2 / M5 / M3.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: SiteId,
    pub name: String,
    pub root_path: String,
    pub content_root: String,
    pub detection: DetectionInfo,
}

impl Site {
    pub fn from_ref_with_detection(site_ref: &SiteRef, detection: DetectionInfo) -> Self {
        let trimmed = site_ref.root_path.trim_end_matches(['/', '\\']);
        let separator = if site_ref.root_path.contains('\\') {
            '\\'
        } else {
            '/'
        };
        let content_root = format!("{trimmed}{separator}content");
        Self {
            id: site_ref.id,
            name: site_ref.name.clone(),
            root_path: site_ref.root_path.clone(),
            content_root,
            detection,
        }
    }
}
