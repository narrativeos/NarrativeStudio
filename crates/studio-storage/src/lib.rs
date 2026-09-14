//! studio-storage: Data persistence layer for NarrativeStudio.
//!
//! Manages DuckDB connections, migrations, and repository patterns.

pub use studio_core::error::{Result, StudioError};

mod migration;
mod project_repo;

pub use migration::run_migrations;
pub use project_repo::ProjectRepo;
