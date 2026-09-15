//! T0: Pure statistical analysis — deterministic, < 100ms, no LLM.

use std::collections::HashMap;

use serde::Serialize;
use studio_core::document::{DocumentData, Token};
use studio_core::entity::EntityCategory;

use crate::text::{is_cjk, split_sentences, visible_char_count};
use crate::Result;

/// A frequently repeated phrase (consecutive token n-gram).
#[derive(Debug, Clone, Serialize)]
pub struct RepeatedPhrase {
    pub text: String,
    pub count: u32,
}

/// Readability assessment for a document.
#[derive(Debug, Clone, Serialize)]
pub struct Readability {
    /// 0-100, higher = easier to read.
    pub score: f64,
    /// Human-readable level label (易读 / 适中 / 偏难 / 困难).
    pub level: String,
    /// Which algorithm produced the score ("flesch" or "cjk-sentence-length").
    pub method: String,
}

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
    /// Number of sentences detected across all blocks.
    pub sentence_count: u64,
    /// Average visible characters per sentence (0 when no sentences).
    pub avg_sentence_chars: f64,
    /// Sentence length histogram: ("≤20", n), ("21-50", n), ("51-100", n), (">100", n).
    pub sentence_length_distribution: Vec<(String, u32)>,
    /// Total adverb tokens (POS AD/DEG/AS).
    pub adverb_count: u32,
    /// Total adjective tokens (POS JJ/VA).
    pub adjective_count: u32,
    /// Top adverbs by frequency.
    pub top_adverbs: Vec<(String, u32)>,
    /// Top adjectives by frequency.
    pub top_adjectives: Vec<(String, u32)>,
    /// Most repeated consecutive token phrases (n=2,3), count >= 5.
    pub top_repeated_phrases: Vec<RepeatedPhrase>,
    /// Readability score, level, and method.
    pub readability: Readability,
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

    // --- Sentence statistics ---
    let mut sentence_lengths: Vec<usize> = Vec::new();
    for block in &doc.blocks {
        for sentence in split_sentences(&block.content) {
            let len = visible_char_count(&sentence);
            if len > 0 {
                sentence_lengths.push(len);
            }
        }
    }
    let sentence_count = sentence_lengths.len() as u64;
    let avg_sentence_chars = if sentence_count > 0 {
        sentence_lengths.iter().map(|l| *l as f64).sum::<f64>() / sentence_count as f64
    } else {
        0.0
    };
    let mut buckets = [0u32; 4];
    for len in &sentence_lengths {
        buckets[sentence_bucket(*len)] += 1;
    }
    let sentence_length_distribution = vec![
        ("≤20".to_string(), buckets[0]),
        ("21-50".to_string(), buckets[1]),
        ("51-100".to_string(), buckets[2]),
        (">100".to_string(), buckets[3]),
    ];

    // --- Adverb / adjective statistics ---
    const ADVERB_POS: &[&str] = &["AD", "DEG", "AS"];
    const ADJECTIVE_POS: &[&str] = &["JJ", "VA"];
    let mut adverb_freq: HashMap<String, u32> = HashMap::new();
    let mut adjective_freq: HashMap<String, u32> = HashMap::new();
    let mut adverb_count = 0u32;
    let mut adjective_count = 0u32;
    for block in &doc.blocks {
        for token in &block.tokens {
            if ADVERB_POS.contains(&token.pos.as_str()) {
                // Skip function words (的/了/不/…) that NLP pipelines tag as
                // adverbs but which carry no stylistic signal.
                if ADVERB_STOP_WORDS.contains(&token.text.as_str()) {
                    continue;
                }
                adverb_count += 1;
                *adverb_freq.entry(token.text.clone()).or_insert(0) += 1;
            } else if ADJECTIVE_POS.contains(&token.pos.as_str()) {
                adjective_count += 1;
                *adjective_freq.entry(token.text.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut top_adverbs: Vec<(String, u32)> = adverb_freq.into_iter().collect();
    top_adverbs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top_adverbs.truncate(50);
    let mut top_adjectives: Vec<(String, u32)> = adjective_freq.into_iter().collect();
    top_adjectives.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top_adjectives.truncate(50);

    // --- Repeated phrases & readability ---
    let top_repeated_phrases = repeated_phrases(doc);
    let readability = compute_readability(doc, &sentence_lengths);

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
        sentence_count,
        avg_sentence_chars,
        sentence_length_distribution,
        adverb_count,
        adjective_count,
        top_adverbs,
        top_adjectives,
        top_repeated_phrases,
        readability,
    })
}

/// Histogram bucket index for a sentence length in visible characters.
fn sentence_bucket(len: usize) -> usize {
    match len {
        0..=20 => 0,
        21..=50 => 1,
        51..=100 => 2,
        _ => 3,
    }
}

/// Function words that NLP pipelines often tag as adverbs (particles,
/// conjunctions, prepositions, negation) but which carry no stylistic
/// signal — excluded from adverb statistics.
const ADVERB_STOP_WORDS: &[&str] = &[
    "的", "了", "着", "地", "得", "之", "其", "此", "该", "这", "那", "是", "在", "有", "和", "与",
    "或", "及", "而", "但", "并", "就", "都", "也", "不", "没", "未", "无", "非", "别", "莫", "又",
    "再", "才", "便", "即", "则", "若", "如", "为", "以", "于", "对", "把", "被", "让", "向", "从",
    "自", "由", "往", "朝", "给", "因", "所以", "因为", "如果", "虽然", "但是", "然而", "于是",
    "因此", "而且", "或者", "以及", "并且", "还是", "从而", "进而",
];

/// POS tags that never contribute to a meaningful phrase.
const PHRASE_STOP_POS: &[&str] = &[
    "PU", "P", "CC", "DT", "PN", "M", "SP", "BA", "DEC", "CD", "OD", "CS", "MSP", "ETC", "NOI",
    "ADD", "DER", "FW", "URL", "SB", "DEV", "AS", "DEG", "-LRB-", "-RRB-", ",",
];

/// Whether a token can take part in a repeated phrase.
fn phrase_eligible(token: &Token) -> bool {
    if PHRASE_STOP_POS.contains(&token.pos.as_str()) {
        return false;
    }
    token.text.chars().any(|c| c.is_alphanumeric())
}

/// Detect repeated phrases via consecutive-token n-grams (n = 2, 3).
///
/// Tokens must be phrase-eligible and strictly adjacent in the source text
/// (a punctuation or function word between them breaks the phrase). Only
/// phrases occurring at least 5 times are reported, top 30 by frequency.
fn repeated_phrases(doc: &DocumentData) -> Vec<RepeatedPhrase> {
    let mut freq: HashMap<String, u32> = HashMap::new();
    for block in &doc.blocks {
        let mut tokens: Vec<&Token> = block.tokens.iter().collect();
        tokens.sort_by_key(|t| t.span.0);
        let eligible: Vec<&Token> = tokens
            .iter()
            .filter(|t| phrase_eligible(t))
            .copied()
            .collect();

        let mut run: Vec<&Token> = Vec::new();
        for t in &eligible {
            if let Some(prev) = run.last() {
                if t.span.0 == prev.span.1 {
                    run.push(t);
                    continue;
                }
            }
            count_ngrams(&run, &mut freq);
            run = vec![t];
        }
        count_ngrams(&run, &mut freq);
    }

    let mut phrases: Vec<RepeatedPhrase> = freq
        .into_iter()
        .filter(|(_, count)| *count >= 5)
        .map(|(text, count)| RepeatedPhrase { text, count })
        .collect();
    phrases.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.text.cmp(&b.text)));
    phrases.truncate(30);
    phrases
}

