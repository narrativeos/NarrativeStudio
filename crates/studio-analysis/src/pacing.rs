//! Pacing analysis (节奏分析): how far the tension moves, and whether that
//! movement holds the reader.
//!
//! The T1 narrative arc answers *where* intensity sits section by section. It
//! cannot answer the pacing question, which is whether that intensity ever
//! *changes*: a document parked at one level for thirty sections reads flat
//! whatever the arc's shape label says, and one that never lets the reader down
//! between peaks reads exhausting. This module measures the variation and scores
//! it.
//!
//! Tension uses the same four signals the arc uses — dialogue share,
//! short-sentence ratio, exclamatory density, entity density — but pinned to
//! fixed reference points instead of min-max normalized across the document.
//! Min-max always produces a 0 and a 1 somewhere, so a flat document still looks
//! varied and one project's curve cannot be read against another's. Fixed
//! references cost a little contrast inside a single project and buy the
//! comparability the pacing score and the comparison view need.
//!
//! Deterministic: no clock, no randomness, segments in reading order, and the
//! same document always yields the same points and the same score.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;

use crate::t1::top_level_section;
use crate::text::{count_quoted_chars, split_sentences, visible_char_count};
use crate::Result;

/// Below this many top-level sections the document is chunked by block instead,
/// and the pacing score declines to judge: with too few segments, "flat" and
/// "varied" say the same thing about one long block.
const MIN_SECTIONS: usize = 3;
/// Segments used when the document has no usable section structure.
const FALLBACK_SEGMENTS: usize = 10;
/// A sentence at or under this many visible characters counts as short (the same
/// cut the arc uses, so both modules mean the same thing by it).
const SHORT_SENTENCE_CHARS: usize = 12;
/// Dialogue share at which the dialogue signal saturates.
const DIALOGUE_FULL: f64 = 0.5;
/// Exclamatory/question marks per 1000 characters at which the signal saturates.
const PUNCT_FULL: f64 = 8.0;
/// Entities per 1000 characters at which the signal saturates.
const ENTITY_FULL: f64 = 12.0;
/// A segment this many times the mean segment length while sitting below the mean
/// tension is a stretch that drags.
const DRAG_LENGTH_RATIO: f64 = 1.8;
/// Tension points a segment may move and still count as the same flat stretch.
const FLAT_BAND: f64 = 8.0;
/// A flat stretch is only worth reporting from this many segments up.
const FLAT_RUN_MIN: u32 = 4;
/// Below this tension stdev the curve is called flat; above `SPIKY_STDEV`, spiky.
const FLAT_STDEV: f64 = 5.0;
const SPIKY_STDEV: f64 = 22.0;
/// Sentence-length stdev (chars) under which the prose itself is monotonous.
const SENTENCE_VARIETY_FLOOR: f64 = 6.0;

/// One segment of the pacing curve: what the section is made of and how tense it
/// reads on the absolute 0-100 scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacingPoint {
    pub section: String,
    pub char_count: u64,
    pub sentence_count: u32,
    pub avg_sentence_chars: f64,
    /// 句长变化: stdev of sentence lengths inside the segment (chars).
    pub sentence_stdev: f64,
    /// Fraction of the segment's visible characters inside quotation marks.
    pub dialogue_share: f64,
    /// Entities per 1000 visible characters.
    pub entity_density: f64,
    /// 0-100 on the absolute scale.
    pub tension: f64,
}

/// A run of consecutive segments whose tension stays inside one `FLAT_BAND`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatStretch {
    pub from_section: String,
    pub to_section: String,
    pub sections: u32,
    pub mean_tension: f64,
    pub char_count: u64,
}

/// A segment much longer than the average while less tense than the average.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragSection {
    pub section: String,
    pub char_count: u64,
    pub tension: f64,
    /// Segment length over the mean segment length.
    pub length_ratio: f64,
}

/// Pacing analysis output for one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacingStats {
    pub points: Vec<PacingPoint>,
    pub segment_count: u32,
    /// True when the document had fewer than `MIN_SECTIONS` sections and the
    /// curve was built from block chunks instead.
    pub chunked: bool,
    /// `flat` | `varied` | `spiky` — how much the tension moves.
    pub verdict: String,
    /// Standard deviation of tension across segments (0-100 scale).
    pub tension_stdev: f64,
    /// 句长变化 across the whole document (chars).
    pub sentence_stdev: f64,
    pub min_tension: f64,
    pub max_tension: f64,
    pub mean_section_chars: f64,
    /// Longest run of segments inside one `FLAT_BAND`.
    pub longest_flat_run: u32,
    pub flat_stretches: Vec<FlatStretch>,
    pub drag_sections: Vec<DragSection>,
    /// 0-100; [`pace_score`] states exactly what it penalises.
    pub pace_score: f64,
}

