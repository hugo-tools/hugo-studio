//! App-wide settings (M9).
//!
//! Distinct from `Workspace` (which lists registered sites): this struct
//! holds preferences that affect how the app *behaves*, not what it
//! contains. Lives in `app_data_dir/settings.json` next to the workspace
//! file.

pub mod settings;
