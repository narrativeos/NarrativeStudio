//! T0: Pure statistical analysis — deterministic, < 100ms, no LLM.

use std::collections::HashMap;

use serde::Serialize;
use studio_core::document::DocumentData;
use studio_core::entity::EntityCategory;

use crate::Result;

/// Complete T0 statistics output.
#[derive(Debug, Clone, Serialize)]
pub struct T0Stats {
    pub word_count: u64,
    pub char_count: u64,
    pub block_count: u32,
    pub top_words: Vec<(String, u32)>,
    pub pos_distribution: Vec<(String, u32)>,
    pub entity_counts: Vec<(String, u32)>,
    pub top_entities: Vec<(String, u32)>,
    pub top_noun_signals: Vec<(String, u32)>,
    pub noun_signal_count: u32,
    pub avg_block_length: f64,
    pub avg_sentence_length: f64,
}

/// Run all T0 statistical analyses on the given document.
pub fn run_t0_analysis(doc: &DocumentData) -> Result<T0Stats> {
    let word_count = doc.total_word_count;
    let char_count = doc.total_char_count;
    let block_count = doc.blocks.len() as u32;

    let mut word_freq: HashMap<String, u32> = HashMap::new();
    for block in &doc.blocks {
        for token in &block.tokens {
            let lower = token.text.to_lowercase();
            *word_freq.entry(lower).or_insert(0) += 1;
        }
    }
    let mut top_words: Vec<(String, u32)> = word_freq.into_iter().collect();
    top_words.sort_by(|a, b| b.1.cmp(&a.1));
    top_words.truncate(50);

    let mut pos_dist: HashMap<String, u32> = HashMap::new();
    for block in &doc.blocks {
        for token in &block.tokens {
            *pos_dist.entry(token.pos.clone()).or_insert(0) += 1;
        }
    }
    let mut pos_distribution: Vec<(String, u32)> = pos_dist.into_iter().collect();
    pos_distribution.sort_by(|a, b| b.1.cmp(&a.1));

    let mut entity_counts_map: HashMap<EntityCategory, u32> = HashMap::new();
    for block in &doc.blocks {
        for entity in &block.entities {
            *entity_counts_map
                .entry(entity.category.clone())
                .or_insert(0) += 1;
        }
    }
    let mut entity_counts: Vec<(String, u32)> = entity_counts_map
        .into_iter()
        .map(|(cat, count)| (cat.as_str().to_string(), count))
        .collect();
    entity_counts.sort_by(|a, b| b.1.cmp(&a.1));

    // Top entity texts by frequency (key characters / places / objects).
    let mut entity_text_freq: HashMap<String, u32> = HashMap::new();
    for block in &doc.blocks {
        for entity in &block.entities {
            *entity_text_freq.entry(entity.text.clone()).or_insert(0) += 1;
        }
    }
    let mut top_entities: Vec<(String, u32)> = entity_text_freq.into_iter().collect();
    top_entities.sort_by(|a, b| b.1.cmp(&a.1));
    top_entities.truncate(50);

    // Top noun signals (domain terms) by frequency.
    let mut signal_freq: HashMap<String, u32> = HashMap::new();
    for block in &doc.blocks {
        for signal in &block.noun_signals {
            *signal_freq.entry(signal.text.clone()).or_insert(0) += 1;
        }
    }
    let mut top_noun_signals: Vec<(String, u32)> = signal_freq.into_iter().collect();
    top_noun_signals.sort_by(|a, b| b.1.cmp(&a.1));
    top_noun_signals.truncate(50);

    let noun_signal_count: u32 = doc.blocks.iter().map(|b| b.noun_signals.len() as u32).sum();

    let avg_block_length = if block_count > 0 {
        char_count as f64 / block_count as f64
    } else {
        0.0
    };
    let avg_sentence_length = if block_count > 0 {
        word_count as f64 / block_count as f64
    } else {
        0.0
    };

    Ok(T0Stats {
        word_count,
        char_count,
        block_count,
        top_words,
        pos_distribution,
        entity_counts,
        top_entities,
        top_noun_signals,
        noun_signal_count,
        avg_block_length,
        avg_sentence_length,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::{SemanticBlock, Token};
    use studio_core::entity::Entity;
    use studio_core::noun_signal::NounSignal;

    fn sample_doc() -> DocumentData {
        let block1 = SemanticBlock {
            source_block_id: "blk-1".into(),
            content: "The quick brown fox jumps over the lazy dog".to_string(),
            section_path: "Test".to_string(),
            block_type: "paragraph".to_string(),
            title: None,
            tokens: vec![
                Token {
                    text: "The".into(),
                    pos: "DT".into(),
                    confidence: 1.0,
                    span: (0, 3),
                    source: "test".into(),
                },
                Token {
                    text: "quick".into(),
                    pos: "JJ".into(),
                    confidence: 1.0,
                    span: (4, 9),
                    source: "test".into(),
                },
                Token {
                    text: "brown".into(),
                    pos: "JJ".into(),
                    confidence: 1.0,
                    span: (10, 15),
                    source: "test".into(),
                },
                Token {
                    text: "fox".into(),
                    pos: "NN".into(),
                    confidence: 1.0,
                    span: (16, 19),
                    source: "test".into(),
                },
                Token {
                    text: "jumps".into(),
                    pos: "VBZ".into(),
                    confidence: 1.0,
                    span: (20, 25),
                    source: "test".into(),
                },
                Token {
                    text: "over".into(),
                    pos: "IN".into(),
                    confidence: 1.0,
                    span: (26, 30),
                    source: "test".into(),
                },
                Token {
                    text: "the".into(),
                    pos: "DT".into(),
                    confidence: 1.0,
                    span: (31, 34),
                    source: "test".into(),
                },
                Token {
                    text: "lazy".into(),
                    pos: "JJ".into(),
                    confidence: 1.0,
                    span: (35, 39),
                    source: "test".into(),
                },
                Token {
                    text: "dog".into(),
                    pos: "NN".into(),
                    confidence: 1.0,
                    span: (40, 43),
                    source: "test".into(),
                },
            ],
            entities: vec![Entity {
                text: "fox".into(),
                category: EntityCategory::Unknown,
                confidence: 0.8,
                source: "test".into(),
                keep: true,
                filter: None,
                filter_reason: None,
                span: (16, 19),
            }],
            noun_signals: vec![NounSignal {
                text: "fox".into(),
                pos: "NN".into(),
                syntactic_role: Some("Subject".into()),
                score: 0.7,
                span: (16, 19),
                evidence: None,
            }],
        };

        let block2 = SemanticBlock {
            source_block_id: "blk-2".into(),
            content: "The dog barked".to_string(),
            section_path: "Test".to_string(),
            block_type: "paragraph".to_string(),
            title: None,
            tokens: vec![
                Token {
                    text: "The".into(),
                    pos: "DT".into(),
                    confidence: 1.0,
                    span: (0, 3),
                    source: "test".into(),
                },
                Token {
                    text: "dog".into(),
                    pos: "NN".into(),
                    confidence: 1.0,
                    span: (4, 7),
                    source: "test".into(),
                },
                Token {
                    text: "barked".into(),
                    pos: "VBD".into(),
                    confidence: 1.0,
                    span: (8, 14),
                    source: "test".into(),
                },
            ],
            entities: vec![],
            noun_signals: vec![],
        };

        DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "Test Doc".into(),
            blocks: vec![block1, block2],
            total_word_count: 12,
            total_char_count: 59,
        }
    }

    #[test]
    fn test_t0_word_count() {
        let doc = sample_doc();
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.word_count, 12);
        assert_eq!(stats.block_count, 2);
    }

    #[test]
    fn test_t0_top_words() {
        let doc = sample_doc();
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.top_words[0].0, "the");
        assert_eq!(stats.top_words[0].1, 3);
    }

    #[test]
    fn test_t0_pos_distribution() {
        let doc = sample_doc();
        let stats = run_t0_analysis(&doc).unwrap();
        let dt_count = stats
            .pos_distribution
            .iter()
            .find(|(p, _)| p == "DT")
            .unwrap()
            .1;
        assert_eq!(dt_count, 3);
    }

    #[test]
    fn test_t0_entity_counts() {
        let doc = sample_doc();
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.entity_counts.len(), 1);
        assert_eq!(stats.entity_counts[0].0, "UNKNOWN");
        assert_eq!(stats.entity_counts[0].1, 1);
    }

    #[test]
    fn test_t0_noun_signals() {
        let doc = sample_doc();
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.noun_signal_count, 1);
    }

    #[test]
    fn test_t0_top_entities() {
        let doc = sample_doc();
        let stats = run_t0_analysis(&doc).unwrap();
        // block1 contributes exactly one entity.
        assert_eq!(stats.top_entities.len(), 1);
        assert_eq!(stats.top_entities[0].1, 1);
    }

    #[test]
    fn test_t0_top_noun_signals() {
        let doc = sample_doc();
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.top_noun_signals[0].0, "fox");
        assert_eq!(stats.top_noun_signals[0].1, 1);
    }

    #[test]
    fn test_t0_empty_doc() {
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "Empty".into(),
            blocks: vec![],
            total_word_count: 0,
            total_char_count: 0,
        };
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.word_count, 0);
        assert_eq!(stats.block_count, 0);
        assert_eq!(stats.avg_block_length, 0.0);
    }
}
