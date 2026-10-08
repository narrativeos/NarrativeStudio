//! Conflict analysis (冲突分析): what the story fights over, and how hard.
//!
//! The design doc lists this dimension as "full text plus entities → conflict
//! types and an intensity curve, LLM semantic analysis". The LLM half stays out of
//! scope; this module does the half a keyword pass can do honestly — and says what
//! it cannot see.
//!
//! A conflict is counted when the text uses conflict vocabulary. Four families are
//! tracked separately, because "the protagonist fights the villain" and "the
//! protagonist cannot decide" are different problems to fix:
//!
//! - `interpersonal` 人际: 冲突 / 战斗 / 争吵 / 威胁 …
//! - `internal` 内心: 挣扎 / 犹豫 / 恐惧 / 自责 …
//! - `societal` 社会: 压迫 / 反抗 / 审判 / 歧视 …
//! - `survival` 生存: 风暴 / 伤口 / 饥荒 / 绝境 …
//!
//! Two limits follow from that and are stated in the output rather than hidden:
//! every term counts the same (a 战斗 and a 争吵 are one hit each — the pass reads
//! words, not stakes), and a document that uses no conflict vocabulary yields
//! `verdict: "none"` with **no score**, because "the vocabulary is missing" is
//! either a non-narrative text or a gap in the lexicon, and grading it either way
//! would be a guess about the story.
//!
//! Deterministic: fixed lexicon, segments in reading order, no clock, no
//! randomness; the same document always yields the same curve and the same score.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;

use crate::segment::{plan_segments, SegmentPlan};
use crate::text::visible_char_count;
use crate::Result;

/// Conflict hits per 1000 visible characters at which a segment saturates. Picked
/// so a scene with roughly one conflict word per two sentences reads as full.
const CONFLICT_FULL: f64 = 8.0;
/// Below this overall density the document is called thin on conflict.
const THIN_DENSITY: f64 = 2.0;
/// Above this, heavy.
const HEAVY_DENSITY: f64 = 8.0;
/// A run of conflict-free segments is only reported from this many segments up.
const ABSENT_RUN_MIN: u32 = 3;
/// Below this many segments, "the peak is early" and "one segment holds everything"
/// carry no information, so neither is charged.
const MIN_SEGMENTS_FOR_SHAPE: usize = 5;
/// Below this many hits, the type mix says too little to charge for being narrow.
const MIN_HITS_FOR_TYPE_JUDGEMENT: u32 = 10;
/// How many top terms are reported per family.
const MAX_TERMS_PER_TYPE: usize = 10;

/// The four conflict families and their vocabulary. Order is stable: it fixes the
/// order of the type list, and it breaks intensity ties deterministically.
const CONFLICT_TYPES: [(&str, &[&str]); 4] = [
    (
        "interpersonal",
        &[
            "冲突", "对抗", "战斗", "打斗", "打架", "攻击", "袭击", "进攻", "反击", "厮杀", "搏斗",
            "决斗", "较量", "争吵", "争执", "吵架", "辱骂", "威胁", "恐吓", "敌对", "敌人", "仇恨",
            "怨恨", "复仇", "报复", "背叛", "欺骗", "陷害", "阴谋", "决裂", "翻脸", "追杀", "围剿",
            "战争", "开战", "交火", "对峙", "谈判", "官司", "诉讼", "指控",
        ],
    ),
    (
        "internal",
        &[
            "矛盾",
            "犹豫",
            "挣扎",
            "纠结",
            "两难",
            "左右为难",
            "进退两难",
            "抉择",
            "恐惧",
            "害怕",
            "畏惧",
            "焦虑",
            "担忧",
            "不安",
            "怀疑",
            "自责",
            "愧疚",
            "后悔",
            "悔恨",
            "煎熬",
            "折磨",
            "痛苦",
            "绝望",
            "崩溃",
            "动摇",
            "孤独",
            "迷茫",
            "困惑",
            "内心",
        ],
    ),
    (
        "societal",
        &[
            "压迫", "剥削", "歧视", "偏见", "不公", "反抗", "抗议", "起义", "暴动", "革命", "罢工",
            "游行", "镇压", "逮捕", "监禁", "囚禁", "流放", "驱逐", "审判", "犯罪", "罪行", "阶级",
            "排挤", "孤立", "羞辱", "嘲笑", "蔑视",
        ],
    ),
    (
        "survival",
        &[
            "风暴", "暴雨", "洪水", "火灾", "地震", "雪崩", "山崩", "饥荒", "瘟疫", "疾病", "中毒",
            "溺水", "受伤", "伤口", "流血", "饥饿", "严寒", "寒冷", "酷暑", "疲惫", "坠落", "悬崖",
            "绝境", "困境", "危险", "危机", "野兽", "猛兽", "荒野", "沙漠", "窒息", "生存", "求生",
            "逃命", "逃生",
        ],
    ),
];

