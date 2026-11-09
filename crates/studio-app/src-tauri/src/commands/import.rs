//! TraceView folder import command.

use studio_core::project::Project;
use studio_import::parse_semantic_result_enriched;
use studio_storage::{create, save_document};
use tauri::{Emitter, Manager};

use crate::{AppState, ImportProgress};

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
pub(crate) async fn import_project(
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
pub(crate) fn run_import(
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

    let doc =
        parse_semantic_result_enriched(&content, popo_content.as_deref(), term_content.as_deref())
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
