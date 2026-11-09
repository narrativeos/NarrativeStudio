//! Tauri commands, grouped by domain. `lib.rs` keeps only the application
//! state and the process bootstrap; every `#[tauri::command]` lives here.

pub mod analyze;
pub mod import;
pub mod project;
pub mod report;
