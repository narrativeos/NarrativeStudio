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
pub mod knowsphere;
pub mod noun_signal;
pub mod project;
pub mod report;

pub use analysis::{AnalysisDimension, AnalysisOptions, AnalysisResult, TaskTier};
pub use concern::{Concern, ConcernLocation, Severity};
pub use document::{DocumentData, SemanticBlock, Token};
pub use entity::{Entity, EntityCategory, EntityStat};
pub use error::{Result, StudioError};
pub use knowsphere::{
    CartesianPosition, EdgeAttributes, EdgeType, KnowledgeEdge, KnowledgeNode, KnowsphereDoc,
    LayerScope, Narrative, NarrativeAttributes, NarrativeType, NodeType, ATMOSPHERE_MAX,
    ATMOSPHERE_MIN, KNOWSPHERE_VERSION,
};
pub use noun_signal::{NounSignal, NounSignalEvidence};
pub use project::{Project, ProjectSummary};
pub use report::Report;
