//! Plot analysis (情节分析): plotlines extracted from the entity flow.
//!
//! The design doc describes plot extraction as "entity flow + LLM". The LLM half is
//! out of scope for the local engine, so this module does the part that can be done
//! without one — and defines it precisely enough to be tested: a **plotline** is a
//! named entity or domain term tracked section by section, and the **coherence** of
//! the plot is inferred from how far those threads run, where they drop out, and
//! where they arrive from.
//!
//! Everything here is deterministic: the same document always yields the same
//! threads, the same shape labels and the same score.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;

use crate::t1::PLOT_ENTITY_CATEGORIES;
use crate::Result;

/// A thread needs this many mentions to be a plotline rather than a passing name.
const MIN_MENTIONS: u32 = 3;
/// How many plotlines are reported, main threads first.
const MAX_PLOTLINES: usize = 12;
/// A thread covering at least this fraction of the sections can carry the story.
const MAIN_COVERAGE: f64 = 0.25;
/// A thread covering at least this fraction runs through the whole document.
const SUSTAINED_COVERAGE: f64 = 0.5;
/// A thread first appearing in the last fifth of the sections arrives too late to
/// have been earned by what came before.
const LATE_ZONE: f64 = 0.8;
/// …and one gone by the end of the first fifth is setup that pays off nowhere else.
const EARLY_ZONE: f64 = 0.2;
/// Below this many sections, "setup" and "late arrival" carry no information, so
/// nothing is judged and nothing is scored.
const MIN_SECTIONS: usize = 3;

/// One appearance of a thread: a section, how strongly it is present there, and how
/// many sections it had been absent for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotlineStage {
    pub section: String,
    pub mentions: u32,
    pub gap_before: u32,
}

/// A plotline: one thread carried by a recurring entity or domain term.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plotline {
    pub subject: String,
    /// Entity category (`PERSON`, `LOCATION`, …) or `SIGNAL` for a noun signal.
    pub kind: String,
    pub mentions: u32,
    pub sections_touched: u32,
    /// Fraction of the document's sections this thread appears in (0-1).
    pub coverage: f64,
    pub first_section: String,
    pub last_section: String,
    pub stages: Vec<PlotlineStage>,
    /// Longest run of consecutive sections where the thread is absent.
    pub longest_gap: u32,
    /// `sustained` | `intermittent` | `burst` | `setup_only` | `late_arrival` | `minor`
    pub shape: String,
    pub is_main: bool,
}

/// A section where the set of active threads changes sharply — the deterministic
/// stand-in for a plot turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurningPoint {
    pub section: String,
    /// 0-1: how different the active threads are from the previous section.
    pub change: f64,
    pub appeared: Vec<String>,
    pub dropped: Vec<String>,
}

/// Plot analysis output for one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotStats {
    pub plotlines: Vec<Plotline>,
    /// Threads that cleared `MIN_MENTIONS`, including ones not listed above.
    pub thread_count: u32,
    pub main_plotlines: Vec<String>,
    pub sub_plotlines: Vec<String>,
    pub turning_points: Vec<TurningPoint>,
    /// Threads mentioned in a single section: setup with no payoff, or noise.
    pub unresolved_threads: Vec<String>,
    /// Threads first appearing in the last fifth of the document.
    pub late_arrivals: Vec<String>,
    /// 0-100; [`coherence_score`] states exactly what it penalises.
    pub coherence_score: f64,
    pub section_count: u32,
}

/// Where every narrative-bearing subject shows up.
struct ThreadMap {
    /// Sections in reading order: a thread only means something relative to where
    /// the document has got to when it appears.
    sections: Vec<String>,
    section_of: HashMap<String, usize>,
    /// subject -> (section index -> mentions in that section)
    presence: HashMap<String, HashMap<usize, u32>>,
    /// subject -> entity category, or `SIGNAL` for a noun signal.
    kind: HashMap<String, String>,
}