/// Count 2- and 3-grams of a consecutive token run into `freq`.
fn count_ngrams(run: &[&Token], freq: &mut HashMap<String, u32>) {
    for n in [2usize, 3] {
        if run.len() < n {
            break;
        }
        for window in run.windows(n) {
            let text: String = window.iter().map(|t| t.text.as_str()).collect();
            *freq.entry(text).or_insert(0) += 1;
        }
    }
}

/// Compute a 0-100 readability score (higher = easier to read).
///
/// CJK documents use a sentence-length heuristic (Chinese prose reads best
/// around 20-25 characters per sentence); Latin documents use the classic
/// Flesch Reading Ease formula.
fn compute_readability(doc: &DocumentData, sentence_lengths: &[usize]) -> Readability {
    if sentence_lengths.is_empty() {
        return Readability {
            score: 0.0,
            level: "无文本".into(),
            method: "none".into(),
        };
    }

    let mut cjk_chars = 0usize;
    let mut alpha_chars = 0usize;
    for block in &doc.blocks {
        for c in block.content.chars() {
            if is_cjk(c) {
                cjk_chars += 1;
            } else if c.is_ascii_alphabetic() {
                alpha_chars += 1;
            }
        }
    }

    if cjk_chars >= alpha_chars {
        let avg =
            sentence_lengths.iter().map(|l| *l as f64).sum::<f64>() / sentence_lengths.len() as f64;
        let score = (100.0 - (avg - 25.0).max(0.0) * 2.0).clamp(0.0, 100.0);
        Readability {
            score,
            level: readability_level(score),
            method: "cjk-sentence-length".into(),
        }
    } else {
        let words: Vec<String> = doc
            .blocks
            .iter()
            .flat_map(|b| b.tokens.iter())
            .map(|t| t.text.to_lowercase())
            .filter(|w| w.chars().any(|c| c.is_ascii_alphabetic()))
            .collect();
        let sentences = sentence_lengths.len().max(1) as f64;
        let word_count = words.len().max(1) as f64;
        let syllables: f64 = words.iter().map(|w| syllable_count(w)).sum::<usize>() as f64;
        let score = (206.835 - 1.015 * (word_count / sentences) - 84.6 * (syllables / word_count))
            .clamp(0.0, 100.0);
        Readability {
            score,
            level: readability_level(score),
            method: "flesch".into(),
        }
    }
}

