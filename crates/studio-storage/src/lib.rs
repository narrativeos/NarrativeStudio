//! studio-storage: Data persistence layer for NarrativeStudio.
//!
//! Manages DuckDB connections, migrations, and repository patterns.

pub use studio_core::error::{Result, StudioError};

mod document_repo;
mod migration;
mod project_repo;

pub use document_repo::{load_document, save_document};
pub use migration::run_migrations;
pub use project_repo::{create, delete, get, list};
