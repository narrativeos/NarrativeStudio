//! Theme analysis (主题分析): which terms the document keeps coming back to.
//!
//! The design doc describes this as "noun signals and full text → theme list and
//! intensity graph, noun-signal clustering plus LLM". The LLM half is out of scope,
//! so this module does the part that is a count and a distribution — and is
//! explicit about the difference between a theme and a term.
//!
//! A **theme** here is a recurring content noun, or a small family of nouns that
//! travel together: it has enough mentions to be more than a passing word, it shows
//! up in more than one segment, and it is either a superstring of its relatives
//! (出版 / 出版业) or co-occurs with them often enough to read as one subject. What
//! the module can measure is recurrence and spread; whether the recurrence *means*
//! something — a motif rather than a genre requirement — is a judgement this engine
//! does not make and the report does not claim.
//!
//! Two sources, chosen in order and reported: `noun_signals` when the import
//! carries them (the NLP pipeline's own content-noun picks), otherwise noun
//! `tokens` — a TraceView export without noun signals still carries hundreds of
//! tagged nouns, and a theme list that says "no data" over those is useless. Names
//! and time expressions are excluded either way; they belong to the character and
//! timeline dimensions.
//!
//! Deterministic: segments in reading order, clusters labelled by their busiest
//! member, ties broken by term, no clock and no randomness.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;

use crate::segment::plan_segments;
use crate::Result;

/// A term needs this many mentions to be a theme rather than a passing word (the
/// same floor the plot threads use, so the two dimensions agree about what counts).
const MIN_MENTIONS: u32 = 3;
/// How many themes are reported, biggest first.
const MAX_THEMES: usize = 8;
/// Members listed inside one theme's cluster.
const MAX_MEMBERS: usize = 8;
/// Same-block co-occurrences needed before two terms are treated as one subject.
const COOC_MIN: u32 = 3;
/// A theme covering this much of the document runs through it.
const SUSTAINED_COVERAGE: f64 = 0.5;
/// …and this much is enough to call it recurring rather than occasional.
const RECURRING_COVERAGE: f64 = 0.2;
/// Above this share of every candidate mention in one theme, the document is
/// repeating itself instead of building a set of subjects.
const OVER_CONCENTRATED: f64 = 0.35;
/// Below this share held by the top three, there is no subject to speak of.
const SCATTERED_TOP3: f64 = 0.12;
/// Above this share of mentions contributed by terms seen exactly once, the nouns
/// never come back: the vocabulary is varied, the subject is missing.
const SINGLETON_HEAVY: f64 = 0.5;
/// One third of the mentions inside a single third of the document is a
/// concentration rather than a spread.
const THIRD_HEAVY: f64 = 0.6;

/// Noun POS tags accepted by the token fallback. Names (`NR`, `NNP`) and time
/// expressions (`NT`) are left out on purpose: they are characters and dates, and
/// both already have their own dimensions.
const NOUN_POS: &[&str] = &["NN", "NNS", "n", "nz", "noun"];

/// Terms too generic to carry a subject. Deliberately short — the length filter
/// already removes single characters, and these are the words the NLP still tags as
/// nouns often enough to top a frequency list.
const STOP_TERMS: &[&str] = &[
    "什么",
    "这个",
    "那个",
    "一个",
    "时候",
    "东西",
    "事情",
    "问题",
    "自己",
    "这里",
    "那里",
    "一样",
    "一切",
    "所有",
    "没有",
    "可能",
    "方式",
    "部分",
    "方面",
    "作用",
    "情况",
    "一样东西",
];

/// A theme: one recurring subject and how far it reaches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// The cluster's busiest term; the cluster is named after it.
    pub term: String,
    /// Terms folded into this theme, busiest first (`term, mentions`).
    pub members: Vec<(String, u32)>,
    pub mentions: u32,
    pub sections_touched: u32,
    /// Fraction of the document's segments this theme appears in (0-1).
    pub coverage: f64,
    /// Share of every collected mention in the document.
    pub share: f64,
    pub first_section: String,
    pub last_section: String,
    /// `sustained` | `recurring` | `episodic` | `occasional`.
    pub reach: String,
    /// Longest run of segments where the theme is absent.
    pub longest_gap: u32,
    /// Where its mentions sit: `spread` | `front` | `middle` | `back`.
    pub concentration: String,
    /// Mentions per segment, aligned with the other curves' X axis.
    pub curve: Vec<u32>,
}