/// Extract the plotlines of a document from its entity and noun-signal flow.
pub fn run_plot_analysis(doc: &DocumentData) -> Result<PlotStats> {
    let map = collect_presence(doc);
    let section_count = map.sections.len();

    let mut threads: Vec<Plotline> = map
        .presence
        .into_iter()
        .filter_map(|(subject, by_section)| {
            build_thread(subject, by_section, &map.kind, &map.sections, section_count)
        })
        .collect();

    // Coverage first: a thread that runs through the document carries the story even
    // when a bursty one is mentioned more often. Ties fall back to mentions and then
    // to the subject, so the order never changes between runs.
    threads.sort_by(|a, b| {
        b.coverage
            .total_cmp(&a.coverage)
            .then(b.mentions.cmp(&a.mentions))
            .then(a.subject.cmp(&b.subject))
    });

    let thread_count = threads.len() as u32;
    let turning_points = find_turning_points(&threads, &map.sections, &map.section_of);
    let unresolved_threads = single_section_threads(&threads, section_count);
    let late_arrivals = late_thread_list(&threads, section_count);
    // Scored over every thread that cleared the mention floor, not just the ones
    // reported below.
    let coherence_score = coherence_score(section_count, &threads);

    let main_plotlines: Vec<String> = threads
        .iter()
        .take(3)
        .filter(|t| t.coverage >= MAIN_COVERAGE)
        .map(|t| t.subject.clone())
        .collect();
    for t in threads.iter_mut() {
        t.is_main = main_plotlines.contains(&t.subject);
    }

    threads.truncate(MAX_PLOTLINES);
    let sub_plotlines: Vec<String> = threads
        .iter()
        .filter(|t| !t.is_main)
        .map(|t| t.subject.clone())
        .collect();

    Ok(PlotStats {
        plotlines: threads,
        thread_count,
        main_plotlines,
        sub_plotlines,
        turning_points,
        unresolved_threads,
        late_arrivals,
        coherence_score,
        section_count: section_count as u32,
    })
}

/// Collect every narrative-bearing subject and the sections it appears in.
fn collect_presence(doc: &DocumentData) -> ThreadMap {
    // Sections in reading order: a thread only means something relative to where the
    // document has got to when it appears.
    let mut sections: Vec<String> = Vec::new();
    let mut section_of: HashMap<String, usize> = HashMap::new();
    for block in &doc.blocks {
        if !section_of.contains_key(&block.section_path) {
            section_of.insert(block.section_path.clone(), sections.len());
            sections.push(block.section_path.clone());
        }
    }

    // subject -> (section index -> mentions in that section)
    let mut presence: HashMap<String, HashMap<usize, u32>> = HashMap::new();
    let mut kind: HashMap<String, String> = HashMap::new();
    for block in &doc.blocks {
        let idx = *section_of
            .get(&block.section_path)
            .expect("every section was registered above");
        let mut subjects: Vec<(String, &str)> = block
            .entities
            .iter()
            .filter(|e| PLOT_ENTITY_CATEGORIES.contains(&e.category))
            .map(|e| (e.text.clone(), e.category.as_str()))
            .collect();
        subjects.extend(
            block
                .noun_signals
                .iter()
                .map(|s| (s.text.clone(), "SIGNAL")),
        );
        for (text, category) in subjects {
            if text.trim().is_empty() {
                continue;
            }
            *presence
                .entry(text.clone())
                .or_default()
                .entry(idx)
                .or_insert(0) += 1;
            // A named entity wins over a noun signal for the same text: the NER
            // result is the more specific claim about what the string is.
            kind.entry(text).or_insert_with(|| category.to_string());
        }
    }
    ThreadMap {
        sections,
        section_of,
        presence,
        kind,
    }
}