/// Raw per-segment counters the tension is computed from.
#[derive(Debug, Default)]
struct Segment {
    label: String,
    chars: u64,
    quoted: u64,
    excl: u64,
    sentences: u64,
    short: u64,
    entities: u64,
    lengths: Vec<u64>,
}

/// Measure how the document's tension moves, segment by segment.
pub fn run_pacing_analysis(doc: &DocumentData) -> Result<PacingStats> {
    let (segments, chunked) = collect_segments(doc);
    if segments.is_empty() {
        return Ok(PacingStats {
            points: Vec::new(),
            segment_count: 0,
            chunked: false,
            verdict: "flat".into(),
            tension_stdev: 0.0,
            sentence_stdev: 0.0,
            min_tension: 0.0,
            max_tension: 0.0,
            mean_section_chars: 0.0,
            longest_flat_run: 0,
            flat_stretches: Vec::new(),
            drag_sections: Vec::new(),
            pace_score: 100.0,
        });
    }

    let all_lengths: Vec<u64> = segments.iter().flat_map(|s| s.lengths.clone()).collect();
    let sentence_stdev = stdev(&all_lengths);

    let points: Vec<PacingPoint> = segments
        .iter()
        .map(|s| {
            let chars = s.chars.max(1) as f64;
            let dialogue_share = s.quoted as f64 / chars;
            let short_ratio = if s.sentences > 0 {
                s.short as f64 / s.sentences as f64
            } else {
                0.0
            };
            let punct_density = s.excl as f64 / chars * 1000.0;
            let entity_density = s.entities as f64 / chars * 1000.0;
            PacingPoint {
                section: s.label.clone(),
                char_count: s.chars,
                sentence_count: s.sentences as u32,
                avg_sentence_chars: if s.sentences > 0 {
                    s.lengths.iter().sum::<u64>() as f64 / s.sentences as f64
                } else {
                    0.0
                },
                sentence_stdev: stdev(&s.lengths),
                dialogue_share,
                entity_density,
                tension: tension_of(dialogue_share, short_ratio, punct_density, entity_density),
            }
        })
        .collect();

    let tensions: Vec<f64> = points.iter().map(|p| p.tension).collect();
    let tension_stdev = stdev_f64(&tensions);
    let mean_section_chars =
        points.iter().map(|p| p.char_count).sum::<u64>() as f64 / points.len() as f64;
    let (flat_stretches, longest_flat_run) = find_flat_stretches(&points);
    let drag_sections = find_drag(&points, mean_section_chars);
    let pace_score = pace_score(
        chunked,
        tension_stdev,
        longest_flat_run,
        drag_sections.len(),
        sentence_stdev,
    );

    Ok(PacingStats {
        points,
        segment_count: segments.len() as u32,
        chunked,
        verdict: verdict(tension_stdev).to_string(),
        tension_stdev,
        sentence_stdev,
        min_tension: tensions.iter().cloned().fold(f64::INFINITY, f64::min),
        max_tension: tensions.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        mean_section_chars,
        longest_flat_run,
        flat_stretches,
        drag_sections,
        pace_score,
    })
}

