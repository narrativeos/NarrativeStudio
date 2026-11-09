use std::sync::Mutex;

use duckdb::Connection;
use serde::Serialize;
use studio_storage::run_migrations;
use tauri::Manager;

mod commands;

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
            commands::project::list_projects,
            commands::project::create_project,
            commands::project::delete_project,
            commands::import::import_project,
            commands::analyze::analyze_t0,
            commands::analyze::analyze_t1,
            commands::analyze::analyze_project,
            commands::analyze::analyze_projects,
            commands::analyze::list_documents,
            commands::analyze::load_document_cmd,
            commands::report::report_generate,
            commands::report::report_export,
            commands::report::export_knowsphere
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