/// One segment of the conflict curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictPoint {
    pub section: String,
    pub char_count: u64,
    /// Conflict vocabulary hits in the segment.
    pub hits: u32,
    /// Hits per 1000 visible characters.
    pub density: f64,
    /// 0-100 on the absolute scale, so two projects' curves can be compared.
    pub intensity: f64,
    /// The family with most hits here; empty when the segment has none.
    pub dominant: String,
    /// Per-family hits, `(kind, hits)`; families with no hits are omitted.
    pub by_type: Vec<(String, u32)>,
}

/// One conflict family as it appears in the whole document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictType {
    /// `interpersonal` | `internal` | `societal` | `survival`.
    pub kind: String,
    pub hits: u32,
    /// Share of all conflict hits.
    pub share: f64,
    /// Hits per 1000 visible characters across the document.
    pub density: f64,
    pub top_terms: Vec<(String, u32)>,
}

/// A run of consecutive segments carrying no conflict vocabulary at all.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictAbsence {
    pub from_section: String,
    pub to_section: String,
    pub sections: u32,
    pub char_count: u64,
}

/// Conflict analysis output for one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictStats {
    pub points: Vec<ConflictPoint>,
    pub segment_count: u32,
    /// True when the curve came from block chunks because the document has too few
    /// sections to cut by.
    pub chunked: bool,

    pub total_hits: u32,
    /// Hits per 1000 visible characters across the document.
    pub density: f64,
    /// Families present, most hits first.
    pub types: Vec<ConflictType>,
    /// The busiest family; empty when nothing was found.
    pub dominant_type: String,
    /// `none` | `thin` | `present` | `heavy`.
    pub verdict: String,

    /// Where the curve is highest, and where that sits in the document (0-1).
    pub peak_section: String,
    pub peak_intensity: f64,
    pub peak_position: f64,
    /// Mean intensity over the final third of the segments: whether whatever the
    /// story built up is still standing at the end.
    pub tail_intensity: f64,
    /// Share of all hits held by the single busiest segment.
    pub concentration: f64,
    pub absent_runs: Vec<ConflictAbsence>,
    pub longest_absent_run: u32,

    /// 0-100, or `None` when no conflict vocabulary was found at all — see
    /// [`conflict_score`] for exactly what it charges.
    pub conflict_score: Option<f64>,
}

/// Raw per-segment counters: one hit count per family, in `CONFLICT_TYPES` order.
#[derive(Debug, Default)]
struct Segment {
    label: String,
    chars: u64,
    hits: [u32; 4],
}