/// Group the blocks into curve segments: top-level sections when there are at
/// least `MIN_SECTIONS` of them, otherwise `FALLBACK_SEGMENTS` equal block
/// chunks. This is the same segmentation the arc uses, so the two curves line up
/// on the same X axis.
fn collect_segments(doc: &DocumentData) -> (Vec<Segment>, bool) {
    if doc.blocks.is_empty() {
        return (Vec::new(), false);
    }

    let mut order: Vec<String> = Vec::new();
    let mut index_of: HashMap<String, usize> = HashMap::new();
    for block in &doc.blocks {
        let top = top_level_section(&block.section_path);
        if !index_of.contains_key(&top) {
            index_of.insert(top.clone(), order.len());
            order.push(top);
        }
    }

    let use_sections = order.len() >= MIN_SECTIONS;
    let count = if use_sections {
        order.len()
    } else {
        FALLBACK_SEGMENTS
    };
    let chunk_size = (doc.blocks.len() as f64 / count as f64).ceil() as usize;

    let mut segments: Vec<Segment> = (0..count)
        .map(|i| Segment {
            label: if use_sections {
                order[i].clone()
            } else {
                format!("{}/{}", i + 1, count)
            },
            ..Default::default()
        })
        .collect();

    for (idx, block) in doc.blocks.iter().enumerate() {
        let seg = if use_sections {
            *index_of
                .get(&top_level_section(&block.section_path))
                .expect("section was registered above")
        } else {
            (idx / chunk_size.max(1)).min(count - 1)
        };
        let s = &mut segments[seg];
        s.chars += visible_char_count(&block.content) as u64;
        s.quoted += count_quoted_chars(&block.content) as u64;
        s.excl += block
            .content
            .chars()
            .filter(|c| matches!(c, '！' | '!' | '？' | '?'))
            .count() as u64;
        for sentence in split_sentences(&block.content) {
            let len = visible_char_count(&sentence) as u64;
            if len > 0 {
                s.sentences += 1;
                s.lengths.push(len);
                if len <= SHORT_SENTENCE_CHARS as u64 {
                    s.short += 1;
                }
            }
        }
        s.entities += block.entities.len() as u64;
    }

    (segments, !use_sections)
}

/// Tension on the absolute scale: each signal saturates at its own reference
/// point, so the same number means the same thing in two documents.
fn tension_of(
    dialogue_share: f64,
    short_ratio: f64,
    punct_density: f64,
    entity_density: f64,
) -> f64 {
    const W_DIALOGUE: f64 = 0.35;
    const W_SHORT: f64 = 0.30;
    const W_PUNCT: f64 = 0.20;
    const W_ENTITY: f64 = 0.15;

    let dialogue = (dialogue_share / DIALOGUE_FULL).min(1.0);
    let short = short_ratio.min(1.0);
    let punct = (punct_density / PUNCT_FULL).min(1.0);
    let entity = (entity_density / ENTITY_FULL).min(1.0);

    (100.0 * (W_DIALOGUE * dialogue + W_SHORT * short + W_PUNCT * punct + W_ENTITY * entity))
        .clamp(0.0, 100.0)
}

/// How much the curve moves. Deliberately not the arc's shape: the arc says
/// where the peak is, this says whether there is one.
fn verdict(tension_stdev: f64) -> &'static str {
    if tension_stdev < FLAT_STDEV {
        "flat"
    } else if tension_stdev > SPIKY_STDEV {
        "spiky"
    } else {
        "varied"
    }
}

/// The longest runs of segments whose whole range fits inside one `FLAT_BAND`.
///
/// The band is applied to the run as a whole rather than between neighbours, so a
/// curve that drifts one point at a time across twenty segments still counts as
/// one stretch — which is what the reader experiences.
fn find_flat_stretches(points: &[PacingPoint]) -> (Vec<FlatStretch>, u32) {
    let mut stretches: Vec<FlatStretch> = Vec::new();
    let mut longest = 0u32;
    let mut start = 0usize;

    for end in 0..=points.len() {
        let break_run = end == points.len() || {
            let slice = &points[start..end];
            let min = slice
                .iter()
                .map(|p| p.tension)
                .fold(f64::INFINITY, f64::min);
            let max = slice
                .iter()
                .map(|p| p.tension)
                .fold(f64::NEG_INFINITY, f64::max);
            max - min > FLAT_BAND
        };
        if !break_run {
            continue;
        }

        let run = (end - start) as u32;
        longest = longest.max(run);
        if run >= FLAT_RUN_MIN {
            let slice = &points[start..end];
            stretches.push(FlatStretch {
                from_section: slice[0].section.clone(),
                to_section: slice[slice.len() - 1].section.clone(),
                sections: run,
                mean_tension: slice.iter().map(|p| p.tension).sum::<f64>() / run as f64,
                char_count: slice.iter().map(|p| p.char_count).sum(),
            });
        }
        // The segment that broke the band starts the next run, so a long curve is
        // reported as consecutive stretches instead of losing the tail.
        start = end;
    }

    (stretches, longest)
}

