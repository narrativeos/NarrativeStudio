use std::sync::Mutex;

use duckdb::Connection;
use serde::Serialize;
use studio_analysis::{run_t0_analysis, run_t1_analysis, T0Stats, T1Stats};
use studio_core::document::DocumentData;
use studio_core::project::{Project, ProjectSummary};
use studio_import::{parse_semantic_result, parse_semantic_result_enriched};
use studio_storage::{create, delete, get, list, load_document, run_migrations, save_document};
use tauri::{Emitter, Manager, State};
use uuid::Uuid;

/// Application state holding the storage layer.
pub struct AppState {
    storage: Mutex<Storage>,
}

/// Combined storage holding the shared connection.
struct Storage {
    conn: Connection,
}

/// Progress event payload sent to the frontend during import.
#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum ImportProgress {
    #[serde(rename = "status")]
    Status { message: String },
    #[serde(rename = "progress")]
    Progress {
        current: u64,
        total: u64,
        message: String,
    },
    #[serde(rename = "done")]
    Done { project_id: String },
    #[serde(rename = "error")]
    Error { message: String },
}

impl AppState {
    fn new(app_handle: &tauri::AppHandle) -> Result<Self, String> {
        // Get the app data directory for file-based persistence
        let data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {e}"))?;
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data dir: {e}"))?;
        let db_path = data_dir.join("narrative_studio.duckdb");

        // Fail fast with a clear message if another instance holds the lock.
        #[cfg(unix)]
        check_db_lock(&db_path)?;

        let conn = Connection::open(&db_path).map_err(|e| {
            format!(
                "Failed to open database at {}.\n\n{}\n\n\
                 If another instance of NarrativeStudio is running, please close it first.",
                db_path.display(),
                e
            )
        })?;
        run_migrations(&conn).map_err(|e| format!("Failed to run migrations: {e}"))?;

        Ok(Self {
            storage: Mutex::new(Storage { conn }),
        })
    }
}

/// Pre-flight check: fail fast with a clear message if the database file is
/// already locked by another process, instead of blocking in `Connection::open`.
///
/// DuckDB (on Unix) takes an exclusive `flock` on the database file, so a
/// non-blocking `flock` probe tells us whether another instance holds it.
#[cfg(unix)]
fn check_db_lock(db_path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::prelude::AsRawFd;

    // Nothing to probe if the file does not exist yet (DuckDB will create it).
    if !db_path.exists() {
        return Ok(());
    }

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(db_path)
        .map_err(|e| format!("Failed to open database file for lock check: {e}"))?;

    // SAFETY: `file` owns a valid fd for the duration of the call.
    let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
            return Err(format!(
                "The database is locked by another running instance of NarrativeStudio:\n{}\n\nPlease close the other instance first.",
                db_path.display()
            ));
        }
        return Err(format!("Failed to check database lock: {err}"));
    }

    // Release immediately; DuckDB acquires its own lock right after.
    // SAFETY: same fd as above.
    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
    Ok(())
}

/// List all projects.
#[tauri::command]
fn list_projects(state: State<AppState>) -> Result<Vec<ProjectSummary>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    list(&storage.conn).map_err(|e| e.to_string())
}

/// Create a new project.
#[tauri::command]
fn create_project(state: State<AppState>, name: String) -> Result<Project, String> {
    let project = Project::new(name);
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    create(&storage.conn, &project).map_err(|e| e.to_string())
}

/// Delete a project and all of its associated data (documents, blocks, tokens,
/// entities, noun signals, and analysis results).
#[tauri::command]
fn delete_project(state: State<AppState>, project_id: String) -> Result<(), String> {
    let pid = Uuid::parse_str(&project_id).map_err(|e| e.to_string())?;
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    delete(&storage.conn, pid).map_err(|e| e.to_string())
}

/// Import a TraceView project folder (containing project.json).
///
/// Expected folder structure:
/// ```text
/// {project_path}/
/// ├── project.json
/// ├── mineru/...
/// ├── popo/...
/// └── semantic/semantic_result.json
/// ```
#[tauri::command]
async fn import_project(
    app_handle: tauri::AppHandle,
    project_path: String,
) -> Result<Project, String> {
    // Run the blocking import on the async runtime's blocking pool so the UI
    // thread stays responsive, emitting `import-progress` events as it goes.
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle.state::<AppState>();
        run_import(&app_handle, &state, &project_path)
    })
    .await
    .map_err(|e| format!("Import task failed: {e}"))?
}