/// Count the conflict vocabulary of a document, segment by segment.
pub fn run_conflict_analysis(doc: &DocumentData) -> Result<ConflictStats> {
    let plan = plan_segments(doc);
    let (segments, term_totals) = count_segments(doc, &plan);

    let total_chars: u64 = segments.iter().map(|s| s.chars).sum();
    let total_hits: u32 = segments.iter().map(|s| s.hits.iter().sum::<u32>()).sum();

    let points: Vec<ConflictPoint> = segments
        .iter()
        .map(|s| {
            let chars = s.chars.max(1) as f64;
            let hits: u32 = s.hits.iter().sum();
            let density = hits as f64 / chars * 1000.0;
            ConflictPoint {
                section: s.label.clone(),
                char_count: s.chars,
                hits,
                density,
                intensity: (density / CONFLICT_FULL).min(1.0) * 100.0,
                dominant: dominant_family(&s.hits).to_string(),
                by_type: CONFLICT_TYPES
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| s.hits[*i] > 0)
                    .map(|(i, t)| (t.0.to_string(), s.hits[i]))
                    .collect(),
            }
        })
        .collect();

    let types = family_totals(&segments, &term_totals, total_hits, total_chars);
    let (peak_section, peak_intensity, peak_position) = peak(&points);
    let (absent_runs, longest_absent_run) = find_absent_runs(&points);
    let tail_intensity = tail_mean(&points);
    let density = if total_chars == 0 {
        0.0
    } else {
        total_hits as f64 / total_chars as f64 * 1000.0
    };
    let concentration = if total_hits == 0 {
        0.0
    } else {
        points.iter().map(|p| p.hits).max().unwrap_or(0) as f64 / total_hits as f64
    };

    let mut stats = ConflictStats {
        points,
        segment_count: segments.len() as u32,
        chunked: plan.chunked,
        total_hits,
        density,
        dominant_type: types.first().map(|t| t.kind.clone()).unwrap_or_default(),
        verdict: verdict(density).to_string(),
        types,
        peak_section,
        peak_intensity,
        peak_position,
        tail_intensity,
        concentration,
        absent_runs,
        longest_absent_run,
        conflict_score: None,
    };
    // The score reads the stats rather than a long parameter list: every number it
    // charges for is one the caller can see in the output.
    stats.conflict_score = conflict_score(&stats);
    Ok(stats)
}

/// Total the vocabulary per segment, and per term across the document.
///
/// Terms are matched as substrings: Chinese has no word boundaries, and the
/// vocabulary is made of multi-character terms so a single common character cannot
/// register as a conflict.
fn count_segments(
    doc: &DocumentData,
    plan: &SegmentPlan,
) -> (Vec<Segment>, [Vec<(String, u32)>; 4]) {
    if plan.labels.is_empty() {
        return (Vec::new(), std::array::from_fn(|_| Vec::new()));
    }

    let mut segments: Vec<Segment> = plan
        .labels
        .iter()
        .map(|label| Segment {
            label: label.clone(),
            ..Default::default()
        })
        .collect();
    let mut term_hits: [HashMap<String, u32>; 4] = std::array::from_fn(|_| HashMap::new());

    for (idx, block) in doc.blocks.iter().enumerate() {
        let s = &mut segments[plan.segment_of[idx]];
        s.chars += visible_char_count(&block.content) as u64;
        for (fi, (_, terms)) in CONFLICT_TYPES.iter().enumerate() {
            for term in *terms {
                let hits = block.content.match_indices(term).count() as u32;
                if hits == 0 {
                    continue;
                }
                s.hits[fi] += hits;
                *term_hits[fi].entry((*term).to_string()).or_insert(0) += hits;
            }
        }
    }

    let term_totals = std::array::from_fn(|fi| {
        let mut list: Vec<(String, u32)> = term_hits[fi].drain().collect();
        // Count first, term second: HashMap order must not reach the report.
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    });

    (segments, term_totals)
}

/// The family with most hits in a segment; ties fall to the lexicon order.
fn dominant_family(hits: &[u32; 4]) -> &'static str {
    let mut best: Option<(u32, &'static str)> = None;
    for (fi, (kind, _)) in CONFLICT_TYPES.iter().enumerate() {
        if hits[fi] > best.map(|(count, _)| count).unwrap_or(0) {
            best = Some((hits[fi], kind));
        }
    }
    best.map(|(_, kind)| kind).unwrap_or("")
}

