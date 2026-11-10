//! studio-import: Data import layer for NarrativeStudio.
//!
//! Responsible for parsing TraceView output files and converting them
//! into internal data models.

pub use studio_core::error::Result;

mod encoding;
mod enrich;
mod plaintext;
mod popo;
mod semantic;
mod term;

pub use encoding::{decode_text_file, format_of_file_name, Decoded};
pub use plaintext::{parse_markdown, parse_plain_text, parse_txt, TextFormat};
pub use popo::{parse_popo_toc, TocEntry};
pub use semantic::{parse_semantic_result, parse_semantic_result_enriched};
pub use term::{parse_term_result, Term};