/// Theme analysis output for one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeStats {
    /// Reported themes, biggest first.
    pub themes: Vec<Theme>,
    /// Themes that cleared [`MIN_MENTIONS`], including unreported ones.
    pub candidate_count: u32,
    /// Mentions of every collected term, reported or not.
    pub total_mentions: u32,
    /// `noun_signals` | `noun_tokens` | `none`: which vocabulary was used. `none`
    /// means nothing usable was found — either the import carries neither, or every
    /// term in it was too short, numeric or generic to carry a subject.
    pub source: String,
    /// Share of all mentions held by the single biggest theme.
    pub top_share: f64,
    /// Share held by the top three together.
    pub top3_share: f64,
    /// Share of all mentions contributed by terms seen exactly once.
    pub singleton_share: f64,
    /// The three biggest theme names.
    pub dominant_terms: Vec<String>,
    pub section_count: u32,
    /// 0-100, or `None` when no usable noun vocabulary was found — see
    /// [`theme_score`] for what it charges.
    pub theme_score: Option<f64>,
}

/// One collected term and where it appears.
#[derive(Debug, Default)]
struct Candidate {
    mentions: u32,
    by_section: HashMap<usize, u32>,
    first: usize,
    last: usize,
}

/// Extract the recurring subjects of a document.
pub fn run_theme_analysis(doc: &DocumentData) -> Result<ThemeStats> {
    let plan = plan_segments(doc);
    let section_count = plan.labels.len();

    let (source, occurrences) = collect_terms(doc, &plan.segment_of);
    if occurrences.is_empty() {
        return Ok(ThemeStats {
            themes: Vec::new(),
            candidate_count: 0,
            total_mentions: 0,
            source: "none".into(),
            top_share: 0.0,
            top3_share: 0.0,
            singleton_share: 0.0,
            dominant_terms: Vec::new(),
            section_count: section_count as u32,
            theme_score: None,
        });
    }

    let total_mentions: u32 = occurrences.values().map(|c| c.mentions).sum();
    let singleton_mentions: u32 = occurrences
        .iter()
        .filter(|(_, c)| c.mentions == 1)
        .map(|(_, c)| c.mentions)
        .sum();

    let clusters = cluster_terms(&occurrences, doc, source == "noun_signals");
    let mut themes: Vec<Theme> = clusters
        .into_iter()
        .filter_map(|members| {
            build_theme(members, &occurrences, &plan, section_count, total_mentions)
        })
        .collect();
    themes.sort_by(|a, b| {
        b.mentions
            .cmp(&a.mentions)
            .then(b.coverage.total_cmp(&a.coverage))
            .then_with(|| a.term.cmp(&b.term))
    });

    let mut stats = ThemeStats {
        candidate_count: themes.len() as u32,
        total_mentions,
        source,
        top_share: themes.first().map(|t| t.share).unwrap_or(0.0),
        top3_share: themes.iter().take(3).map(|t| t.share).sum(),
        singleton_share: if total_mentions == 0 {
            0.0
        } else {
            singleton_mentions as f64 / total_mentions as f64
        },
        dominant_terms: themes.iter().take(3).map(|t| t.term.clone()).collect(),
        section_count: section_count as u32,
        theme_score: None,
        themes,
    };
    // The score and the shares are computed over every candidate; only the listing
    // is capped, so a document with forty themes is not judged as if it had eight.
    stats.theme_score = theme_score(&stats);
    stats.themes.truncate(MAX_THEMES);
    Ok(stats)
}

