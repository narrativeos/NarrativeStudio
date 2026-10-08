//! studio-analysis: Analysis engine for NarrativeStudio.
//!
//! Implements the T0/T1/T2 analysis task pipeline:
//! - T0: pure statistics (words, sentences, POS, phrases, readability)
//! - T1: structure (sections, characters, narrative arc, co-occurrences)
//! - Plot: plotlines and coherence, from the entity flow
//! - Pacing: how far the tension moves, and where it drags
//! - Conflict: what the story fights over, and how hard
//! - Assessment: rule-based dimension scores and recommendations

pub use studio_core::error::Result;

mod assessment;
mod conflict;
mod hash;
mod pacing;
mod plot;
mod segment;
mod t0;
mod t1;
mod text;

pub use assessment::{run_assessment, Assessment, DimensionScore, Recommendation};
pub use conflict::{
    run_conflict_analysis, ConflictAbsence, ConflictPoint, ConflictStats, ConflictType,
};
pub use hash::{document_hash, ANALYSIS_VERSION};
pub use pacing::{run_pacing_analysis, DragSection, FlatStretch, PacingPoint, PacingStats};
pub use plot::{run_plot_analysis, PlotStats, Plotline, PlotlineStage, TurningPoint};
pub use t0::{run_t0_analysis, Readability, RepeatedPhrase, T0Stats};
pub use t1::{run_t1_analysis, ArcPoint, CharacterProfile, NarrativeArc, SectionInfo, T1Stats};