/// Per-family totals for the whole document, busiest first, zero families omitted.
fn family_totals(
    segments: &[Segment],
    term_totals: &[Vec<(String, u32)>; 4],
    total_hits: u32,
    total_chars: u64,
) -> Vec<ConflictType> {
    let mut types: Vec<ConflictType> = CONFLICT_TYPES
        .iter()
        .enumerate()
        .filter_map(|(fi, (kind, _))| {
            let hits: u32 = segments.iter().map(|s| s.hits[fi]).sum();
            if hits == 0 {
                return None;
            }
            Some(ConflictType {
                kind: (*kind).to_string(),
                hits,
                share: hits as f64 / total_hits.max(1) as f64,
                density: if total_chars == 0 {
                    0.0
                } else {
                    hits as f64 / total_chars as f64 * 1000.0
                },
                top_terms: term_totals[fi]
                    .iter()
                    .take(MAX_TERMS_PER_TYPE)
                    .cloned()
                    .collect(),
            })
        })
        .collect();

    types.sort_by(|a, b| b.hits.cmp(&a.hits).then_with(|| a.kind.cmp(&b.kind)));
    types
}

/// The highest point on the curve and where it sits in the document (0-1).
/// Ties go to the earliest segment: a story that peaks twice is charged for the
/// first peak, which is the one the rest of the book has to live up to.
fn peak(points: &[ConflictPoint]) -> (String, f64, f64) {
    let mut best: Option<(usize, f64, String)> = None;
    for (i, p) in points.iter().enumerate() {
        if p.intensity > best.as_ref().map(|(_, v, _)| *v).unwrap_or(-1.0) {
            best = Some((i, p.intensity, p.section.clone()));
        }
    }
    match best {
        None => (String::new(), 0.0, 0.0),
        Some((i, intensity, section)) => {
            let position = if points.len() <= 1 {
                0.0
            } else {
                i as f64 / (points.len() - 1) as f64
            };
            (section, intensity, position)
        }
    }
}

/// Mean intensity over the final third of the segments (at least one segment).
fn tail_mean(points: &[ConflictPoint]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let take = ((points.len() as f64 / 3.0).ceil() as usize).max(1);
    let start = points.len() - take;
    points[start..].iter().map(|p| p.intensity).sum::<f64>() / take as f64
}

/// Runs of consecutive segments with no conflict vocabulary at all.
fn find_absent_runs(points: &[ConflictPoint]) -> (Vec<ConflictAbsence>, u32) {
    let mut runs: Vec<ConflictAbsence> = Vec::new();
    let mut longest = 0u32;
    let mut start = 0usize;

    for end in 0..=points.len() {
        let break_run = end == points.len() || points[end].hits > 0;
        if !break_run {
            continue;
        }
        let run = (end - start) as u32;
        longest = longest.max(run);
        if run >= ABSENT_RUN_MIN {
            let slice = &points[start..end];
            runs.push(ConflictAbsence {
                from_section: slice[0].section.clone(),
                to_section: slice[slice.len() - 1].section.clone(),
                sections: run,
                char_count: slice.iter().map(|p| p.char_count).sum(),
            });
        }
        // The conflict-bearing segment starts the next stretch of silence.
        start = end + 1;
    }

    (runs, longest)
}