/// Turn one subject's section map into a plotline, or drop it when it is only
/// mentioned in passing.
fn build_thread(
    subject: String,
    by_section: HashMap<usize, u32>,
    kind: &HashMap<String, String>,
    sections: &[String],
    section_count: usize,
) -> Option<Plotline> {
    let mentions: u32 = by_section.values().sum();
    if mentions < MIN_MENTIONS {
        return None;
    }
    let mut idxs: Vec<usize> = by_section.keys().copied().collect();
    idxs.sort_unstable();

    let mut stages: Vec<PlotlineStage> = Vec::with_capacity(idxs.len());
    let mut longest_gap = 0u32;
    let mut prev: Option<usize> = None;
    for idx in &idxs {
        let gap_before = prev.map(|p| (*idx - p - 1) as u32).unwrap_or(0);
        longest_gap = longest_gap.max(gap_before);
        stages.push(PlotlineStage {
            section: sections[*idx].clone(),
            mentions: by_section[idx],
            gap_before,
        });
        prev = Some(*idx);
    }

    let coverage = if section_count > 0 {
        idxs.len() as f64 / section_count as f64
    } else {
        0.0
    };
    let subject_kind = kind
        .get(&subject)
        .cloned()
        .unwrap_or_else(|| "SIGNAL".to_string());

    Some(Plotline {
        subject,
        kind: subject_kind,
        mentions,
        sections_touched: idxs.len() as u32,
        coverage,
        first_section: sections[idxs[0]].clone(),
        last_section: sections[idxs[idxs.len() - 1]].clone(),
        stages,
        longest_gap,
        shape: classify_shape(
            coverage,
            longest_gap,
            idxs[0],
            idxs[idxs.len() - 1],
            section_count,
        )
        .to_string(),
        is_main: false,
    })
}

/// Classify how a thread runs through the document. Position is checked before
/// length: a thread that only surfaces in the last section is a late arrival even
/// though it is also short.
fn classify_shape(
    coverage: f64,
    longest_gap: u32,
    first_idx: usize,
    last_idx: usize,
    section_count: usize,
) -> &'static str {
    if section_count >= MIN_SECTIONS {
        let last_slot = (section_count - 1) as f64;
        if first_idx as f64 >= last_slot * LATE_ZONE {
            return "late_arrival";
        }
        if last_idx as f64 <= last_slot * EARLY_ZONE {
            return "setup_only";
        }
    }
    if coverage >= SUSTAINED_COVERAGE {
        return "sustained";
    }
    if longest_gap >= 2 {
        return "intermittent";
    }
    "minor"
}

/// The sections where the set of active threads changes the most.
///
/// Only threads that cleared `MIN_MENTIONS` count as active. The change is the
/// symmetric difference over the union of the two sections' thread sets: 0 when the
/// same threads carry both sections, 1 when they share none. This is a structural
/// proxy for a plot turn — it says the cast changed here, not why.
fn find_turning_points(
    threads: &[Plotline],
    sections: &[String],
    section_of: &HashMap<String, usize>,
) -> Vec<TurningPoint> {
    let mut active: Vec<HashSet<String>> = vec![HashSet::new(); sections.len()];
    for t in threads {
        for stage in &t.stages {
            if let Some(idx) = section_of.get(&stage.section) {
                active[*idx].insert(t.subject.clone());
            }
        }
    }

    let mut turns: Vec<TurningPoint> = Vec::new();
    for idx in 1..sections.len() {
        let before = &active[idx - 1];
        let after = &active[idx];
        let union = before.union(after).count();
        if union == 0 {
            continue;
        }
        let changed = before.difference(after).count() + after.difference(before).count();
        if changed == 0 {
            continue;
        }
        let mut appeared: Vec<String> = after.difference(before).cloned().collect();
        let mut dropped: Vec<String> = before.difference(after).cloned().collect();
        appeared.sort();
        dropped.sort();
        appeared.truncate(8);
        dropped.truncate(8);
        turns.push(TurningPoint {
            section: sections[idx].clone(),
            change: changed as f64 / union as f64,
            appeared,
            dropped,
        });
    }
    turns.sort_by(|a, b| {
        b.change
            .total_cmp(&a.change)
            .then(a.section.cmp(&b.section))
    });
    turns.truncate(3);
    turns
}

