//! studio-analysis: Analysis engine for NarrativeStudio.
//!
//! Implements the T0/T1/T2 analysis task pipeline.

pub use studio_core::error::Result;

mod t0;

pub use t0::{run_t0_analysis, T0Stats};