/// How much conflict vocabulary the document carries overall.
fn verdict(density: f64) -> &'static str {
    if density <= 0.0 {
        "none"
    } else if density < THIN_DENSITY {
        "thin"
    } else if density > HEAVY_DENSITY {
        "heavy"
    } else {
        "present"
    }
}
/// 0-100, or `None` when the text carries no conflict vocabulary at all.
///
/// Starts at 100 and charges for what a vocabulary pass can actually see:
///
/// - thin coverage: up to −30, scaling with how far below `THIN_DENSITY` the
///   document sits;
/// - one family carrying everything: up to −15, once there are enough hits for the
///   mix to mean something — a story fought on one plane only is a flatter story;
/// - long conflict-free stretches: −5 per segment past `ABSENT_RUN_MIN`, up to −20;
/// - a peak in the first third while the last third runs at less than half of it:
///   −20, because the fight was won before the reader was invested in it;
/// - one segment holding most of the conflict: up to −15 — the same fight repeated
///   instead of escalated.
///
/// The last two need at least `MIN_SEGMENTS_FOR_SHAPE` segments to mean anything,
/// and the type mix needs `MIN_HITS_FOR_TYPE_JUDGEMENT` hits.
fn conflict_score(stats: &ConflictStats) -> Option<f64> {
    if stats.total_hits == 0 {
        return None;
    }

    let thin = if stats.density >= THIN_DENSITY {
        0.0
    } else {
        (THIN_DENSITY - stats.density) / THIN_DENSITY * 30.0
    };
    let dominant_share = stats.types.first().map(|t| t.share).unwrap_or(0.0);
    let narrow = if stats.total_hits >= MIN_HITS_FOR_TYPE_JUDGEMENT {
        ((dominant_share - 0.8) / 0.2 * 15.0).clamp(0.0, 15.0)
    } else {
        0.0
    };
    let absent = if stats.longest_absent_run >= ABSENT_RUN_MIN {
        (((stats.longest_absent_run - ABSENT_RUN_MIN) as f64 + 1.0) * 5.0).min(20.0)
    } else {
        0.0
    };

    let shaped = stats.segment_count as usize >= MIN_SEGMENTS_FOR_SHAPE;
    let early = if shaped
        && stats.peak_position < 1.0 / 3.0
        && stats.tail_intensity < stats.peak_intensity * 0.5
    {
        20.0
    } else {
        0.0
    };
    let lumpy = if shaped && stats.concentration > 0.5 {
        ((stats.concentration - 0.5) * 60.0).min(15.0)
    } else {
        0.0
    };

    Some((100.0 - thin - narrow - absent - early - lumpy).clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

    /// No conflict vocabulary at all: the floor of the curve.
    const CALM: &str =
        "下午的编辑室里很安静，桌上放着昨天的稿子和一杯凉掉的茶。窗外下着小雨，没有人说话。";
    /// Physical conflict: 攻击 / 反击 / 战斗 — three interpersonal hits.
    const FIGHT: &str = "他举起刀发起攻击，她侧身躲开并反击。两人扭打在一起，战斗持续了一整夜。";
    /// 内心 conflict: 内心 / 矛盾 / 恐惧 / 崩溃.
    const DILEMMA: &str = "他反复问自己该不该说出真相，内心充满矛盾与恐惧，几乎崩溃。";
    /// Societal conflict: 抗议 / 压迫 / 镇压 / 逮捕.
    const RIOT: &str = "广场上的人群举起标语，抗议当局的压迫，警察随即镇压并逮捕了带头的人。";
    /// Survival conflict: 风暴 / 洪水 / 饥饿 / 寒冷 / 疲惫.
    const STORM: &str =
        "风暴摧毁了堤坝，洪水冲进村子。他抱着孩子爬上天台，饥饿和寒冷让人疲惫不堪。";

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

    fn sections(texts: &[(&str, &str)]) -> DocumentData {
        doc_of(texts.iter().map(|(s, t)| para(s, t)).collect())
    }

    #[test]
    fn test_conflict_vocabulary_is_counted() {
        let stats = run_conflict_analysis(&sections(&[("S1", CALM), ("S2", FIGHT), ("S3", CALM)]))
            .expect("conflict analysis");

        assert_eq!(stats.segment_count, 3);
        assert_eq!(stats.total_hits, 3);
        assert_eq!(stats.points[0].hits, 0);
        assert_eq!(stats.points[1].hits, 3);
        assert_eq!(stats.points[1].dominant, "interpersonal");
        assert_eq!(
            stats.points[1].by_type,
            vec![("interpersonal".to_string(), 3)]
        );
        assert_eq!(stats.types.len(), 1);
        assert_eq!(stats.types[0].kind, "interpersonal");
        assert!((stats.types[0].share - 1.0).abs() < 1e-9);
        let terms: Vec<&str> = stats.types[0]
            .top_terms
            .iter()
            .map(|(t, _)| t.as_str())
            .collect();
        assert!(terms.contains(&"战斗"));
        assert!(terms.contains(&"攻击"));
        assert_eq!(stats.dominant_type, "interpersonal");
        assert_ne!(stats.verdict, "none");
    }

    #[test]
    fn test_families_are_kept_apart() {
        let stats =
            run_conflict_analysis(&sections(&[("S1", DILEMMA), ("S2", RIOT), ("S3", STORM)]))
                .expect("conflict analysis");

        assert_eq!(stats.total_hits, 13);
        // Busiest first; the 4-4 tie between 内心 and 社会 falls to the kind name,
        // so the report order never depends on HashMap order.
        let kinds: Vec<&str> = stats.types.iter().map(|t| t.kind.as_str()).collect();
        assert_eq!(kinds, ["survival", "internal", "societal"]);
        assert_eq!(stats.dominant_type, "survival");
        assert_eq!(stats.points[0].dominant, "internal");
        assert_eq!(stats.points[2].dominant, "survival");

        let share_sum: f64 = stats.types.iter().map(|t| t.share).sum();
        assert!((share_sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_a_document_without_conflict_vocabulary_is_not_scored() {
        let doc = doc_of(
            ["S1", "S2", "S3", "S4", "S5"]
                .iter()
                .map(|s| para(s, CALM))
                .collect(),
        );
        let stats = run_conflict_analysis(&doc).expect("conflict analysis");

        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.verdict, "none");
        // "The vocabulary is missing" is not a grade: it is either a non-narrative
        // text or a gap in the lexicon.
        assert_eq!(stats.conflict_score, None);
        assert!(stats.types.is_empty());
        assert!(stats.dominant_type.is_empty());
        assert_eq!(stats.longest_absent_run, 5);
        assert_eq!(stats.absent_runs.len(), 1);
        assert_eq!(stats.absent_runs[0].from_section, "S1");
        assert_eq!(stats.absent_runs[0].to_section, "S5");
    }

    #[test]
    fn test_intensity_is_absolute_not_relative_to_the_document() {
        let short = run_conflict_analysis(&sections(&[("S1", FIGHT), ("S2", CALM), ("S3", CALM)]))
            .expect("conflict analysis");
        let long = run_conflict_analysis(&doc_of(
            std::iter::once(para("S1", FIGHT))
                .chain((2..=10).map(|i| para(&format!("S{i}"), CALM)))
                .collect(),
        ))
        .expect("conflict analysis");

        // The same paragraph reads equally intense whatever surrounds it, which is
        // what lets two projects' curves be compared; the document-level density is
        // where the rest of the book shows up.
        assert!(short.points[0].intensity > 0.0);
        assert_eq!(short.points[0].intensity, long.points[0].intensity);
        assert!(short.density > long.density);
    }
    #[test]
    fn test_a_peak_only_at_the_start_is_charged() {
        let early = run_conflict_analysis(&doc_of(
            [("S1", FIGHT), ("S2", FIGHT)]
                .iter()
                .chain(
                    [
                        ("S3", CALM),
                        ("S4", CALM),
                        ("S5", CALM),
                        ("S6", CALM),
                        ("S7", CALM),
                        ("S8", CALM),
                    ]
                    .iter(),
                )
                .map(|(s, t)| para(s, t))
                .collect(),
        ))
        .expect("conflict analysis");
        let late = run_conflict_analysis(&doc_of(
            [
                ("S1", CALM),
                ("S2", CALM),
                ("S3", CALM),
                ("S4", CALM),
                ("S5", CALM),
                ("S6", CALM),
            ]
            .iter()
            .chain([("S7", FIGHT), ("S8", FIGHT)].iter())
            .map(|(s, t)| para(s, t))
            .collect(),
        ))
        .expect("conflict analysis");

        // Both books spend the same words on the same fight; one wins it before the
        // reader has arrived, and its last third runs at nothing.
        assert_eq!(early.peak_section, "S1");
        assert_eq!(early.peak_position, 0.0);
        assert_eq!(early.tail_intensity, 0.0);
        assert!(late.peak_position > 0.8);
        assert!(late.tail_intensity > 0.0);

        let early_score = early.conflict_score.expect("early scored");
        let late_score = late.conflict_score.expect("late scored");
        assert!(late_score > early_score);
        assert!(late_score - early_score >= 15.0);
    }

    #[test]
    fn test_a_conflict_free_stretch_is_reported() {
        let stats = run_conflict_analysis(&doc_of(
            [("S1", FIGHT)]
                .iter()
                .chain(
                    [
                        ("S2", CALM),
                        ("S3", CALM),
                        ("S4", CALM),
                        ("S5", CALM),
                        ("S6", CALM),
                    ]
                    .iter(),
                )
                .chain([("S7", FIGHT)].iter())
                .map(|(s, t)| para(s, t))
                .collect(),
        ))
        .expect("conflict analysis");

        assert_eq!(stats.longest_absent_run, 5);
        assert_eq!(stats.absent_runs.len(), 1);
        assert_eq!(stats.absent_runs[0].from_section, "S2");
        assert_eq!(stats.absent_runs[0].to_section, "S6");
        assert_eq!(stats.absent_runs[0].sections, 5);
    }

    #[test]
    fn test_one_segment_holding_everything_is_measured() {
        let lumpy = run_conflict_analysis(&doc_of(
            [("S1", FIGHT)]
                .iter()
                .chain(
                    [
                        ("S2", CALM),
                        ("S3", CALM),
                        ("S4", CALM),
                        ("S5", CALM),
                        ("S6", CALM),
                    ]
                    .iter(),
                )
                .map(|(s, t)| para(s, t))
                .collect(),
        ))
        .expect("conflict analysis");
        let spread = run_conflict_analysis(&doc_of(
            (1..=6).map(|i| para(&format!("S{i}"), FIGHT)).collect(),
        ))
        .expect("conflict analysis");

        assert!((lumpy.concentration - 1.0).abs() < 1e-9);
        assert!((spread.concentration - 1.0 / 6.0).abs() < 1e-9);
        assert!(spread.absent_runs.is_empty());
        assert!(lumpy.conflict_score.expect("scored") < 100.0);
    }

    #[test]
    fn test_empty_document_has_no_curve() {
        let stats = run_conflict_analysis(&doc_of(vec![])).expect("conflict analysis");

        assert_eq!(stats.segment_count, 0);
        assert!(stats.points.is_empty());
        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.verdict, "none");
        assert_eq!(stats.conflict_score, None);
        assert!(stats.peak_section.is_empty());
        assert_eq!(stats.peak_intensity, 0.0);
    }

    #[test]
    fn test_same_document_yields_identical_output() {
        let doc = sections(&[
            ("S1", FIGHT),
            ("S2", DILEMMA),
            ("S3", RIOT),
            ("S3 / b", STORM),
        ]);
        let a = serde_json::to_string(&run_conflict_analysis(&doc).unwrap()).unwrap();
        let b = serde_json::to_string(&run_conflict_analysis(&doc).unwrap()).unwrap();
        assert_eq!(a, b);
    }
}
