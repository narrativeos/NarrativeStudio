//! studio-report: Report generation for NarrativeStudio.
//!
//! Aggregates analysis results into exportable reports (Markdown, JSON).

pub use studio_core::error::Result;

mod markdown;

pub use markdown::{render_markdown, ReportInput};
