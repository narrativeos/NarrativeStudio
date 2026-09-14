//! Error types for NarrativeStudio.

use thiserror::Error;

/// Unified error type for all NarrativeStudio operations.
#[derive(Error, Debug)]
pub enum StudioError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Import error: {0}")]
    Import(String),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, StudioError>;
