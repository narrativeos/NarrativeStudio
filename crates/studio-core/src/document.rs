//! Document and semantic block models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::Entity;
use crate::noun_signal::NounSignal;

/// A token from the NLP pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub text: String,
    pub pos: String,
    pub confidence: f32,
    pub span: (usize, usize),
    pub source: String,
}

/// A semantic block — the basic unit of text analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticBlock {
    pub source_block_id: String,
    pub content: String,
    pub section_path: String,
    pub block_type: String,
    pub title: Option<String>,
    pub tokens: Vec<Token>,
    pub entities: Vec<Entity>,
    pub noun_signals: Vec<NounSignal>,
}

/// Complete document data loaded from TraceView output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentData {
    pub doc_id: Uuid,
    pub title: String,
    pub blocks: Vec<SemanticBlock>,
    pub total_word_count: u64,
    pub total_char_count: u64,
}

impl DocumentData {
    /// Calculate total word count from tokens.
    pub fn compute_word_count(&self) -> u64 {
        self.blocks.iter().map(|b| b.tokens.len() as u64).sum()
    }

    /// Calculate total character count from block content.
    pub fn compute_char_count(&self) -> u64 {
        self.blocks.iter().map(|b| b.content.len() as u64).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_block() -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test-1".to_string(),
            content: "Hardwired: Marlowe 3.0 analysis".to_string(),
            section_path: "Hardwired: Marlowe 3.0 analysis".to_string(),
            block_type: "title".to_string(),
            title: None,
            tokens: vec![
                Token {
                    text: "Hardwired".to_string(),
                    pos: "NNP".to_string(),
                    confidence: 0.99,
                    span: (0, 9),
                    source: "en_modernbert".to_string(),
                },
                Token {
                    text: "analysis".to_string(),
                    pos: "NN".to_string(),
                    confidence: 1.0,
                    span: (24, 32),
                    source: "en_modernbert".to_string(),
                },
            ],
            entities: vec![],
            noun_signals: vec![],
        }
    }

    #[test]
    fn test_document_word_count() {
        let doc = DocumentData {
            doc_id: Uuid::new_v4(),
            title: "Test".to_string(),
            blocks: vec![sample_block()],
            total_word_count: 0,
            total_char_count: 0,
        };
        assert_eq!(doc.compute_word_count(), 2);
    }

    #[test]
    fn test_document_char_count() {
        let doc = DocumentData {
            doc_id: Uuid::new_v4(),
            title: "Test".to_string(),
            blocks: vec![sample_block()],
            total_word_count: 0,
            total_char_count: 0,
        };
        assert_eq!(doc.compute_char_count(), 31);
    }
}
