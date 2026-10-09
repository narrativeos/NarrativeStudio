//! studio-analysis: Analysis engine for NarrativeStudio.
//!
//! Implements the T0/T1/T2 analysis task pipeline:
//! - T0: pure statistics (words, sentences, POS, phrases, readability)
//! - T1: structure (sections, characters, narrative arc, co-occurrences)
//! - Plot: plotlines and coherence, from the entity flow
//! - Pacing: how far the tension moves, and where it drags
//! - Conflict: what the story fights over, and how hard
//! - Theme: which terms the document keeps coming back to
//! - Setting: where the story happens, and whether the reader is told
//! - Cliché: which stock phrases the text reaches for, and how often
//! - Dialogue: how much of the text is spoken, and where it is spoken
//! - Explicit: what a reader might want warned about, and how strong the words are
//! - Structure: which classic shape the document's own landmarks fit
//! - Timeline: where the text says when, and whether its dates agree
//!   (with 历法: the era table, the sexagenary wheel and the lunar conversion behind it)
//! - Assessment: rule-based dimension scores and recommendations

pub use studio_core::error::Result;

mod assessment;
mod calendar;
mod cliche;
mod conflict;
mod dialogue;
mod explicit;
mod hash;
mod lexicon;
mod pacing;
mod plot;
mod segment;
mod setting;
mod structure;
mod t0;
mod t1;
mod text;
mod theme;
mod timeline;

pub use assessment::{run_assessment, Assessment, DimensionScore, Recommendation};
pub use cliche::{
    run_cliche_analysis, ClicheHit, ClicheIssue, ClichePhrase, ClichePoint, ClicheStats,
};
pub use conflict::{
    run_conflict_analysis, ConflictAbsence, ConflictPoint, ConflictStats, ConflictType,
};
pub use dialogue::{
    run_dialogue_analysis, DialogueIssue, DialoguePoint, DialogueSample, DialogueStats,
};
pub use explicit::{
    run_explicit_analysis, ExplicitHit, ExplicitIssue, ExplicitPoint, ExplicitStats, ExplicitTerm,
};
pub use hash::{document_hash, ANALYSIS_VERSION};
pub use pacing::{run_pacing_analysis, DragSection, FlatStretch, PacingPoint, PacingStats};
pub use plot::{run_plot_analysis, PlotStats, Plotline, PlotlineStage, TurningPoint};
pub use setting::{run_setting_analysis, Place, Scene, SettingIssue, SettingPoint, SettingStats};
pub use structure::{
    run_structure_analysis, StructureAct, StructureBeat, StructureIssue, StructureLandmark,
    StructureModel, StructureStats,
};
pub use t0::{run_t0_analysis, Readability, RepeatedPhrase, T0Stats};
pub use t1::{run_t1_analysis, ArcPoint, CharacterProfile, NarrativeArc, SectionInfo, T1Stats};
pub use theme::{run_theme_analysis, Theme, ThemeStats};
pub use timeline::{
    run_timeline_analysis, TimelineIssue, TimelineMarker, TimelinePoint, TimelineStats,
    TimelineStep,
};
