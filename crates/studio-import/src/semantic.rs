//! Parse TraceView semantic_result.json into internal models.

use serde::Deserialize;
use studio_core::document::{DocumentData, SemanticBlock, Token};
use studio_core::entity::{Entity, EntityCategory};
use studio_core::noun_signal::{NounSignal, NounSignalEvidence};

use crate::Result;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawSemanticResult {
    #[serde(default)]
    blocks: Vec<RawBlock>,
    #[serde(default)]
    document_aggregation: Option<RawAggregation>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RawAggregation {
    #[serde(default)]
    total_blocks: u32,
    #[serde(default)]
    total_entities: u32,
    #[serde(default)]
    total_noun_signals: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawBlock {
    #[serde(default)]
    pub(crate) source_block_ids: Vec<String>,
    #[serde(default)]
    pub(crate) block_ids: Vec<u64>,
    #[serde(default)]
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) section_path: String,
    #[serde(default, rename = "type")]
    pub(crate) block_type: Option<String>,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) tokens: Vec<RawToken>,
    #[serde(default)]
    pub(crate) entities: Vec<RawEntity>,
    #[serde(default)]
    pub(crate) noun_signals: Vec<RawNounSignal>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawToken {
    text: String,
    pos: String,
    #[serde(default)]
    confidence: f32,
    #[serde(default)]
    span: Vec<usize>,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawEntity {
    #[serde(default)]
    text: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    confidence: f32,
    #[serde(default)]
    span: Vec<usize>,
    #[serde(default)]
    keep: bool,
    #[serde(default)]
    filter: Option<String>,
    #[serde(default)]
    filter_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawNounSignal {
    #[serde(default)]
    pub(crate) text: String,
    #[serde(default)]
    pub(crate) pos: String,
    #[serde(default)]
    pub(crate) syntactic_role: Option<String>,
    #[serde(default)]
    pub(crate) score: f32,
    #[serde(default)]
    pub(crate) span: Vec<usize>,
    #[serde(default)]
    pub(crate) evidence: Option<RawNounSignalEvidence>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawNounSignalEvidence {
    #[serde(default)]
    pub(crate) head_rel: Option<String>,
    #[serde(default)]
    pub(crate) extra: Option<serde_json::Value>,
}

fn convert_token(raw: RawToken) -> Token {
    Token {
        text: raw.text,
        pos: raw.pos,
        confidence: raw.confidence,
        span: (
            raw.span.first().copied().unwrap_or(0),
            raw.span.get(1).copied().unwrap_or(0),
        ),
        source: raw.source,
    }
}

fn convert_entity(raw: RawEntity) -> Entity {
    Entity {
        text: raw.text,
        category: raw.category.parse().unwrap_or(EntityCategory::Unknown),
        confidence: raw.confidence,
        source: raw.source,
        keep: raw.keep,
        filter: raw.filter,
        filter_reason: raw.filter_reason,
        span: (
            raw.span.first().copied().unwrap_or(0),
            raw.span.get(1).copied().unwrap_or(0),
        ),
    }
}

fn convert_noun_signal(raw: RawNounSignal) -> NounSignal {
    NounSignal {
        text: raw.text,
        pos: raw.pos,
        syntactic_role: raw.syntactic_role,
        score: raw.score,
        span: (
            raw.span.first().copied().unwrap_or(0),
            raw.span.get(1).copied().unwrap_or(0),
        ),
        evidence: raw.evidence.map(|e| NounSignalEvidence {
            head_rel: e.head_rel,
            extra: e.extra.unwrap_or(serde_json::Value::Null),
        }),
    }
}

fn convert_block(raw: RawBlock) -> SemanticBlock {
    SemanticBlock {
        source_block_id: raw.source_block_ids.first().cloned().unwrap_or_default(),
        content: raw.content,
        section_path: raw.section_path,
        block_type: raw.block_type.unwrap_or_else(|| "paragraph".to_string()),
        title: raw.title,
        tokens: raw.tokens.into_iter().map(convert_token).collect(),
        entities: raw.entities.into_iter().map(convert_entity).collect(),
        noun_signals: raw
            .noun_signals
            .into_iter()
            .map(convert_noun_signal)
            .collect(),
    }
}

/// Parse a TraceView semantic_result.json file into DocumentData.
pub fn parse_semantic_result(json: &str) -> Result<DocumentData> {
    parse_semantic_result_enriched(json, None, None)
}

/// Parse a TraceView semantic_result.json file into DocumentData, optionally
/// enriching the blocks with:
///
/// * `popo_json` — popo_result.json; its TOC is matched against block titles
///   to fill in `section_path` (chapter/section hierarchy).
/// * `term_json` — term_result.json; its domain terms are attached to their
///   blocks as noun signals (领域术语).
///
/// Enrichment is best-effort: a missing or malformed auxiliary file logs a
/// warning but never fails the import.
pub fn parse_semantic_result_enriched(
    semantic_json: &str,
    popo_json: Option<&str>,
    term_json: Option<&str>,
) -> Result<DocumentData> {
    let mut raw: RawSemanticResult = serde_json::from_str(semantic_json)
        .map_err(|e| studio_core::error::StudioError::Import(e.to_string()))?;

    if let Some(popo) = popo_json {
        match crate::popo::parse_popo_toc(popo) {
            Ok(toc) if !toc.is_empty() => {
                let matched = crate::enrich::apply_toc(&mut raw.blocks, &toc);
                eprintln!(
                    "[import] TOC enrichment: {matched}/{} entries matched",
                    toc.len()
                );
            }
            Ok(_) => {}
            Err(e) => eprintln!("[import] popo TOC enrichment skipped: {e}"),
        }
    }

    if let Some(terms) = term_json {
        match crate::term::parse_term_result(terms) {
            Ok(parsed) if !parsed.is_empty() => {
                let added = crate::enrich::apply_terms(&mut raw.blocks, &parsed);
                eprintln!("[import] term enrichment: {added} noun signals added");
            }
            Ok(_) => {}
            Err(e) => eprintln!("[import] term enrichment skipped: {e}"),
        }
    }

    let blocks: Vec<SemanticBlock> = raw.blocks.into_iter().map(convert_block).collect();
    let total_word_count = blocks.iter().map(|b| b.tokens.len() as u64).sum();
    let total_char_count = blocks.iter().map(|b| b.content.len() as u64).sum();

    Ok(DocumentData {
        doc_id: uuid::Uuid::new_v4(),
        title: "Untitled".to_string(),
        blocks,
        total_word_count,
        total_char_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"{
        "version": "1.0",
        "blocks": [
            {
                "source_block_ids": ["53f1a2b9-ac84-486d-9fe1-a4df1c832433"],
                "content": "Hardwired: Marlowe 3.0 analysis",
                "section_path": "Hardwired: Marlowe 3.0 analysis",
                "type": "title",
                "title": null,
                "tokens": [
                    {"text": "Hardwired", "pos": "JJ", "confidence": 0.99, "span": [0, 10], "source": "en_modernbert"}
                ],
                "entities": [
                    {"text": "Marlowe", "category": "UNKNOWN", "source": "pos/nnp", "confidence": 0.55, "span": [12, 19], "keep": false, "filter": "F2_confidence", "filter_reason": "low confidence"}
                ],
                "noun_signals": [
                    {"text": "analysis", "pos": "NN", "syntactic_role": "unknown", "score": 0.5, "span": [56, 64], "evidence": {"head_rel": "dep"}}
                ]
            }
        ],
        "document_aggregation": {
            "total_blocks": 1,
            "total_entities": 1,
            "total_noun_signals": 1
        }
    }"#;

    #[test]
    fn test_parse_semantic_result() {
        let doc = parse_semantic_result(SAMPLE_JSON).unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.total_word_count, 1);
        assert_eq!(doc.total_char_count, 31);
        assert_eq!(doc.blocks[0].content, "Hardwired: Marlowe 3.0 analysis");
        assert_eq!(
            doc.blocks[0].source_block_id,
            "53f1a2b9-ac84-486d-9fe1-a4df1c832433"
        );
        assert_eq!(doc.blocks[0].block_type, "title");
        assert_eq!(doc.blocks[0].tokens.len(), 1);
        assert_eq!(doc.blocks[0].entities.len(), 1);
        assert_eq!(doc.blocks[0].noun_signals.len(), 1);
    }

    #[test]
    fn test_entity_conversion() {
        let doc = parse_semantic_result(SAMPLE_JSON).unwrap();
        let entity = &doc.blocks[0].entities[0];
        assert_eq!(entity.text, "Marlowe");
        assert_eq!(entity.category, EntityCategory::Unknown);
        assert!(!entity.keep);
        assert_eq!(entity.filter.as_deref(), Some("F2_confidence"));
    }

    #[test]
    fn test_invalid_json() {
        let result = parse_semantic_result("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_semantic_result_enriched() {
        let semantic = r#"{
            "blocks": [
                {"block_ids": [1], "content": "第一章 出版学的核心概念", "type": "title"},
                {"block_ids": [2], "content": "正文段落 出版学 是研究出版活动的", "type": "text"},
                {"block_ids": [3], "content": "第一节 出版物", "type": "title"},
                {"block_ids": [4], "content": "正文段落二", "type": "text"}
            ]
        }"#;
        let popo = r#"{
            "result": {
                "toc": {
                    "entries": [
                        {"title": "第一章 出版学的核心概念", "level": 1},
                        {"title": "第一节 出版物", "level": 2}
                    ]
                }
            }
        }"#;
        let terms = r#"{
            "terms": [
                {"text": "出版学", "category": "UNKNOWN", "score": 0.9, "block_ids": [2]}
            ]
        }"#;

        let doc = parse_semantic_result_enriched(semantic, Some(popo), Some(terms)).unwrap();
        assert_eq!(doc.blocks.len(), 4);
        assert_eq!(doc.blocks[0].section_path, "第一章 出版学的核心概念");
        assert_eq!(doc.blocks[1].section_path, "第一章 出版学的核心概念");
        assert_eq!(
            doc.blocks[2].section_path,
            "第一章 出版学的核心概念 / 第一节 出版物"
        );
        assert_eq!(
            doc.blocks[3].section_path,
            "第一章 出版学的核心概念 / 第一节 出版物"
        );
        assert_eq!(doc.blocks[1].noun_signals.len(), 1);
        assert_eq!(doc.blocks[1].noun_signals[0].text, "出版学");
        assert_eq!(doc.blocks[1].noun_signals[0].pos, "UNKNOWN");
    }

    #[test]
    fn test_parse_semantic_result_enriched_bad_aux_files() {
        // Malformed auxiliary files must not fail the import.
        let semantic = r#"{"blocks": [{"block_ids": [1], "content": "正文", "type": "text"}]}"#;
        let doc = parse_semantic_result_enriched(semantic, Some("not json"), Some("also not json"))
            .unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].section_path, "");
        assert!(doc.blocks[0].noun_signals.is_empty());
    }
}
