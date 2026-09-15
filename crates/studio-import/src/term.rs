//! Parse TraceView term_result.json (domain term extraction output).
//!
//! The term pipeline extracts domain terms (persons, organizations,
//! locations, products, ...) with the semantic block IDs they occur in.
//! These terms are the "noun signals" / 领域术语 shown in the analysis UI.

use serde::Deserialize;

use crate::Result;

/// A domain term extracted by the TraceView term pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    /// The term text as it appears in the document.
    pub text: String,
    /// Entity category (e.g. PERSON, ORGANIZATION, LOCATION, PRODUCT).
    pub category: String,
    /// Pipeline confidence score.
    pub score: f32,
    /// Integer semantic block IDs the term occurs in.
    pub block_ids: Vec<u64>,
}

/// Common HTML tag names. The TraceView term pipeline occasionally extracts
/// markup fragments (e.g. `sub` from `<sub>`) as terms; these are artifacts,
/// not domain terms, and are dropped.
const HTML_TAG_ARTIFACTS: &[&str] = &[
    "sub", "sup", "br", "div", "span", "p", "b", "i", "u", "em", "strong", "small", "big", "code",
    "pre", "table", "tr", "td", "th", "a", "img", "html", "head", "body", "title", "meta", "link",
    "script", "style",
];

fn is_html_tag_artifact(text: &str) -> bool {
    HTML_TAG_ARTIFACTS.contains(&text.trim().to_ascii_lowercase().as_str())
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawTermResult {
    #[serde(default)]
    terms: Vec<RawTerm>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawTerm {
    #[serde(default)]
    text: String,
    #[serde(default)]
    original_text: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    score: f32,
    #[serde(default)]
    block_ids: Vec<u64>,
}

/// Parse term_result.json and return the extracted terms.
///
/// The pipeline's `text` field is a normalized form that is sometimes garbled
/// (e.g. `法果` for `法国`); the `original_text` field carries the surface
/// form as printed in the document and is preferred for display. Terms that
/// are empty or are HTML tag artifacts (`sub`, `sup`, ...) are dropped.
pub fn parse_term_result(json: &str) -> Result<Vec<Term>> {
    let raw: RawTermResult = serde_json::from_str(json)?;
    Ok(raw
        .terms
        .into_iter()
        .map(|t| {
            let text = if t.original_text.trim().is_empty() {
                t.text
            } else {
                t.original_text
            };
            Term {
                text: text.trim().to_string(),
                category: t.category,
                score: t.score,
                block_ids: t.block_ids,
            }
        })
        .filter(|t| !t.text.is_empty() && !is_html_tag_artifact(&t.text))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_term_result() {
        let json = r#"{
            "version": "1.0",
            "terms": [
                {"text": "出版学", "category": "UNKNOWN", "score": 0.9, "block_ids": [1, 5]},
                {"text": "武汉大学出版社", "category": "ORGANIZATION", "score": 0.5, "block_ids": [3]},
                {"text": "  ", "category": "UNKNOWN", "score": 0.1, "block_ids": [2]}
            ]
        }"#;
        let terms = parse_term_result(json).unwrap();
        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0].text, "出版学");
        assert_eq!(terms[0].category, "UNKNOWN");
        assert_eq!(terms[0].block_ids, vec![1, 5]);
        assert_eq!(terms[1].text, "武汉大学出版社");
        assert_eq!(terms[1].category, "ORGANIZATION");
    }

    #[test]
    fn test_parse_term_result_prefers_original_text() {
        // The normalized `text` is garbled; `original_text` is the surface form.
        let json = r#"{
            "terms": [
                {"text": "法果", "original_text": "法国", "category": "LOCATION", "score": 0.2, "block_ids": [1]},
                {"text": "sub", "original_text": "sub", "category": "UNKNOWN", "score": 0.2, "block_ids": [2]},
                {"text": "sup", "original_text": "sup", "category": "UNKNOWN", "score": 0.2, "block_ids": [3]}
            ]
        }"#;
        let terms = parse_term_result(json).unwrap();
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].text, "法国");
        assert_eq!(terms[0].category, "LOCATION");
    }

    #[test]
    fn test_parse_term_result_empty() {
        let terms = parse_term_result(r#"{"terms": []}"#).unwrap();
        assert!(terms.is_empty());
    }

    #[test]
    fn test_parse_term_result_invalid_json() {
        assert!(parse_term_result("not json").is_err());
    }
}
