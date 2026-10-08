//! studio-storage: Data persistence layer for NarrativeStudio.
//!
//! Manages DuckDB connections, migrations, and repository patterns.

pub use studio_core::error::{Result, StudioError};

mod analysis_repo;
mod document_repo;
mod migration;
mod project_repo;
mod report_repo;

pub use analysis_repo::{
    invalidate_project, list_concerns, load_cached, load_result, save_concerns, save_result,
    StoredResult,
};
pub use document_repo::{load_document, save_document};
pub use migration::run_migrations;
pub use project_repo::{create, delete, get, list};
pub use report_repo::{get_report, latest_report, mark_exported, save_report, StoredReport};
