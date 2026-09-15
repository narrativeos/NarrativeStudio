//! studio-analysis: Analysis engine for NarrativeStudio.
//!
//! Implements the T0/T1/T2 analysis task pipeline:
//! - T0: pure statistics (words, sentences, POS, phrases, readability)
//! - T1: structure (sections, characters, narrative arc, co-occurrences)
//! - Assessment: rule-based dimension scores and recommendations

pub use studio_core::error::Result;

mod assessment;
mod t0;
mod t1;
mod text;

pub use assessment::{run_assessment, Assessment, DimensionScore, Recommendation};
pub use t0::{run_t0_analysis, Readability, RepeatedPhrase, T0Stats};
pub use t1::{run_t1_analysis, ArcPoint, CharacterProfile, NarrativeArc, SectionInfo, T1Stats};