/// Segments far longer than the average that are also less tense than the
/// average: the length a reader notices as dragging.
fn find_drag(points: &[PacingPoint], mean_chars: f64) -> Vec<DragSection> {
    if points.is_empty() || mean_chars <= 0.0 {
        return Vec::new();
    }
    let mean_tension = points.iter().map(|p| p.tension).sum::<f64>() / points.len() as f64;

    let mut drag: Vec<DragSection> = points
        .iter()
        .filter(|p| {
            p.char_count as f64 >= mean_chars * DRAG_LENGTH_RATIO && p.tension < mean_tension
        })
        .map(|p| DragSection {
            section: p.section.clone(),
            char_count: p.char_count,
            tension: p.tension,
            length_ratio: p.char_count as f64 / mean_chars,
        })
        .collect();

    drag.sort_by(|a, b| {
        b.length_ratio
            .total_cmp(&a.length_ratio)
            .then_with(|| a.section.cmp(&b.section))
    });
    drag.truncate(10);
    drag
}

/// 0-100. Starts at 100 and subtracts for what a pacing curve can actually see:
/// a curve that does not move, long stretches that do not move, chapters that are
/// long while sitting below the mean tension, and sentence lengths that never
/// vary. Documents without real section structure are not scored, matching how
/// the plot coherence and the structure balance treat them.
fn pace_score(
    chunked: bool,
    tension_stdev: f64,
    longest_flat_run: u32,
    drag_count: usize,
    sentence_stdev: f64,
) -> f64 {
    if chunked {
        return 100.0;
    }

    let monotony = if tension_stdev >= FLAT_STDEV {
        0.0
    } else {
        (FLAT_STDEV - tension_stdev) / FLAT_STDEV * 30.0
    };
    let stretch = if longest_flat_run >= FLAT_RUN_MIN {
        (((longest_flat_run - FLAT_RUN_MIN) as f64 + 1.0) * 6.0).min(24.0)
    } else {
        0.0
    };
    let drag = (drag_count as f64 * 8.0).min(24.0);
    let flat_prose = if sentence_stdev >= SENTENCE_VARIETY_FLOOR {
        0.0
    } else {
        (SENTENCE_VARIETY_FLOOR - sentence_stdev) / SENTENCE_VARIETY_FLOOR * 18.0
    };

    (100.0 - monotony - stretch - drag - flat_prose).clamp(0.0, 100.0)
}

/// Population standard deviation of integer lengths.
fn stdev(values: &[u64]) -> f64 {
    stdev_f64(&values.iter().map(|v| *v as f64).collect::<Vec<f64>>())
}

