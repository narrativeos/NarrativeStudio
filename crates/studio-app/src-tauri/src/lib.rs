use studio_core::project::ProjectSummary;
use tauri::State;

/// Application state holding the storage layer.
pub struct AppState {
    // Will hold storage connection in PR-3
}

/// List all projects.
#[tauri::command]
fn list_projects(_state: State<AppState>) -> Result<Vec<ProjectSummary>, String> {
    // PR-3 will implement actual storage query
    Ok(vec![])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {})
        .invoke_handler(tauri::generate_handler![list_projects])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}