/// Rough English syllable count: one per vowel group, minimum one.
fn syllable_count(word: &str) -> usize {
    let mut count = 0;
    let mut in_vowel = false;
    for c in word.chars() {
        let is_vowel = matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
        if is_vowel && !in_vowel {
            count += 1;
            in_vowel = true;
        } else if !is_vowel {
            in_vowel = false;
        }
    }
    count.max(1)
}

fn readability_level(score: f64) -> String {
    match score {
        s if s >= 80.0 => "易读".into(),
        s if s >= 60.0 => "适中".into(),
        s if s >= 40.0 => "偏难".into(),
        _ => "困难".into(),
    }
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
        assert_eq!(stats.sentence_count, 0);
        assert_eq!(stats.readability.method, "none");
    }

    #[test]
    fn test_t0_sentence_stats() {
        let block = SemanticBlock {
            source_block_id: "blk-s".into(),
            content: "短句。这是一个稍微长一点点的句子，用来测试分句功能是否正常。".into(),
            section_path: "T".into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
        };
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "S".into(),
            blocks: vec![block],
            total_word_count: 0,
            total_char_count: 0,
        };
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.sentence_count, 2);
        // "短句" (2 chars) falls in ≤20; the other sentence (24 chars) in 21-50.
        let dist = &stats.sentence_length_distribution;
        assert_eq!(dist[0].1, 1);
        assert_eq!(dist[1].1, 1);
        assert!(stats.avg_sentence_chars > 0.0);
    }

    #[test]
    fn test_t0_adverbs_adjectives() {
        let block = SemanticBlock {
            source_block_id: "blk-adv".into(),
            content: "他非常认真地工作，非常仔细。".into(),
            section_path: "T".into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![
                Token {
                    text: "非常".into(),
                    pos: "DEG".into(),
                    confidence: 1.0,
                    span: (1, 3),
                    source: "t".into(),
                },
                Token {
                    text: "认真".into(),
                    pos: "AD".into(),
                    confidence: 1.0,
                    span: (3, 5),
                    source: "t".into(),
                },
                Token {
                    text: "非常".into(),
                    pos: "DEG".into(),
                    confidence: 1.0,
                    span: (8, 10),
                    source: "t".into(),
                },
                Token {
                    text: "仔细".into(),
                    pos: "JJ".into(),
                    confidence: 1.0,
                    span: (10, 12),
                    source: "t".into(),
                },
            ],
            entities: vec![],
            noun_signals: vec![],
        };
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "A".into(),
            blocks: vec![block],
            total_word_count: 4,
            total_char_count: 14,
        };
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.adverb_count, 3);
        assert_eq!(stats.adjective_count, 1);
        assert_eq!(stats.top_adverbs[0], ("非常".to_string(), 2));
        assert_eq!(stats.top_adjectives[0], ("仔细".to_string(), 1));
    }

    #[test]
    fn test_t0_repeated_phrases() {
        // "出版单位" (two adjacent tokens) repeated 5 times.
        let mut blocks = Vec::new();
        for i in 0..5 {
            let tokens = vec![
                Token {
                    text: "出版".into(),
                    pos: "NN".into(),
                    confidence: 1.0,
                    span: (0, 2),
                    source: "t".into(),
                },
                Token {
                    text: "单位".into(),
                    pos: "NN".into(),
                    confidence: 1.0,
                    span: (2, 4),
                    source: "t".into(),
                },
            ];
            blocks.push(SemanticBlock {
                source_block_id: format!("blk-{i}"),
                content: "出版单位".into(),
                section_path: "T".into(),
                block_type: "paragraph".into(),
                title: None,
                tokens,
                entities: vec![],
                noun_signals: vec![],
            });
        }
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "R".into(),
            blocks,
            total_word_count: 10,
            total_char_count: 20,
        };
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.top_repeated_phrases.len(), 1);
        assert_eq!(stats.top_repeated_phrases[0].text, "出版单位");
        assert_eq!(stats.top_repeated_phrases[0].count, 5);
    }

    #[test]
    fn test_t0_repeated_phrases_broken_by_punctuation() {
        // Punctuation token between the two words breaks the phrase.
        let tokens = vec![
            Token {
                text: "出版".into(),
                pos: "NN".into(),
                confidence: 1.0,
                span: (0, 2),
                source: "t".into(),
            },
            Token {
                text: "，".into(),
                pos: "PU".into(),
                confidence: 1.0,
                span: (2, 3),
                source: "t".into(),
            },
            Token {
                text: "单位".into(),
                pos: "NN".into(),
                confidence: 1.0,
                span: (3, 5),
                source: "t".into(),
            },
        ];
        let block = SemanticBlock {
            source_block_id: "blk-p".into(),
            content: "出版，单位".into(),
            section_path: "T".into(),
            block_type: "paragraph".into(),
            title: None,
            tokens,
            entities: vec![],
            noun_signals: vec![],
        };
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "P".into(),
            blocks: vec![block],
            total_word_count: 3,
            total_char_count: 5,
        };
        let stats = run_t0_analysis(&doc).unwrap();
        assert!(stats.top_repeated_phrases.is_empty());
    }

    #[test]
    fn test_t0_readability_cjk() {
        let block = SemanticBlock {
            source_block_id: "blk-r".into(),
            content: "出版学是研究出版活动的学科。".into(),
            section_path: "T".into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
        };
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "R".into(),
            blocks: vec![block],
            total_word_count: 0,
            total_char_count: 14,
        };
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.readability.method, "cjk-sentence-length");
        // 13-char sentence < 25 => full score.
        assert_eq!(stats.readability.score, 100.0);
        assert_eq!(stats.readability.level, "易读");
    }

    #[test]
    fn test_t0_readability_flesch() {
        let block = SemanticBlock {
            source_block_id: "blk-f".into(),
            content: "The cat sat on the mat.".into(),
            section_path: "T".into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![
                Token {
                    text: "The".into(),
                    pos: "DT".into(),
                    confidence: 1.0,
                    span: (0, 3),
                    source: "t".into(),
                },
                Token {
                    text: "cat".into(),
                    pos: "NN".into(),
                    confidence: 1.0,
                    span: (4, 7),
                    source: "t".into(),
                },
            ],
            entities: vec![],
            noun_signals: vec![],
        };
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "F".into(),
            blocks: vec![block],
            total_word_count: 2,
            total_char_count: 23,
        };
        let stats = run_t0_analysis(&doc).unwrap();
        assert_eq!(stats.readability.method, "flesch");
        // Short words, one sentence => high Flesch score.
        assert!(stats.readability.score > 80.0);
    }
}
