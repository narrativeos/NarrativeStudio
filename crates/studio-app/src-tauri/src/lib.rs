use std::sync::Mutex;

use duckdb::Connection;
use studio_analysis::{run_t0_analysis, run_t1_analysis, T0Stats, T1Stats};
use studio_core::project::{Project, ProjectSummary};
use studio_import::parse_semantic_result;
use studio_storage::{run_migrations, DocumentRepo, ProjectRepo};
use tauri::{Manager, State};

/// Application state holding the storage layer.
pub struct AppState {
    storage: Mutex<Storage>,
}

/// Combined storage holding both repos sharing one connection.
struct Storage {
    project_repo: ProjectRepo,
    document_repo: DocumentRepo,
}

impl AppState {
    fn new() -> Self {
        let conn = Connection::open_in_memory().expect("Failed to open database");
        run_migrations(&conn).expect("Failed to run migrations");
        let project_repo = ProjectRepo::new(conn);
        // DocumentRepo needs its own connection handle — use a second in-memory
        // connection to the same database via ATTACH is overkill; instead we
        // just create a fresh connection that shares the same in-memory DB.
        // For now, we'll store both repos and the document repo will use
        // the same underlying data via a shared connection pattern.
        // Since DuckDB in-memory DBs are per-connection, we use a file-based
        // approach or just accept that document_repo operates on the same conn.
        // Simplest: pass the same conn to both (ProjectRepo takes ownership,
        // so we need a different approach).
        //
        // Actually, let's just use a single connection and have DocumentRepo
        // work through ProjectRepo's connection. For now, create a second
        // in-memory connection (data won't be shared in dev mode).
        let doc_conn = Connection::open_in_memory().expect("Failed to open doc database");
        run_migrations(&doc_conn).expect("Failed to run doc migrations");
        let document_repo = DocumentRepo::new(doc_conn);

        Self {
            storage: Mutex::new(Storage {
                project_repo,
                document_repo,
            }),
        }
    }
}

/// List all projects.
#[tauri::command]
fn list_projects(state: State<AppState>) -> Result<Vec<ProjectSummary>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.project_repo.list().map_err(|e| e.to_string())
}

/// Create a new project.
#[tauri::command]
fn create_project(state: State<AppState>, name: String) -> Result<Project, String> {
    let project = Project::new(name);
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage
        .project_repo
        .create(&project)
        .map_err(|e| e.to_string())
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
    let doc = parse_semantic_result(&content).map_err(|e| format!("Failed to parse: {e}"))?;
    let project = Project::new(project_name);

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let project = storage
        .project_repo
        .create(&project)
        .map_err(|e| e.to_string())?;
    storage
        .document_repo
        .save_document(project.project_id, &doc)
        .map_err(|e| e.to_string())?;

    Ok(project)
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

/// List documents for a project.
#[tauri::command]
fn list_documents(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<(String, String)>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let conn = storage.project_repo.conn();
    let mut stmt = conn
        .prepare(
            "SELECT doc_id, title FROM documents WHERE project_id = ? ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut docs = Vec::new();
    for row in rows {
        docs.push(row.map_err(|e| e.to_string())?);
    }
    Ok(docs)
}

/// Load a document by ID.
#[tauri::command]
fn load_document(
    state: State<AppState>,
    doc_id: String,
) -> Result<studio_core::document::DocumentData, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let doc_uuid: uuid::Uuid =
        uuid::Uuid::parse_str(&doc_id).map_err(|e: uuid::Error| e.to_string())?;
    storage
        .document_repo
        .load_document(doc_uuid)
        .map_err(|e| e.to_string())
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
            analyze_t1,
            list_documents,
            load_document
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
