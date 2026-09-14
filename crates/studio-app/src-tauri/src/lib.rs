use std::sync::Mutex;

use duckdb::Connection;
use studio_analysis::{run_t0_analysis, run_t1_analysis, T0Stats, T1Stats};
use studio_core::project::{Project, ProjectSummary};
use studio_import::parse_semantic_result;
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

/// Import a TraceView semantic_result.json file.
#[tauri::command]
fn import_semantic_file(
    state: State<AppState>,
    file_path: String,
    project_name: String,
) -> Result<Project, String> {
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {e}"))?;
    let _doc = parse_semantic_result(&content).map_err(|e| format!("Failed to parse: {e}"))?;
    let project = Project::new(project_name);
    let repo = state.project_repo.lock().map_err(|e| e.to_string())?;
    repo.create(&project).map_err(|e| e.to_string())
}

/// Run T0 analysis on a semantic_result.json file.
#[tauri::command]
fn analyze_t0(file_path: String) -> Result<T0Stats, String> {
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {e}"))?;
    let doc = parse_semantic_result(&content).map_err(|e| format!("Failed to parse: {e}"))?;
    run_t0_analysis(&doc).map_err(|e| e.to_string())
}

/// Run T1 structural analysis on a semantic_result.json file.
#[tauri::command]
fn analyze_t1(file_path: String) -> Result<T1Stats, String> {
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {e}"))?;
    let doc = parse_semantic_result(&content).map_err(|e| format!("Failed to parse: {e}"))?;
    run_t1_analysis(&doc).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            app.manage(AppState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_projects,
            create_project,
            import_semantic_file,
            analyze_t0,
            analyze_t1
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
