//! studio-storage: Data persistence layer for NarrativeStudio.
//!
//! Manages DuckDB connections, migrations, and repository patterns.

pub use studio_core::error::{Result, StudioError};

mod analysis_repo;
mod document_repo;
mod entity_registry;
mod migration;
mod project_repo;
mod report_repo;

pub use analysis_repo::{
    invalidate_project, list_concerns, load_cached, load_result, load_results_for_hash,
    save_analysis_bundle, save_concerns, save_result, DimensionRow, StoredResult,
};
pub use document_repo::{load_document, load_project_documents, save_document};
pub use entity_registry::{entity_stats_for_project, resolve_or_insert, stable_id_for};
pub use migration::run_migrations;
pub use project_repo::{create, delete, get, list};
pub use report_repo::{get_report, latest_report, mark_exported, save_report, StoredReport};
