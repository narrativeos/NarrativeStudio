//! studio-core: Core types and interfaces for NarrativeStudio.
//!
//! This crate defines the fundamental data models shared across all
//! NarrativeStudio crates. It contains no business logic — only types,
//! enums, and error definitions.

pub mod analysis;
pub mod concern;
pub mod document;
pub mod entity;
pub mod error;
pub mod noun_signal;
pub mod project;
pub mod report;

pub use analysis::{AnalysisDimension, AnalysisOptions, AnalysisResult, TaskTier};
pub use concern::{Concern, ConcernLocation, Severity};
pub use document::{DocumentData, SemanticBlock, Token};
pub use entity::{Entity, EntityCategory};
pub use error::{Result, StudioError};
pub use noun_signal::{NounSignal, NounSignalEvidence};
pub use project::{Project, ProjectSummary};
pub use report::Report;
