//! Parse TraceView popo_result.json (TOC extraction output).
//!
//! The popo pipeline extracts the document's table of contents. The entries
//! carry the chapter/section titles and their hierarchy level, which is used
//! to assign `section_path` to semantic blocks during import.

use serde::Deserialize;

use crate::Result;

/// A single table-of-contents entry from popo_result.json.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    /// Chapter/section title as printed in the document.
    pub title: String,
    /// Hierarchy level (1 = chapter, 2 = section, ...).
    pub level: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawPopoResult {
    #[serde(default)]
    result: Option<RawPopoResultInner>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawPopoResultInner {
    #[serde(default)]
    toc: Option<RawToc>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawToc {
    #[serde(default)]
    entries: Vec<RawTocEntry>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawTocEntry {
    #[serde(default)]
    title: String,
    #[serde(default)]
    level: u32,
}

/// Parse popo_result.json and return the TOC entries in document order.
///
/// Entries with empty titles are dropped; levels are clamped to >= 1.
pub fn parse_popo_toc(json: &str) -> Result<Vec<TocEntry>> {
    let raw: RawPopoResult = serde_json::from_str(json)?;
    let entries = raw
        .result
        .and_then(|r| r.toc)
        .map(|t| t.entries)
        .unwrap_or_default();
    Ok(entries
        .into_iter()
        .map(|e| TocEntry {
            title: e.title.trim().to_string(),
            level: e.level.max(1),
        })
        .filter(|e| !e.title.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_popo_toc() {
        let json = r#"{
            "task_id": "abc",
            "status": "completed",
            "result": {
                "doc_id": "d1",
                "status": "ok",
                "message": "done",
                "toc": {
                    "entries": [
                        {"title": "第一章 出版学的核心概念", "level": 1, "page": 13},
                        {"title": "第一节 出版物", "level": 2, "page": 13},
                        {"title": "", "level": 2},
                        {"title": "  ", "level": 0}
                    ]
                }
            },
            "error": null
        }"#;
        let toc = parse_popo_toc(json).unwrap();
        assert_eq!(toc.len(), 2);
        assert_eq!(toc[0].title, "第一章 出版学的核心概念");
        assert_eq!(toc[0].level, 1);
        assert_eq!(toc[1].title, "第一节 出版物");
        assert_eq!(toc[1].level, 2);
    }

    #[test]
    fn test_parse_popo_toc_missing_result() {
        let toc = parse_popo_toc(r#"{"status": "failed", "error": "boom"}"#).unwrap();
        assert!(toc.is_empty());
    }

    #[test]
    fn test_parse_popo_toc_invalid_json() {
        assert!(parse_popo_toc("not json").is_err());
    }
}