/// Collect every usable noun term with its mention sites, and report which
/// vocabulary the document offered.
fn collect_terms(doc: &DocumentData, segment_of: &[usize]) -> (String, HashMap<String, Candidate>) {
    let use_signals = doc.blocks.iter().any(|b| !b.noun_signals.is_empty());
    let source = if use_signals {
        "noun_signals"
    } else {
        "noun_tokens"
    };

    let mut occurrences: HashMap<String, Candidate> = HashMap::new();
    for (idx, block) in doc.blocks.iter().enumerate() {
        let seg = segment_of.get(idx).copied().unwrap_or(0);
        for term in block_terms(block, use_signals) {
            let entry = occurrences.entry(term).or_insert_with(|| Candidate {
                first: seg,
                last: seg,
                ..Default::default()
            });
            entry.mentions += 1;
            *entry.by_section.entry(seg).or_insert(0) += 1;
            entry.first = entry.first.min(seg);
            entry.last = entry.last.max(seg);
        }
    }

    (source.to_string(), occurrences)
}

/// The normalized terms one block offers, from the chosen source, deduplicated so a
/// term repeated inside a block counts once towards co-occurrence.
fn block_terms(block: &studio_core::document::SemanticBlock, use_signals: bool) -> Vec<String> {
    let raw: Vec<&str> = if use_signals {
        block.noun_signals.iter().map(|s| s.text.as_str()).collect()
    } else {
        block
            .tokens
            .iter()
            .filter(|t| NOUN_POS.contains(&t.pos.as_str()))
            .map(|t| t.text.as_str())
            .collect()
    };

    let mut terms: Vec<String> = raw.iter().filter_map(|t| normalize(t)).collect();
    terms.sort();
    terms.dedup();
    terms
}

/// Trim, lowercase and reject what cannot carry a subject: single characters,
/// punctuation-only runs, numbers and the generic words that top every frequency
/// list.
fn normalize(text: &str) -> Option<String> {
    let trimmed = text.trim().to_lowercase();
    if trimmed.chars().count() < 2 {
        return None;
    }
    if STOP_TERMS.contains(&trimmed.as_str()) {
        return None;
    }
    if !trimmed
        .chars()
        .any(|c| c.is_alphabetic() || crate::text::is_cjk(c))
    {
        return None;
    }
    Some(trimmed)
}

/// Disjoint-set over the term list. The lower index always wins a union, so a
/// cluster is keyed by the alphabetically first term in it and the grouping is
/// reproducible.
#[derive(Debug)]
struct Universe {
    parent: Vec<usize>,
}

impl Universe {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
        }
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        if ra < rb {
            self.parent[rb] = ra;
        } else {
            self.parent[ra] = rb;
        }
    }
}

/// Fold terms into subjects: one cluster per group linked by containment (出版 /
/// 出版业) or by often sharing a block.
///
/// Both links are needed. Containment without co-occurrence would split 出版 and
/// 出版业 because they never appear in the same sentence; co-occurrence without
/// containment would split a subject whose words are synonyms rather than
/// compounds. Clusters are returned in the order their first term appears in the
/// sorted term list, so the grouping never depends on HashMap order.
fn cluster_terms(
    occurrences: &HashMap<String, Candidate>,
    doc: &DocumentData,
    use_signals: bool,
) -> Vec<Vec<String>> {
    let mut terms: Vec<String> = occurrences.keys().cloned().collect();
    terms.sort();
    let index: HashMap<&str, usize> = terms
        .iter()
        .enumerate()
        .map(|(i, t)| (t.as_str(), i))
        .collect();

    let mut universe = Universe::new(terms.len());

    for i in 0..terms.len() {
        for j in (i + 1)..terms.len() {
            let (a, b) = (&terms[i], &terms[j]);
            if a != b && (a.contains(b.as_str()) || b.contains(a.as_str())) {
                universe.union(i, j);
            }
        }
    }

    let mut cooc: HashMap<(usize, usize), u32> = HashMap::new();
    for block in &doc.blocks {
        let present: Vec<usize> = block_terms(block, use_signals)
            .iter()
            .filter_map(|t| index.get(t.as_str()).copied())
            .collect();
        for i in 0..present.len() {
            for j in (i + 1)..present.len() {
                *cooc.entry((present[i], present[j])).or_insert(0) += 1;
            }
        }
    }
    let mut pairs: Vec<(usize, usize)> = cooc
        .iter()
        .filter(|(_, count)| **count >= COOC_MIN)
        .map(|((a, b), _)| (*a, *b))
        .collect();
    pairs.sort();
    for (a, b) in pairs {
        universe.union(a, b);
    }

    let mut groups: HashMap<usize, Vec<String>> = HashMap::new();
    for (i, term) in terms.iter().enumerate() {
        let root = universe.find(i);
        groups.entry(root).or_default().push(term.clone());
    }
    let mut roots: Vec<usize> = groups.keys().copied().collect();
    roots.sort_unstable();
    roots
        .into_iter()
        .map(|r| groups.remove(&r).expect("root was collected above"))
        .collect()
}

