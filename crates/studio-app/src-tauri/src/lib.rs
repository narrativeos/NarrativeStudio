use std::sync::Mutex;

use duckdb::Connection;
use studio_core::project::{Project, ProjectSummary};
use studio_storage::{run_migrations, ProjectRepo};
use tauri::{Manager, State};

/// Application state holding the storage layer.
pub struct AppState {
    project_repo: Mutex<ProjectRepo>,
}

impl AppState {
    fn new() -> Self {
        let conn = Connection::open_in_memory().expect("Failed to open database");
        run_migrations(&conn).expect("Failed to run migrations");
        Self {
            project_repo: Mutex::new(ProjectRepo::new(conn)),
        }
    }
}

/// List all projects.
#[tauri::command]
fn list_projects(state: State<AppState>) -> Result<Vec<ProjectSummary>, String> {
    let repo = state.project_repo.lock().map_err(|e| e.to_string())?;
    repo.list().map_err(|e| e.to_string())
}

/// Create a new project.
#[tauri::command]
fn create_project(state: State<AppState>, name: String) -> Result<Project, String> {
    let project = Project::new(name);
    let repo = state.project_repo.lock().map_err(|e| e.to_string())?;
    repo.create(&project).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            app.manage(AppState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_projects, create_project])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
