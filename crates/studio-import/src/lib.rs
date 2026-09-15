//! studio-import: Data import layer for NarrativeStudio.
//!
//! Responsible for parsing TraceView output files and converting them
//! into internal data models.

pub use studio_core::error::Result;

mod enrich;
mod popo;
mod semantic;
mod term;

pub use popo::{parse_popo_toc, TocEntry};
pub use semantic::{parse_semantic_result, parse_semantic_result_enriched};
pub use term::{parse_term_result, Term};