/// Turn one cluster into a theme, or drop it: a subject needs to be said more than
/// twice and in more than one place.
fn build_theme(
    members: Vec<String>,
    occurrences: &HashMap<String, Candidate>,
    plan: &crate::segment::SegmentPlan,
    section_count: usize,
    total_mentions: u32,
) -> Option<Theme> {
    let mut ranked: Vec<(String, u32)> = members
        .iter()
        .filter_map(|t| occurrences.get(t).map(|c| (t.clone(), c.mentions)))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mentions: u32 = ranked.iter().map(|(_, m)| *m).sum();
    if mentions < MIN_MENTIONS {
        return None;
    }

    let mut by_section: HashMap<usize, u32> = HashMap::new();
    for term in &members {
        let candidate = occurrences.get(term)?;
        for (section, count) in &candidate.by_section {
            *by_section.entry(*section).or_insert(0) += *count;
        }
    }
    let sections_touched = by_section.len();
    if sections_touched < 2 {
        // Said three times in one place is emphasis, not a thread running through
        // the document.
        return None;
    }

    let first = by_section.keys().copied().min().unwrap_or(0);
    let last = by_section.keys().copied().max().unwrap_or(0);
    let coverage = if section_count == 0 {
        0.0
    } else {
        sections_touched as f64 / section_count as f64
    };
    let curve: Vec<u32> = (0..section_count)
        .map(|i| by_section.get(&i).copied().unwrap_or(0))
        .collect();

    Some(Theme {
        term: ranked.first().map(|(t, _)| t.clone()).unwrap_or_default(),
        members: ranked.into_iter().take(MAX_MEMBERS).collect(),
        mentions,
        sections_touched: sections_touched as u32,
        coverage,
        share: if total_mentions == 0 {
            0.0
        } else {
            mentions as f64 / total_mentions as f64
        },
        first_section: plan.labels.get(first).cloned().unwrap_or_default(),
        last_section: plan.labels.get(last).cloned().unwrap_or_default(),
        reach: reach(coverage, sections_touched).to_string(),
        longest_gap: longest_gap(&curve),
        concentration: concentration(&by_section, section_count, mentions).to_string(),
        curve,
    })
}

/// How far the theme reaches across the document.
fn reach(coverage: f64, sections_touched: usize) -> &'static str {
    if sections_touched <= 2 {
        "episodic"
    } else if coverage >= SUSTAINED_COVERAGE {
        "sustained"
    } else if coverage >= RECURRING_COVERAGE {
        "recurring"
    } else {
        "occasional"
    }
}

