//! What the analysed document actually carries.
//!
//! The analysis engine reads four kinds of input: the block text, tokens with
//! part-of-speech tags, entities, and noun signals. A TraceView import carries
//! all four; a Markdown or TXT import carries only the first. That difference
//! decides which questions can be answered at all, so it is stated in one place
//! rather than rediscovered — and possibly misreported — by each dimension.
//!
//! It is derived from the document, not recorded on the project: what the rows
//! contain describes the analysis better than a flag set at import time would,
//! and it cannot go stale.

use studio_core::document::DocumentData;

/// Which of the optional inputs the document carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputCoverage {
    /// Whether any block carries segmented tokens.
    pub has_tokens: bool,
    /// Whether any block carries a recognised part-of-speech tag.
    pub has_pos: bool,
    /// Whether any block carries an entity.
    pub has_entities: bool,
    /// Whether any block carries a noun signal.
    pub has_noun_signals: bool,
}

impl InputCoverage {
    /// True when the document carries everything the engine can use, i.e. it
    /// came from a full NLP import.
    pub fn is_full(&self) -> bool {
        self.has_tokens && self.has_entities
    }

    /// The dimensions that cannot answer, named the way the report names them.
    ///
    /// Order follows the report, so the sentence reads top to bottom.
    pub fn unreadable(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.has_tokens {
            missing.push("词频与重复短语");
        }
        if !self.has_pos {
            missing.push("副词与形容词");
        }
        if !self.has_entities && !self.has_noun_signals {
            missing.push("情节线");
        }
        if !self.has_entities {
            missing.push("地点与场景的实体部分");
        }
        if !self.has_noun_signals && !self.has_pos {
            missing.push("主题词表");
        }
        missing
    }

    /// One sentence for the report and the UI, saying what cannot be read and
    /// why. Empty when nothing is missing, so callers can print it unconditionally.
    ///
    /// The point of the sentence is the distinction between *cannot be read* and
    /// *not present*: an absent number is not evidence of an absent quality.
    pub fn note(&self) -> String {
        let missing = self.unreadable();
        if missing.is_empty() {
            return String::new();
        }
        format!(
            "本项目是纯文本导入，没有分词、词性与实体，因此{}读不到。读不到不等于没有：这些数字没有被测量，而不是测量结果为零。",
            missing.join("、")
        )
    }
}

/// Derive the coverage of a document from its blocks.
pub fn input_coverage(doc: &DocumentData) -> InputCoverage {
    let mut coverage = InputCoverage {
        has_tokens: false,
        has_pos: false,
        has_entities: false,
        has_noun_signals: false,
    };
    for block in &doc.blocks {
        if !block.tokens.is_empty() {
            coverage.has_tokens = true;
            if block.tokens.iter().any(|t| !t.pos.is_empty()) {
                coverage.has_pos = true;
            }
        }
        if !block.entities.is_empty() {
            coverage.has_entities = true;
        }
        if !block.noun_signals.is_empty() {
            coverage.has_noun_signals = true;
        }
        if coverage.has_tokens
            && coverage.has_pos
            && coverage.has_entities
            && coverage.has_noun_signals
        {
            break;
        }
    }
    coverage
}