/// Population standard deviation; 0 for a single value, where spread is undefined.
fn stdev_f64(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance =
        values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

    /// Narration with no dialogue, no exclamations, no entities and no short
    /// sentences: the tension floor. Both sentences are 20 characters, so
    /// sentence variety is zero too.
    const PLAIN: &str =
        "这是一家成立三十年的出版社的年度总结报告。它完整记录了全年所有的编辑与印刷发行工作。";

    /// Dialogue, exclamations and short sentences: the tension ceiling of a
    /// paragraph written without entities.
    const TENSE: &str = "“快走！”他喊道。“你听见了吗？”门外传来一阵急促的脚步声！";

    /// One quiet line of dialogue in otherwise long sentences: a mild lift, so a
    /// curve can be varied without being spiky.
    const MILD: &str = "他在编辑室里翻着稿子，说：“这份稿子需要大改。”没有人接话，只有窗外的雨声。";

    fn para(path: &str, content: &str) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: content.into(),
            section_path: path.into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
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
    fn test_repeating_one_paragraph_is_flat() {
        let doc = doc_of(
            ["S1", "S2", "S3", "S4", "S5"]
                .iter()
                .map(|s| para(s, PLAIN))
                .collect(),
        );
        let stats = run_pacing_analysis(&doc).unwrap();

        assert_eq!(stats.segment_count, 5);
        assert!(!stats.chunked);
        assert!(stats.tension_stdev.abs() < 1e-9);
        assert_eq!(stats.verdict, "flat");
        assert_eq!(stats.longest_flat_run, 5);
        assert_eq!(stats.flat_stretches.len(), 1);
        assert_eq!(stats.flat_stretches[0].from_section, "S1");
        assert_eq!(stats.flat_stretches[0].to_section, "S5");
        assert_eq!(
            stats.flat_stretches[0].char_count,
            stats.points[0].char_count * 5
        );
        // Nothing drags when every segment is the same size, but a curve that
        // never moves and sentences that never vary are both charged.
        assert!(stats.drag_sections.is_empty());
        assert!((stats.pace_score - 40.0).abs() < 0.5);
    }

    #[test]
    fn test_dialogue_lifts_tension_on_an_absolute_scale() {
        let doc = doc_of(vec![
            para("S1", PLAIN),
            para("S2", TENSE),
            para("S3", PLAIN),
        ]);
        let stats = run_pacing_analysis(&doc).unwrap();

        assert_eq!(stats.points[0].tension, 0.0);
        assert!(stats.points[1].tension > 40.0);
        assert!(stats.points[1].dialogue_share > 0.25);
        assert_eq!(stats.points[0].dialogue_share, 0.0);
        assert_eq!(stats.min_tension, 0.0);
        assert!(stats.max_tension > 40.0);
    }

    #[test]
    fn test_alternating_curve_is_varied_and_scores_better() {
        let flat = run_pacing_analysis(&doc_of(
            ["S1", "S2", "S3", "S4", "S5"]
                .iter()
                .map(|s| para(s, PLAIN))
                .collect(),
        ))
        .unwrap();
        let varied = run_pacing_analysis(&doc_of(
            ["S1", "S2", "S3", "S4", "S5"]
                .iter()
                .map(|s| {
                    para(
                        s,
                        if s.ends_with('2') || s.ends_with('4') {
                            MILD
                        } else {
                            PLAIN
                        },
                    )
                })
                .collect(),
        ))
        .unwrap();

        assert_eq!(flat.verdict, "flat");
        assert_eq!(varied.verdict, "varied");
        assert!(varied.tension_stdev > FLAT_STDEV);
        assert!(varied.tension_stdev < SPIKY_STDEV);
        assert!(varied.flat_stretches.is_empty());
        assert!(varied.pace_score > flat.pace_score);
    }

    #[test]
    fn test_long_quiet_section_drags() {
        let mut blocks = vec![
            para("S1", TENSE),
            para("S2", TENSE),
            para("S3", TENSE),
            para("S4", TENSE),
        ];
        for _ in 0..4 {
            blocks.push(para("S5", PLAIN));
        }
        let stats = run_pacing_analysis(&doc_of(blocks)).unwrap();

        assert_eq!(stats.drag_sections.len(), 1);
        assert_eq!(stats.drag_sections[0].section, "S5");
        assert!(stats.drag_sections[0].length_ratio > 2.0);
    }

    #[test]
    fn test_document_without_sections_is_chunked_and_not_scored() {
        let doc = doc_of(vec![para("S1", TENSE), para("S2", PLAIN)]);
        let stats = run_pacing_analysis(&doc).unwrap();

        assert!(stats.chunked);
        assert_eq!(stats.segment_count, FALLBACK_SEGMENTS as u32);
        assert_eq!(stats.points[0].section, "1/10");
        assert_eq!(stats.pace_score, 100.0);
    }

    #[test]
    fn test_empty_document_is_neutral() {
        let stats = run_pacing_analysis(&doc_of(vec![])).unwrap();

        assert!(stats.points.is_empty());
        assert_eq!(stats.segment_count, 0);
        assert_eq!(stats.pace_score, 100.0);
    }

    #[test]
    fn test_sentence_variety_is_measured() {
        let same = run_pacing_analysis(&doc_of(
            ["S1", "S2", "S3"].iter().map(|s| para(s, PLAIN)).collect(),
        ))
        .unwrap();
        let mixed = run_pacing_analysis(&doc_of(
            ["S1", "S2", "S3"]
                .iter()
                .map(|s| {
                    para(
                        s,
                        "短句。这一句明显更长一些，因为它带了从句和修饰。还有一句中等长度。",
                    )
                })
                .collect(),
        ))
        .unwrap();

        assert_eq!(same.sentence_stdev, 0.0);
        assert!(mixed.sentence_stdev > 6.0);
    }

    #[test]
    fn test_same_document_yields_identical_output() {
        let doc = doc_of(vec![
            para("S1", TENSE),
            para("S2", PLAIN),
            para("S3", TENSE),
            para("S3 / b", PLAIN),
        ]);
        let a = serde_json::to_string(&run_pacing_analysis(&doc).unwrap()).unwrap();
        let b = serde_json::to_string(&run_pacing_analysis(&doc).unwrap()).unwrap();
        assert_eq!(a, b);
    }
}