/// Longest run of segments where the theme is absent.
fn longest_gap(curve: &[u32]) -> u32 {
    let mut longest = 0u32;
    let mut run = 0u32;
    for value in curve {
        if *value == 0 {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

/// Whether the theme's mentions sit in one third of the document.
fn concentration(
    by_section: &HashMap<usize, u32>,
    section_count: usize,
    mentions: u32,
) -> &'static str {
    if section_count < 3 || mentions == 0 {
        return "spread";
    }
    let front_end = section_count / 3;
    let middle_end = section_count * 2 / 3;
    let (mut front, mut middle, mut back) = (0u32, 0u32, 0u32);
    for (section, count) in by_section {
        if *section < front_end {
            front += *count;
        } else if *section < middle_end {
            middle += *count;
        } else {
            back += *count;
        }
    }
    let total = mentions as f64;
    if back as f64 >= THIRD_HEAVY * total {
        "back"
    } else if middle as f64 >= THIRD_HEAVY * total {
        "middle"
    } else if front as f64 >= THIRD_HEAVY * total {
        "front"
    } else {
        "spread"
    }
}

/// 0-100, or `None` when no usable noun was found.
///
/// Starts at 100 and charges for what a term frequency can actually see:
///
/// - no theme reaching across the document (`RECURRING_COVERAGE` of the segments):
///   −25 — there is no subject being carried;
/// - one theme holding more than `OVER_CONCENTRATED` of every mention: up to −15,
///   the document repeats one word instead of building a set of subjects;
/// - the top three holding less than `SCATTERED_TOP3`: up to −20 — nothing dominates;
/// - terms seen exactly once carrying more than `SINGLETON_HEAVY` of the mentions:
///   up to −25 — the nouns never come back, so no theme was ever established.
///
/// What it cannot see is the difference between a motif and a genre requirement:
/// a war novel that says 战壕 in every chapter scores well here whether or not the
/// trenches mean anything.
fn theme_score(stats: &ThemeStats) -> Option<f64> {
    if stats.candidate_count == 0 || stats.total_mentions == 0 {
        return None;
    }

    let no_thread = if stats
        .themes
        .iter()
        .any(|t| t.coverage >= RECURRING_COVERAGE)
    {
        0.0
    } else {
        25.0
    };
    let heavy = ((stats.top_share - OVER_CONCENTRATED) / 0.2 * 15.0).clamp(0.0, 15.0);
    let scattered = if stats.top3_share < SCATTERED_TOP3 {
        (SCATTERED_TOP3 - stats.top3_share) / SCATTERED_TOP3 * 20.0
    } else {
        0.0
    };
    let singleton = if stats.singleton_share > SINGLETON_HEAVY {
        ((stats.singleton_share - SINGLETON_HEAVY) * 50.0).min(25.0)
    } else {
        0.0
    };

    Some((100.0 - no_thread - heavy - scattered - singleton).clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;
    use studio_core::document::Token;
    use studio_core::noun_signal::NounSignal;

    fn signal(text: &str) -> NounSignal {
        NounSignal {
            text: text.to_string(),
            pos: "NN".to_string(),
            syntactic_role: None,
            score: 0.7,
            span: (0, 0),
            evidence: None,
        }
    }

    fn token(text: &str, pos: &str) -> Token {
        Token {
            text: text.to_string(),
            pos: pos.to_string(),
            confidence: 1.0,
            span: (0, 0),
            source: "test".to_string(),
        }
    }

    fn block(path: &str, signals: &[&str], tokens: &[(&str, &str)]) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: "内容".into(),
            section_path: path.into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: tokens.iter().map(|(t, p)| token(t, p)).collect(),
            entities: vec![],
            noun_signals: signals.iter().map(|s| signal(s)).collect(),
        }
    }

    fn doc_of(blocks: Vec<SemanticBlock>) -> DocumentData {
        DocumentData {
            doc_id: uuid::Uuid::nil(),
            title: "Test".into(),
            blocks,
            total_word_count: 0,
            total_char_count: 0,
        }
    }

    #[test]
    fn test_a_recurring_term_becomes_a_theme() {
        let doc = doc_of(vec![
            block("S1", &["出版", "印刷"], &[]),
            block("S2", &["出版", "发行"], &[]),
            block("S3", &["出版", "编辑"], &[]),
            block("S4", &["出版", "版权"], &[]),
            block("S5", &["出版", "读者"], &[]),
        ]);
        let stats = run_theme_analysis(&doc).expect("theme analysis");

        assert_eq!(stats.source, "noun_signals");
        assert_eq!(stats.themes.len(), 1);
        let theme = &stats.themes[0];
        assert_eq!(theme.term, "出版");
        assert_eq!(theme.mentions, 5);
        assert_eq!(theme.sections_touched, 5);
        assert!((theme.coverage - 1.0).abs() < 1e-9);
        assert!((theme.share - 0.5).abs() < 1e-9);
        assert_eq!(theme.reach, "sustained");
        assert_eq!(theme.concentration, "spread");
        assert_eq!(theme.curve, vec![1, 1, 1, 1, 1]);
        assert_eq!(stats.dominant_terms, vec!["出版".to_string()]);
        assert_eq!(stats.total_mentions, 10);
        // One term holding half of every mention is repetition, not a set of
        // subjects, so the score is charged for it without being catastrophic.
        let score = stats.theme_score.expect("scored");
        assert!(score > 80.0 && score < 95.0, "score was {score}");
    }

    #[test]
    fn test_noun_signals_are_preferred_over_tokens() {
        let doc = doc_of(vec![
            block("S1", &["合同"], &[("机器", "NN")]),
            block("S2", &["合同"], &[("机器", "NN")]),
            block("S3", &["合同"], &[("机器", "NN")]),
        ]);
        let stats = run_theme_analysis(&doc).expect("theme analysis");

        assert_eq!(stats.source, "noun_signals");
        assert_eq!(stats.themes.len(), 1);
        assert_eq!(stats.themes[0].term, "合同");
        assert!(stats.dominant_terms.iter().all(|t| t != "机器"));
    }

    #[test]
    fn test_tokens_are_used_when_the_import_has_no_signals() {
        let doc = doc_of(vec![
            block("S1", &[], &[("出版", "NN"), ("张三", "NR"), ("昨天", "NT")]),
            block("S2", &[], &[("出版", "NN"), ("张三", "NR"), ("昨天", "NT")]),
            block("S3", &[], &[("出版", "NN"), ("张三", "NR"), ("昨天", "NT")]),
        ]);
        let stats = run_theme_analysis(&doc).expect("theme analysis");

        assert_eq!(stats.source, "noun_tokens");
        assert_eq!(stats.themes.len(), 1);
        assert_eq!(stats.themes[0].term, "出版");
    }

    #[test]
    fn test_names_and_dates_never_become_themes() {
        let doc = doc_of(vec![
            block("S1", &[], &[("张三", "NR"), ("昨天", "NT")]),
            block("S2", &[], &[("张三", "NR"), ("昨天", "NT")]),
            block("S3", &[], &[("张三", "NR"), ("昨天", "NT")]),
        ]);
        let stats = run_theme_analysis(&doc).expect("theme analysis");

        assert!(stats.themes.is_empty());
        assert_eq!(stats.candidate_count, 0);
        assert_eq!(stats.theme_score, None);
    }

    #[test]
    fn test_compounds_are_folded_into_one_theme() {
        let doc = doc_of(vec![
            block("S1", &["出版"], &[]),
            block("S2", &["出版", "出版业"], &[]),
            block("S3", &["出版", "出版业"], &[]),
            block("S4", &["出版", "出版业"], &[]),
            block("S5", &["出版"], &[]),
        ]);
        let stats = run_theme_analysis(&doc).expect("theme analysis");

        assert_eq!(stats.themes.len(), 1);
        let theme = &stats.themes[0];
        // The cluster is named after its busiest member, not the shortest term.
        assert_eq!(theme.term, "出版");
        assert_eq!(theme.mentions, 8);
        let members: Vec<&str> = theme.members.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(members, ["出版", "出版业"]);
    }

    #[test]
    fn test_terms_that_share_their_blocks_are_folded_together() {
        let doc = doc_of(vec![
            block("S1", &["合同", "条款"], &[]),
            block("S2", &["合同", "条款"], &[]),
            block("S3", &["合同", "条款"], &[]),
            block("S4", &["合同"], &[]),
        ]);
        let stats = run_theme_analysis(&doc).expect("theme analysis");

        assert_eq!(stats.themes.len(), 1);
        let theme = &stats.themes[0];
        assert_eq!(theme.term, "合同");
        assert_eq!(theme.mentions, 7);
        let members: Vec<&str> = theme.members.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(members, ["合同", "条款"]);
    }

    #[test]
    fn test_mentions_piled_at_the_end_are_reported_as_such() {
        let doc = doc_of(vec![
            block("S1", &["序言"], &[]),
            block("S2", &["前言"], &[]),
            block("S3", &["导论"], &[]),
            block("S4", &["雨水"], &[]),
            block("S5", &["雨水"], &[]),
            block("S6", &["雨水"], &[]),
        ]);
        let stats = run_theme_analysis(&doc).expect("theme analysis");

        assert_eq!(stats.themes.len(), 1);
        assert_eq!(stats.themes[0].term, "雨水");
        assert_eq!(stats.themes[0].concentration, "back");
        assert_eq!(stats.themes[0].first_section, "S4");
        assert_eq!(stats.themes[0].last_section, "S6");
        assert_eq!(stats.themes[0].longest_gap, 3);
    }

    #[test]
    fn test_terms_that_never_repeat_leave_no_theme() {
        let doc = doc_of(vec![
            block("S1", &["印刷"], &[]),
            block("S2", &["发行"], &[]),
            block("S3", &["编辑"], &[]),
            block("S4", &["版权"], &[]),
            block("S5", &["读者"], &[]),
            block("S6", &["书评"], &[]),
        ]);
        let stats = run_theme_analysis(&doc).expect("theme analysis");

        assert!(stats.themes.is_empty());
        assert_eq!(stats.candidate_count, 0);
        assert_eq!(stats.total_mentions, 6);
        assert!((stats.singleton_share - 1.0).abs() < 1e-9);
        // Varied vocabulary with nothing coming back is not a scored theme list.
        assert_eq!(stats.theme_score, None);
    }

    #[test]
    fn test_a_running_subject_is_scored_and_a_scattered_one_is_not() {
        let running = run_theme_analysis(&doc_of(vec![
            block("S1", &["出版", "印刷"], &[]),
            block("S2", &["出版", "发行"], &[]),
            block("S3", &["出版", "编辑"], &[]),
            block("S4", &["出版", "版权"], &[]),
            block("S5", &["出版", "读者"], &[]),
            block("S6", &["出版", "书评"], &[]),
        ]))
        .expect("theme analysis");
        let scattered = run_theme_analysis(&doc_of(vec![
            block("S1", &["印刷", "装订", "纸张"], &[]),
            block("S2", &["发行", "渠道", "书店"], &[]),
            block("S3", &["编辑", "校对", "排版"], &[]),
            block("S4", &["版权", "合同", "条款"], &[]),
            block("S5", &["读者", "书评", "销量"], &[]),
            block("S6", &["市场", "定价", "印数"], &[]),
        ]))
        .expect("theme analysis");

        // The document with a subject running through it is judged; the one whose
        // nouns never come back has no subject to judge and is left unscored rather
        // than given a low score that would read as a weak theme list.
        assert!(running.theme_score.expect("running scored") > 80.0);
        assert_eq!(scattered.theme_score, None);
        assert!(scattered.singleton_share > running.singleton_share);
    }

    #[test]
    fn test_generic_words_are_not_themes() {
        let doc = doc_of(vec![
            block("S1", &["问题", "事情"], &[]),
            block("S2", &["问题", "事情"], &[]),
            block("S3", &["问题", "事情"], &[]),
        ]);
        let stats = run_theme_analysis(&doc).expect("theme analysis");

        assert!(stats.themes.is_empty());
        assert_eq!(stats.total_mentions, 0);
        assert_eq!(stats.theme_score, None);
    }

    #[test]
    fn test_empty_document_has_no_themes() {
        let stats = run_theme_analysis(&doc_of(vec![])).expect("theme analysis");

        assert_eq!(stats.source, "none");
        assert!(stats.themes.is_empty());
        assert_eq!(stats.section_count, 0);
        assert_eq!(stats.theme_score, None);
    }

    #[test]
    fn test_same_document_yields_identical_output() {
        let doc = doc_of(vec![
            block("S1", &["出版", "出版业", "印刷"], &[]),
            block("S2", &["出版", "合同"], &[]),
            block("S3", &["出版业", "合同", "条款"], &[]),
            block("S4", &["出版", "印刷"], &[]),
        ]);
        let a = serde_json::to_string(&run_theme_analysis(&doc).unwrap()).unwrap();
        let b = serde_json::to_string(&run_theme_analysis(&doc).unwrap()).unwrap();
        assert_eq!(a, b);
    }
}
