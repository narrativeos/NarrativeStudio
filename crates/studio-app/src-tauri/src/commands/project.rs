//! Project CRUD commands.

use studio_core::project::{Project, ProjectSummary};
use studio_storage::{create, delete, list};
use tauri::State;
use uuid::Uuid;

use crate::AppState;

/// List all projects.
#[tauri::command]
pub(crate) fn list_projects(state: State<AppState>) -> Result<Vec<ProjectSummary>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    list(&storage.conn).map_err(|e| e.to_string())
}

/// Create a new project.
#[tauri::command]
pub(crate) fn create_project(state: State<AppState>, name: String) -> Result<Project, String> {
    let project = Project::new(name);
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    create(&storage.conn, &project).map_err(|e| e.to_string())
}

/// Delete a project and all of its associated data (documents, blocks, tokens,
/// entities, noun signals, and analysis results).
#[tauri::command]
pub(crate) fn delete_project(state: State<AppState>, project_id: String) -> Result<(), String> {
    let pid = Uuid::parse_str(&project_id).map_err(|e| e.to_string())?;
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    delete(&storage.conn, pid).map_err(|e| e.to_string())
}