/// Blocking body of [`import_project`]: reads the folder, parses the semantic
/// result, and writes it to the database, emitting progress events along the way.
fn run_import(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    project_path: &str,
) -> Result<Project, String> {
    let project_dir = std::path::Path::new(project_path);

    // 1. Validate: project.json must exist
    let project_json_path = project_dir.join("project.json");
    if !project_json_path.exists() {
        return Err(format!(
            "Not a valid TraceView project folder: {} not found",
            project_json_path.display()
        ));
    }

    // 2. Read project.json for metadata
    app_handle
        .emit(
            "import-progress",
            ImportProgress::Status {
                message: "读取 project.json ...".into(),
            },
        )
        .ok();

    let project_json_content = std::fs::read_to_string(&project_json_path)
        .map_err(|e| format!("Failed to read project.json: {e}"))?;
    let project_meta: serde_json::Value = serde_json::from_str(&project_json_content)
        .map_err(|e| format!("Failed to parse project.json: {e}"))?;

    let project_name = project_meta
        .get("ProjectName")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Project")
        .to_string();

    // 3. Find semantic_result.json
    app_handle
        .emit(
            "import-progress",
            ImportProgress::Status {
                message: "查找 semantic/semantic_result.json ...".into(),
            },
        )
        .ok();

    let semantic_path = project_dir.join("semantic").join("semantic_result.json");
    if !semantic_path.exists() {
        return Err(format!(
            "semantic/semantic_result.json not found in {}",
            project_path
        ));
    }

    // 4. Read and parse semantic_result.json
    let file_size_mb = semantic_path
        .metadata()
        .map(|m| m.len() / 1024 / 1024)
        .unwrap_or(0);
    app_handle
        .emit(
            "import-progress",
            ImportProgress::Status {
                message: format!("读取 semantic_result.json ({} MB) ...", file_size_mb),
            },
        )
        .ok();

    let content = std::fs::read_to_string(&semantic_path)
        .map_err(|e| format!("Failed to read semantic_result.json: {e}"))?;

    // 4b. Read optional enrichment files:
    //     - popo/popo_result.json        -> TOC (chapter/section structure)
    //     - semantic/term_result.json    -> domain terms (noun signals)
    let popo_path = project_dir.join("popo").join("popo_result.json");
    let popo_content = if popo_path.exists() {
        app_handle
            .emit(
                "import-progress",
                ImportProgress::Status {
                    message: "读取 popo TOC (章节结构) ...".into(),
                },
            )
            .ok();
        Some(
            std::fs::read_to_string(&popo_path)
                .map_err(|e| format!("Failed to read popo_result.json: {e}"))?,
        )
    } else {
        None
    };

    let term_path = project_dir.join("semantic").join("term_result.json");
    let term_content = if term_path.exists() {
        app_handle
            .emit(
                "import-progress",
                ImportProgress::Status {
                    message: "读取 term_result.json (领域术语) ...".into(),
                },
            )
            .ok();
        Some(
            std::fs::read_to_string(&term_path)
                .map_err(|e| format!("Failed to read term_result.json: {e}"))?,
        )
    } else {
        None
    };

    app_handle
        .emit(
            "import-progress",
            ImportProgress::Status {
                message: format!("解析 JSON ({} MB) ...", content.len() / 1024 / 1024),
            },
        )
        .ok();

    let doc = parse_semantic_result_enriched(
        &content,
        popo_content.as_deref(),
        term_content.as_deref(),
    )
    .map_err(|e| format!("Failed to parse semantic data: {e}"))?;

    let block_count = doc.blocks.len() as u64;
    app_handle
        .emit(
            "import-progress",
            ImportProgress::Status {
                message: format!(
                    "解析完成: {} 个文本块, {} 字",
                    block_count, doc.total_char_count
                ),
            },
        )
        .ok();

    // 5. Create project in database
    app_handle
        .emit(
            "import-progress",
            ImportProgress::Status {
                message: "创建项目记录 ...".into(),
            },
        )
        .ok();

    let mut project = Project::new(&project_name);
    project.source_path = Some(project_path.to_string());

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let project = create(&storage.conn, &project).map_err(|e| e.to_string())?;

    // 6. Save document
    app_handle
        .emit(
            "import-progress",
            ImportProgress::Progress {
                current: 0,
                total: block_count,
                message: "写入数据库 ...".into(),
            },
        )
        .ok();

    save_document(
        &storage.conn,
        project.project_id,
        &doc,
        Some(&|current, total| {
            app_handle
                .emit(
                    "import-progress",
                    ImportProgress::Progress {
                        current,
                        total,
                        message: "写入数据库 ...".into(),
                    },
                )
                .ok();
        }),
    )
    .map_err(|e| e.to_string())?;

    // 7. Done
    app_handle
        .emit(
            "import-progress",
            ImportProgress::Done {
                project_id: project.project_id.to_string(),
            },
        )
        .ok();

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

/// Analysis result for a single project (T0 + T1).
#[derive(Debug, Clone, Serialize)]
pub struct ProjectAnalysis {
    pub project_id: String,
    pub name: String,
    pub t0: T0Stats,
    pub t1: T1Stats,
}

/// Load all documents of a project and merge them into a single `DocumentData`.
///
/// A project currently holds one document, but this merges any number so the
/// analysis stays correct if a project ever contains multiple documents.
fn load_project_document(conn: &Connection, project_id: Uuid) -> Result<DocumentData, String> {
    let mut stmt = conn
        .prepare("SELECT doc_id FROM documents WHERE project_id = ? ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;
    let doc_ids: Vec<String> = stmt
        .query_map([project_id.to_string()], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut merged = DocumentData {
        doc_id: Uuid::new_v4(),
        title: String::new(),
        blocks: Vec::new(),
        total_word_count: 0,
        total_char_count: 0,
    };

    for doc_id in doc_ids {
        let uuid = Uuid::parse_str(&doc_id).map_err(|e| e.to_string())?;
        let doc = load_document(conn, uuid).map_err(|e| e.to_string())?;
        merged.total_word_count += doc.total_word_count;
        merged.total_char_count += doc.total_char_count;
        merged.blocks.extend(doc.blocks);
    }

    Ok(merged)
}

/// Core: run T0/T1 analysis on a project by loading its documents from the DB.
fn analyze_project_inner(
    state: &State<AppState>,
    project_id: &str,
) -> Result<ProjectAnalysis, String> {
    let pid = Uuid::parse_str(project_id).map_err(|e| e.to_string())?;
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let project = get(&storage.conn, pid).map_err(|e| e.to_string())?;
    let doc = load_project_document(&storage.conn, pid)?;
    let t0 = run_t0_analysis(&doc).map_err(|e| e.to_string())?;
    let t1 = run_t1_analysis(&doc).map_err(|e| e.to_string())?;
    Ok(ProjectAnalysis {
        project_id: pid.to_string(),
        name: project.name,
        t0,
        t1,
    })
}

/// Run T0/T1 analysis on a single project (data loaded from the database).
///
/// Marked `async` so the DB load + T0/T1 analysis run on Tauri's async runtime
/// (a worker thread) rather than the main/UI thread. Keeping this heavy work off
/// the main thread is what prevents the frontend from freezing on large docs.
#[tauri::command]
async fn analyze_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectAnalysis, String> {
    analyze_project_inner(&state, &project_id)
}

/// Run T0/T1 analysis on multiple projects for cross-project comparison.
///
/// `async` for the same reason as [`analyze_project`]: keep the heavy work off
/// the main/UI thread so the frontend stays responsive.
#[tauri::command]
async fn analyze_projects(
    state: State<'_, AppState>,
    project_ids: Vec<String>,
) -> Result<Vec<ProjectAnalysis>, String> {
    project_ids
        .iter()
        .map(|id| analyze_project_inner(&state, id))
        .collect()
}

/// List documents for a project.
#[tauri::command]
fn list_documents(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<(String, String)>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let mut stmt = storage
        .conn
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
fn load_document_cmd(
    state: State<AppState>,
    doc_id: String,
) -> Result<studio_core::document::DocumentData, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    let doc_uuid: uuid::Uuid =
        uuid::Uuid::parse_str(&doc_id).map_err(|e: uuid::Error| e.to_string())?;
    load_document(&storage.conn, doc_uuid).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = match AppState::new(app.handle()) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("NarrativeStudio 启动失败:\n{e}");
                    // 在 macOS 上显示原生错误对话框
                    #[cfg(target_os = "macos")]
                    {
                        let _ = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(format!(
                                "display dialog \"NarrativeStudio 启动失败\\n\\n{}\" buttons {{\"确定\"}} default-button \"确定\" with icon stop",
                                e.replace('"', "\\\"")
                            ))
                            .output();
                    }
                    std::process::exit(1);
                }
            };
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_projects,
            create_project,
            delete_project,
            import_project,
            analyze_t0,
            analyze_t1,
            analyze_project,
            analyze_projects,
            list_documents,
            load_document_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
