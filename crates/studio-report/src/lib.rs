//! studio-report: Report generation for NarrativeStudio.
//!
//! Aggregates analysis results into exportable reports (Markdown, print-ready
//! HTML for PDF, JSON).

pub use studio_core::error::Result;

mod html;
mod knowsphere;
mod markdown;

pub use html::render_html;
pub use knowsphere::{render_knowsphere, KnowsphereInput};
pub use markdown::{render_markdown, ReportInput};
