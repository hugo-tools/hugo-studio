use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
}

#[tauri::command]
#[specta::specta]
pub fn health_check() -> HealthStatus {
    HealthStatus {
        status: "ready".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}