/// Threads mentioned in exactly one section: setup that is never picked up, or a name
/// that means nothing to the plot. Only judged once there are enough sections for the
/// distinction to mean anything.
fn single_section_threads(threads: &[Plotline], section_count: usize) -> Vec<String> {
    if section_count < MIN_SECTIONS {
        return vec![];
    }
    let mut out: Vec<(u32, String)> = threads
        .iter()
        .filter(|t| t.sections_touched == 1)
        .map(|t| (t.mentions, t.subject.clone()))
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    out.into_iter().map(|(_, s)| s).take(10).collect()
}

/// Threads whose first appearance is in the last fifth of the document.
fn late_thread_list(threads: &[Plotline], section_count: usize) -> Vec<String> {
    if section_count < MIN_SECTIONS {
        return vec![];
    }
    threads
        .iter()
        .filter(|t| t.shape == "late_arrival")
        .map(|t| t.subject.clone())
        .take(10)
        .collect()
}

/// 0-100. Starts at 100 and subtracts for the structural problems a thread map can
/// actually see: setup that goes nowhere, threads arriving too late to be earned,
/// threads abandoned for long stretches, and no thread running through the document
/// at all. Documents with too few sections are not scored, matching how the
/// assessment's structure dimension treats them.
fn coherence_score(section_count: usize, threads: &[Plotline]) -> f64 {
    if section_count < MIN_SECTIONS || threads.is_empty() {
        return 100.0;
    }
    let mut score = 100.0f64;
    for t in threads {
        match t.shape.as_str() {
            "setup_only" | "late_arrival" => score -= 8.0,
            _ => {}
        }
        if t.longest_gap >= 3 {
            score -= 5.0;
        }
    }
    if !threads.iter().any(|t| t.coverage >= SUSTAINED_COVERAGE) {
        score -= 15.0;
    }
    score.clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;
    use studio_core::entity::{Entity, EntityCategory};
    use studio_core::noun_signal::NounSignal;

    fn person(path: &str, names: &[&str]) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: "text".into(),
            section_path: path.into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: names
                .iter()
                .map(|n| Entity {
                    text: (*n).into(),
                    category: EntityCategory::Person,
                    confidence: 0.9,
                    source: "test".into(),
                    keep: true,
                    filter: None,
                    filter_reason: None,
                    span: (0, 0),
                })
                .collect(),
            noun_signals: vec![],
        }
    }

    fn signal(path: &str, texts: &[&str]) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: "text".into(),
            section_path: path.into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: texts
                .iter()
                .map(|t| NounSignal {
                    text: (*t).into(),
                    pos: "NOUN".into(),
                    syntactic_role: None,
                    score: 0.5,
                    span: (0, 0),
                    evidence: None,
                })
                .collect(),
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
    fn test_sustained_thread_is_main() {
        let doc = doc_of(vec![
            person("S1", &["Marlowe"]),
            person("S2", &["Marlowe"]),
            person("S3", &["Marlowe"]),
            person("S4", &["Marlowe"]),
            person("S5", &["Marlowe"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        assert_eq!(stats.plotlines.len(), 1);
        let line = &stats.plotlines[0];
        assert_eq!(line.subject, "Marlowe");
        assert_eq!(line.mentions, 5);
        assert_eq!(line.coverage, 1.0);
        assert_eq!(line.shape, "sustained");
        assert!(line.is_main);
        assert_eq!(stats.main_plotlines, vec!["Marlowe".to_string()]);
        assert_eq!(stats.coherence_score, 100.0);
    }

    #[test]
    fn test_passing_name_is_not_a_thread() {
        let doc = doc_of(vec![
            person("S1", &["Marlowe", "Nixon"]),
            person("S2", &["Marlowe"]),
            person("S3", &["Marlowe"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        assert_eq!(stats.thread_count, 1);
        assert_eq!(stats.plotlines[0].subject, "Marlowe");
    }

    #[test]
    fn test_thread_appearing_only_at_the_end_is_a_late_arrival() {
        let doc = doc_of(vec![
            person("S1", &["Marlowe"]),
            person("S2", &["Marlowe"]),
            person("S3", &["Marlowe"]),
            person("S4", &["Marlowe"]),
            person("S5", &["Marlowe", "Tobey", "Tobey", "Tobey"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        let tobey = stats
            .plotlines
            .iter()
            .find(|p| p.subject == "Tobey")
            .unwrap();
        assert_eq!(tobey.shape, "late_arrival");
        assert_eq!(stats.late_arrivals, vec!["Tobey".to_string()]);
    }

    #[test]
    fn test_thread_dropped_after_the_opening_is_unresolved() {
        let doc = doc_of(vec![
            person("S1", &["Marlowe", "Chase", "Chase", "Chase"]),
            person("S2", &["Marlowe"]),
            person("S3", &["Marlowe"]),
            person("S4", &["Marlowe"]),
            person("S5", &["Marlowe"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        let chase = stats
            .plotlines
            .iter()
            .find(|p| p.subject == "Chase")
            .unwrap();
        assert_eq!(chase.shape, "setup_only");
        assert_eq!(stats.unresolved_threads, vec!["Chase".to_string()]);
        assert!(stats.coherence_score < 100.0);
    }

    #[test]
    fn test_long_absence_is_intermittent() {
        let doc = doc_of(vec![
            person("S1", &["Marlowe", "Ty", "Ty"]),
            person("S2", &["Marlowe"]),
            person("S3", &["Marlowe"]),
            person("S4", &["Marlowe", "Ty"]),
            person("S5", &["Marlowe"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        let ty = stats.plotlines.iter().find(|p| p.subject == "Ty").unwrap();
        assert_eq!(ty.mentions, 3);
        assert_eq!(ty.longest_gap, 2);
        assert_eq!(ty.shape, "intermittent");
        assert_eq!(ty.stages[1].gap_before, 2);
    }

    #[test]
    fn test_cast_change_is_a_turning_point() {
        let doc = doc_of(vec![
            person("S1", &["Marlowe", "Marlowe"]),
            person("S2", &["Marlowe", "Marlowe"]),
            person("S3", &["Nixon", "Nixon", "Nixon"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        assert_eq!(stats.turning_points.len(), 1);
        let turn = &stats.turning_points[0];
        assert_eq!(turn.section, "S3");
        assert_eq!(turn.change, 1.0);
        assert_eq!(turn.appeared, vec!["Nixon".to_string()]);
        assert_eq!(turn.dropped, vec!["Marlowe".to_string()]);
    }

    #[test]
    fn test_dangling_threads_cost_coherence() {
        let clean = doc_of(vec![
            person("S1", &["Marlowe"]),
            person("S2", &["Marlowe"]),
            person("S3", &["Marlowe"]),
            person("S4", &["Marlowe"]),
            person("S5", &["Marlowe"]),
        ]);
        let ragged = doc_of(vec![
            person("S1", &["Marlowe", "Chase", "Chase", "Chase"]),
            person("S2", &["Marlowe", "Dana", "Dana", "Dana"]),
            person("S3", &["Marlowe"]),
            person("S4", &["Marlowe"]),
            person("S5", &["Marlowe", "Tobey", "Tobey", "Tobey"]),
        ]);
        let clean_score = run_plot_analysis(&clean).unwrap().coherence_score;
        let ragged_score = run_plot_analysis(&ragged).unwrap().coherence_score;
        assert_eq!(clean_score, 100.0);
        // Chase opened and never returned (-8), Tobey arrived at the end (-8); Dana
        // appears mid-document, which is neither, so it costs nothing.
        assert!((ragged_score - 84.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_no_thread_runs_through_costs_coherence() {
        let doc = doc_of(vec![
            person("S1", &["Chase", "Chase", "Chase"]),
            person("S2", &["Dana", "Dana", "Dana"]),
            person("S3", &["Ty", "Ty", "Ty"]),
            person("S4", &["Nixon", "Nixon", "Nixon"]),
            person("S5", &["Tobey", "Tobey", "Tobey"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        // Nothing is sustained (-15); Chase opened and never returned (-8), Tobey
        // arrived at the end (-8).
        assert!((stats.coherence_score - 69.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_noun_signal_carries_a_thread() {
        let doc = doc_of(vec![
            signal("S1", &["obsession"]),
            signal("S2", &["obsession"]),
            signal("S3", &["obsession"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        assert_eq!(stats.plotlines[0].subject, "obsession");
        assert_eq!(stats.plotlines[0].kind, "SIGNAL");
    }

    #[test]
    fn test_at_most_three_main_threads() {
        let doc = doc_of(vec![
            person("S1", &["A", "B", "C", "D"]),
            person("S2", &["A", "B", "C", "D"]),
            person("S3", &["A", "B", "C", "D"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        assert_eq!(stats.main_plotlines.len(), 3);
        assert_eq!(stats.plotlines.iter().filter(|p| p.is_main).count(), 3);
        assert_eq!(stats.sub_plotlines.len(), 1);
    }

    #[test]
    fn test_short_documents_are_not_judged() {
        let doc = doc_of(vec![
            person("S1", &["Marlowe", "Chase", "Chase"]),
            person("S2", &["Marlowe"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        assert_eq!(stats.section_count, 2);
        assert!(stats.unresolved_threads.is_empty());
        assert!(stats.late_arrivals.is_empty());
        assert_eq!(stats.coherence_score, 100.0);
    }

    #[test]
    fn test_empty_document() {
        let stats = run_plot_analysis(&doc_of(vec![])).unwrap();
        assert_eq!(stats.section_count, 0);
        assert_eq!(stats.thread_count, 0);
        assert!(stats.plotlines.is_empty());
        assert!(stats.turning_points.is_empty());
        assert_eq!(stats.coherence_score, 100.0);
    }

    #[test]
    fn test_threads_are_collected_from_every_narrative_category() {
        let mut first = person("S1", &["Marlowe"]);
        first.entities.push(Entity {
            text: "NYPD".into(),
            category: EntityCategory::Organization,
            confidence: 0.9,
            source: "test".into(),
            keep: true,
            filter: None,
            filter_reason: None,
            span: (0, 0),
        });
        let doc = doc_of(vec![
            first,
            person("S2", &["Marlowe", "NYPD"]),
            person("S3", &["Marlowe", "NYPD"]),
        ]);
        let stats = run_plot_analysis(&doc).unwrap();
        let nypd = stats
            .plotlines
            .iter()
            .find(|p| p.subject == "NYPD")
            .unwrap();
        assert_eq!(nypd.kind, "ORGANIZATION");
    }

    #[test]
    fn test_repeated_runs_are_identical() {
        // Threads are collected out of a HashMap; the reported order must not depend
        // on that iteration order.
        let doc = doc_of(vec![
            person("S1", &["Marlowe", "Tobey", "Chase"]),
            person("S2", &["Marlowe", "Tobey"]),
            person("S3", &["Marlowe", "Chase"]),
            person("S4", &["Marlowe", "Tobey", "Chase"]),
            person("S5", &["Marlowe", "Tobey", "Chase"]),
        ]);
        let a = run_plot_analysis(&doc).unwrap();
        let b = run_plot_analysis(&doc).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }
}
