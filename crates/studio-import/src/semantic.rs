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
struct RawBlock {
    #[serde(default)]
    source_block_ids: Vec<String>,
    #[serde(default)]
    content: String,
    #[serde(default)]
    section_path: String,
    #[serde(default, rename = "type")]
    block_type: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    tokens: Vec<RawToken>,
    #[serde(default)]
    entities: Vec<RawEntity>,
    #[serde(default)]
    noun_signals: Vec<RawNounSignal>,
}

#[derive(Debug, Deserialize)]
struct RawToken {
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
struct RawEntity {
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
struct RawNounSignal {
    text: String,
    pos: String,
    #[serde(default)]
    syntactic_role: Option<String>,
    #[serde(default)]
    score: f32,
    #[serde(default)]
    span: Vec<usize>,
    #[serde(default)]
    evidence: Option<RawNounSignalEvidence>,
}

#[derive(Debug, Deserialize)]
struct RawNounSignalEvidence {
    #[serde(default)]
    head_rel: Option<String>,
    #[serde(default)]
    extra: Option<serde_json::Value>,
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
    let raw: RawSemanticResult = serde_json::from_str(json)
        .map_err(|e| studio_core::error::StudioError::Import(e.to_string()))?;

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
        assert_eq!(doc.blocks[0].source_block_id, "53f1a2b9-ac84-486d-9fe1-a4df1c832433");
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
}
