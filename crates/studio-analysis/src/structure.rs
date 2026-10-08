//! Story structure (故事结构分析): which shape the document's own landmarks fit.
//!
//! The design doc asks for 经典模型匹配 — matching the chapter tree against the
//! three-act, 起承转合 and hero's-journey shapes. This module does that, and it does
//! it the only way a deterministic pass can: it first finds where the document *itself*
//! says something happens, then asks what falls where.
//!
//! The landmarks come from signals already measured elsewhere, not from reading:
//! - a **turn** is a section boundary where the set of active threads changes sharply
//!   (the same definition 情节分析 reports as 转折点);
//! - a **peak** or **trough** is a local extreme of the pacing tension curve;
//! - the **climax** is the curve's highest point;
//! - the **opening** is simply where the document starts.
//!
//! **This dimension has no score, on purpose.** "Fits three-act shape" is not a virtue:
//! a story that refuses the shape may be doing so deliberately, and every beat here is
//! placed by a window (三幕 = 25/50/25 is a convention, not a measurement) sitting on
//! top of signals that are themselves proxies. A number would dress that up as a
//! precision the method does not have. What is reported instead is a fit in words: how
//! many beats the document puts where the model expects, which ones sit further out
//! than expected, and which ones these signals simply do not show.
//!
//! Two limits are stated in the output rather than hidden:
//! - `missing` means the signals show nothing there, not that the story lacks the beat.
//!   A quiet 召唤 that changes no entity set is invisible to a turn, and a turning point
//!   told in a line of dialogue may sit in a segment whose tension never moves.
//! - The cut matters. Where the document has fewer than three sections the curve runs
//!   over equal block chunks, so the positions are arithmetic rather than the author's
//!   own chapter boundaries; the output says so.
//!
//! Deterministic: fixed beat table, landmarks in reading order, ties broken by strength
//! then label, no clock and no randomness; the same document always yields the same
//! landmarks, the same acts and the same fit.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;

use crate::pacing::PacingStats;
use crate::plot::{PlotStats, TurningPoint};
use crate::segment::plan_segments;
use crate::text::visible_char_count;
use crate::Result;

/// Below this many visible characters the beat windows are placed over too little text
/// for the match to say anything.
const MIN_TOTAL_CHARS: u64 = 1500;
/// Thread-set change at which a section boundary counts as a structural turn. Below
/// this, a boundary is a cast change rather than a turn.
const TURN_MIN_CHANGE: f64 = 0.15;
/// Tension points a segment must sit above the document mean to count as a peak.
const PEAK_BAND: f64 = 8.0;
/// ...and below the mean to count as a trough.
const TROUGH_BAND: f64 = 8.0;
/// A beat counts as 错位 rather than 未见 when the nearest landmark sits at most this
/// much of the document outside its window.
const DISPLACE_TOLERANCE: f64 = 0.12;
/// A first act this long, or a final act this short, is reported as lopsided.
const LONG_FIRST_ACT: f64 = 0.50;
const SHORT_LAST_ACT: f64 = 0.10;
/// Fewer landmarks than this and there is not enough to place a shape on.
const MIN_LANDMARKS: u32 = 4;
/// How many landmarks are reported, reading order.
const MAX_LANDMARKS: usize = 24;
/// How many structure notes are reported.
const MAX_ISSUES: usize = 8;

/// A place where the document's own signals say something happens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureLandmark {
    /// `opening` | `turn` | `peak` | `trough` | `climax`.
    pub kind: String,
    /// The curve segment the landmark sits in.
    pub section: String,
    /// Where the segment sits in the document, measured at its midpoint (0-1).
    pub position: f64,
    /// Thread change (0-1) for a turn, tension (0-100) for the rest, 0 for the opening.
    pub strength: f64,
    /// What the signals show there.
    pub detail: String,
}

/// One beat a model expects, and what the document puts there.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureBeat {
    /// Beat id, e.g. `inciting_incident`.
    pub beat: String,
    /// Where the model expects it, as document fractions (start, end).
    pub window: (f64, f64),
    /// `found` | `displaced` | `missing`.
    pub status: String,
    /// Where the supporting landmark is; empty when the beat is 未见.
    pub section: String,
    /// Position of that landmark (0-1); 0 when missing.
    pub position: f64,
    /// How far outside the window the nearest landmark sits (0 when inside).
    pub offset: f64,
    /// Which landmark answers the beat, in words.
    pub evidence: String,
}

/// One classic shape tried against the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureModel {
    /// `three_act` | `kisho_tenketsu` | `hero_journey`.
    pub model: String,
    /// `matched` | `partial` | `unclear` — how many beats land where expected.
    pub fit: String,
    pub beats: Vec<StructureBeat>,
    /// How many beats were found / displaced / missing.
    pub found: u32,
    pub displaced: u32,
    pub missing: u32,
}

/// One act of the document's own three-way split, cut at its two strongest turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureAct {
    /// `act_one` | `act_two` | `act_three`.
    pub name: String,
    pub from_section: String,
    pub to_section: String,
    pub char_count: u64,
    /// Share of the document's visible characters (0-1).
    pub share: f64,
    pub mean_tension: f64,
    pub min_tension: f64,
    pub max_tension: f64,
}

/// Something the shape does that a writer would want to know about.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureIssue {
    /// `no_early_turn` | `missing_midpoint` | `climax_early` | `no_descent` |
    /// `act_imbalance` | `too_few_landmarks`.
    pub kind: String,
    pub section: String,
    pub detail: String,
}

/// Story structure analysis output for one document. There is no score field: the
/// dimension describes a shape rather than grading one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureStats {
    /// The landmarks the matching worked from, reading order.
    pub landmarks: Vec<StructureLandmark>,
    /// The document's own three-way split; empty when fewer than two turns were found.
    pub acts: Vec<StructureAct>,
    /// The shapes tried against the document; empty when `readable` is false.
    pub models: Vec<StructureModel>,
    pub issues: Vec<StructureIssue>,
    pub segment_count: u32,
    /// True when the segments are block chunks rather than sections.
    pub chunked: bool,
    pub char_count: u64,
    /// Turns among the landmarks.
    pub turn_count: u32,
    /// False when the document is too short, too flatly cut, or has no tension curve
    /// for the matching to mean anything.
    pub readable: bool,
    /// Why `readable` is false, in words; empty when it is true.
    pub note: String,
}

/// The beat table: `model, beat, window start, window end, landmark kinds in preference
/// order`.
///
/// The windows are a convention written down, not a measurement: 三幕 at 25/50/25 comes
/// from screenwriting handbooks, and a manuscript that puts its first turn at 15% is not
/// off by 10% in any physical sense. They are wide on purpose. What is reported is which
/// landmark the document actually puts in each window, so the writer can see the shape
/// rather than be graded against the table.
///
/// Kinds are tried in the order listed, and the first kind with a landmark in the window
/// answers the beat: a beat that means "something changes" prefers a turn over a peak,
/// because a turn is the stronger claim about the plot.
const BEATS: &[(&str, &str, f64, f64, &[&str])] = &[
    // 三幕结构.
    ("three_act", "setup", 0.00, 0.24, &["opening", "peak"]),
    ("three_act", "first_turn", 0.18, 0.38, &["turn", "peak"]),
    (
        "three_act",
        "midpoint",
        0.42,
        0.62,
        &["turn", "peak", "trough"],
    ),
    ("three_act", "second_turn", 0.60, 0.80, &["turn", "peak"]),
    ("three_act", "climax", 0.76, 0.96, &["climax", "peak"]),
    ("three_act", "resolution", 0.90, 1.00, &["trough", "turn"]),
    // 起承转合.
    ("kisho_tenketsu", "kisho", 0.00, 0.28, &["opening", "peak"]),
    ("kisho_tenketsu", "sho", 0.25, 0.52, &["peak", "turn"]),
    (
        "kisho_tenketsu",
        "ten",
        0.50,
        0.78,
        &["turn", "trough", "peak"],
    ),
    (
        "kisho_tenketsu",
        "ketsu",
        0.75,
        1.00,
        &["trough", "climax", "peak"],
    ),
    // 英雄之旅 — only the beats these signals can actually support; the full twelve-step
    // version asks for things no entity or punctuation curve can see.
    (
        "hero_journey",
        "ordinary_world",
        0.00,
        0.15,
        &["opening", "trough"],
    ),
    ("hero_journey", "call", 0.08, 0.28, &["turn", "peak"]),
    ("hero_journey", "threshold", 0.22, 0.40, &["turn", "peak"]),
    ("hero_journey", "tests", 0.35, 0.62, &["peak", "turn"]),
    (
        "hero_journey",
        "inmost_cave",
        0.58,
        0.80,
        &["trough", "turn"],
    ),
    (
        "hero_journey",
        "resurrection",
        0.74,
        0.92,
        &["peak", "climax", "turn"],
    ),
    ("hero_journey", "return", 0.88, 1.00, &["trough", "turn"]),
];

/// The models in the order they are reported.
const MODELS: &[&str] = &["three_act", "kisho_tenketsu", "hero_journey"];

/// One curve segment with the position it occupies in the document.
struct Seg {
    label: String,
    chars: u64,
    /// Document fraction at the segment's midpoint — where a landmark at this segment
    /// is said to sit.
    mid: f64,
    /// The pacing curve's tension here, when pacing produced a point for this segment.
    tension: Option<f64>,
}

/// Cut the document into curve segments and attach each raw section path to the segment
/// its first block falls in.
///
/// The path map is what lets a 转折点 from 情节分析 — which is keyed by the document's own
/// `section_path`, possibly nested — be placed on the shared curve, including when the
/// curve runs over block chunks rather than sections.
fn collect_segments(
    doc: &DocumentData,
    pacing: &PacingStats,
) -> (Vec<Seg>, bool, HashMap<String, usize>) {
    let plan = plan_segments(doc);
    let mut chars: Vec<u64> = vec![0; plan.labels.len()];
    let mut section_to_segment: HashMap<String, usize> = HashMap::new();
    for (idx, block) in doc.blocks.iter().enumerate() {
        let segment = plan.segment_of[idx];
        chars[segment] += visible_char_count(&block.content) as u64;
        section_to_segment
            .entry(block.section_path.clone())
            .or_insert(segment);
    }

    let total: u64 = chars.iter().sum();
    let tension_of: HashMap<&str, f64> = pacing
        .points
        .iter()
        .map(|p| (p.section.as_str(), p.tension))
        .collect();

    let mut segments = Vec::with_capacity(plan.labels.len());
    let mut seen = 0u64;
    for (idx, label) in plan.labels.iter().enumerate() {
        // Midpoint by character mass, not by segment count: a chapter of 6000 characters
        // occupies twice the document of one of 3000, and the beat windows are read
        // against that mass.
        let mid = if total == 0 {
            0.0
        } else {
            (seen as f64 + chars[idx] as f64 / 2.0) / total as f64
        };
        segments.push(Seg {
            label: label.clone(),
            chars: chars[idx],
            mid,
            tension: tension_of.get(label.as_str()).copied(),
        });
        seen += chars[idx];
    }

    (segments, plan.chunked, section_to_segment)
}

/// Tension rendered for a detail string; a segment with no pacing point says so.
fn tension_text(tension: Option<f64>) -> String {
    match tension {
        Some(t) => format!("{t:.0}"),
        None => "—".to_string(),
    }
}

/// What a 转折点 changed, in words: the threads that arrive and the ones that leave.
fn turn_detail(turn: &TurningPoint) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !turn.appeared.is_empty() {
        parts.push(format!(
            "新出现：{}",
            turn.appeared
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("、")
        ));
    }
    if !turn.dropped.is_empty() {
        parts.push(format!(
            "退出：{}",
            turn.dropped
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("、")
        ));
    }
    let body = if parts.is_empty() {
        "活跃线程变化".to_string()
    } else {
        parts.join("；")
    };
    format!("线程变化 {:.0}%：{}", turn.change * 100.0, body)
}

/// Where the document's own signals say something happens.
///
/// Reading order, and within one segment in kind order (`opening`, `turn`, `peak`,
/// `trough`, `climax`), so the same document always yields the same list. A segment can
/// carry more than one kind — a chapter can be both a cast change and the curve's
/// highest point — and the two are kept apart because a model asks for one or the other.
///
/// `flat_curve` is 节奏分析's own verdict: when the tension never moves, no segment is
/// named as the climax.
fn find_landmarks(
    segments: &[Seg],
    turns: &[TurningPoint],
    section_to_segment: &HashMap<String, usize>,
    flat_curve: bool,
) -> Vec<StructureLandmark> {
    if segments.is_empty() {
        return Vec::new();
    }

    let kind_rank = |kind: &str| match kind {
        "opening" => 0,
        "turn" => 1,
        "peak" => 2,
        "trough" => 3,
        _ => 4,
    };

    let mut placed: Vec<(usize, StructureLandmark)> = Vec::new();

    // The opening is not a signal, it is where the document starts — but every model
    // expects something there, so it is stated rather than left to be inferred.
    placed.push((
        0,
        StructureLandmark {
            kind: "opening".to_string(),
            section: segments[0].label.clone(),
            position: segments[0].mid,
            strength: 0.0,
            detail: format!(
                "开篇于 {}，张力 {}",
                segments[0].label,
                tension_text(segments[0].tension)
            ),
        },
    ));

    for turn in turns {
        if turn.change < TURN_MIN_CHANGE {
            continue;
        }
        if let Some(&segment) = section_to_segment.get(&turn.section) {
            placed.push((
                segment,
                StructureLandmark {
                    kind: "turn".to_string(),
                    section: segments[segment].label.clone(),
                    position: segments[segment].mid,
                    strength: turn.change,
                    detail: turn_detail(turn),
                },
            ));
        }
    }

    let tensions: Vec<f64> = segments.iter().filter_map(|s| s.tension).collect();
    if !tensions.is_empty() {
        let mean = tensions.iter().sum::<f64>() / tensions.len() as f64;

        // The curve's highest point, earliest when two segments tie.
        let mut climax: Option<(usize, f64)> = None;
        for (idx, seg) in segments.iter().enumerate() {
            if let Some(t) = seg.tension {
                if climax.map(|(_, best)| t > best).unwrap_or(true) {
                    climax = Some((idx, t));
                }
            }
        }
        // A curve that never moves has no high point: the maximum would just be whichever
        // segment happens to come first. 节奏分析 already decides whether the curve
        // moves, so that judgement is reused rather than made twice.
        let climax = if flat_curve { None } else { climax };
        if let Some((idx, t)) = climax {
            placed.push((
                idx,
                StructureLandmark {
                    kind: "climax".to_string(),
                    section: segments[idx].label.clone(),
                    position: segments[idx].mid,
                    strength: t,
                    detail: format!("全文张力最高点 {t:.0}（均值 {mean:.0}）"),
                },
            ));
        }

        // Local extremes far enough from the document's own mean to be a shape rather
        // than noise. Endpoints count as extremes against their single neighbour: a
        // document that starts high and only comes down has a real peak at the start.
        // The segment holding the curve's highest point is skipped here: it is already
        // named 张力最高点, and a second row for the same segment with the same number
        // would read like two findings rather than one.
        for (idx, seg) in segments.iter().enumerate() {
            let Some(t) = seg.tension else {
                continue;
            };
            if climax.map(|(i, _)| i) == Some(idx) {
                continue;
            }
            let prev = idx
                .checked_sub(1)
                .and_then(|i| segments.get(i))
                .and_then(|s| s.tension);
            let next = segments.get(idx + 1).and_then(|s| s.tension);
            let above = prev.is_none_or(|p| t > p) && next.is_none_or(|n| t > n);
            let below = prev.is_none_or(|p| t < p) && next.is_none_or(|n| t < n);
            if t >= mean + PEAK_BAND && above {
                placed.push((
                    idx,
                    StructureLandmark {
                        kind: "peak".to_string(),
                        section: seg.label.clone(),
                        position: seg.mid,
                        strength: t,
                        detail: format!("张力高点 {t:.0}（均值 {mean:.0}）"),
                    },
                ));
            } else if t <= mean - TROUGH_BAND && below {
                placed.push((
                    idx,
                    StructureLandmark {
                        kind: "trough".to_string(),
                        section: seg.label.clone(),
                        position: seg.mid,
                        strength: t,
                        detail: format!("张力低点 {t:.0}（均值 {mean:.0}）"),
                    },
                ));
            }
        }
    }

    placed.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| kind_rank(&a.1.kind).cmp(&kind_rank(&b.1.kind)))
    });
    placed
        .into_iter()
        .take(MAX_LANDMARKS)
        .map(|(_, landmark)| landmark)
        .collect()
}

/// The strongest of a set of candidates: highest strength, earliest position on a tie.
fn strongest<'a>(
    candidates: impl Iterator<Item = &'a StructureLandmark>,
) -> Option<&'a StructureLandmark> {
    candidates.min_by(|a, b| {
        b.strength
            .total_cmp(&a.strength)
            .then_with(|| a.position.total_cmp(&b.position))
    })
}

/// Whether a landmark of kind `actual` can answer a beat asking for kind `requested`.
///
/// The curve's highest point is also a high point, so a `climax` landmark answers a
/// request for a `peak`; the reverse is not true, because a local high that is not the
/// document's highest point cannot carry a claim about the whole curve.
fn kind_answers(requested: &str, actual: &str) -> bool {
    requested == actual || (requested == "peak" && actual == "climax")
}

/// Try one model against the landmarks.
///
/// Each beat is answered by the strongest landmark of the first kind that has one inside
/// the window; failing that, by the nearest one within `DISPLACE_TOLERANCE` of the window
/// (reported as 错位, with the distance kept so the report can say how far off); failing
/// that, as 未见. A landmark may answer several beats — the curve's highest point is
/// legitimately both the 三幕 climax and the 英雄之旅 复活 — because the claim being made
/// is "this signal sits here", not "this signal is used up".
fn match_model(model: &str, landmarks: &[StructureLandmark]) -> StructureModel {
    let mut beats: Vec<StructureBeat> = Vec::new();

    for (model_id, beat, from, to, kinds) in BEATS {
        if *model_id != model {
            continue;
        }

        let chosen = kinds
            .iter()
            .map(|kind| {
                strongest(landmarks.iter().filter(|l| {
                    kind_answers(kind, &l.kind) && l.position >= *from && l.position <= *to
                }))
            })
            .find(Option::is_some)
            .flatten();

        if let Some(landmark) = chosen {
            beats.push(StructureBeat {
                beat: beat.to_string(),
                window: (*from, *to),
                status: "found".to_string(),
                section: landmark.section.clone(),
                position: landmark.position,
                offset: 0.0,
                evidence: landmark.detail.clone(),
            });
            continue;
        }

        // Nothing in the window: the nearest landmark of the kinds this beat asks for,
        // if it is close enough to be worth naming.
        let distance = |position: f64| {
            if position < *from {
                *from - position
            } else if position > *to {
                position - *to
            } else {
                0.0
            }
        };
        let nearest = kinds
            .iter()
            .flat_map(|kind| {
                landmarks
                    .iter()
                    .filter(move |l| kind_answers(kind, &l.kind))
            })
            .min_by(|a, b| {
                distance(a.position)
                    .total_cmp(&distance(b.position))
                    .then_with(|| a.position.total_cmp(&b.position))
            });

        let (status, landmark, offset) = match nearest {
            Some(landmark) if distance(landmark.position) <= DISPLACE_TOLERANCE => {
                ("displaced", Some(landmark), distance(landmark.position))
            }
            _ => ("missing", None, 0.0),
        };

        beats.push(StructureBeat {
            beat: beat.to_string(),
            window: (*from, *to),
            status: status.to_string(),
            section: landmark.map(|l| l.section.clone()).unwrap_or_default(),
            position: landmark.map(|l| l.position).unwrap_or_default(),
            offset,
            evidence: landmark.map(|l| l.detail.clone()).unwrap_or_default(),
        });
    }

    let found = beats.iter().filter(|b| b.status == "found").count() as u32;
    let displaced = beats.iter().filter(|b| b.status == "displaced").count() as u32;
    let missing = beats.iter().filter(|b| b.status == "missing").count() as u32;
    let total = beats.len() as u32;
    // Three quarters of the beats in place is called 吻合, half 部分吻合. These are the
    // thresholds of the verdict word, not a score: nothing else is computed from them,
    // and the counts they come from are reported beside the word.
    let fit = if total > 0 && found * 4 >= total * 3 {
        "matched"
    } else if total > 0 && found * 2 >= total {
        "partial"
    } else {
        "unclear"
    };

    StructureModel {
        model: model.to_string(),
        fit: fit.to_string(),
        beats,
        found,
        displaced,
        missing,
    }
}

/// The document's own three-way split, cut at its two strongest turns.
///
/// This is not the three-act *model* — it is where this document actually breaks, which
/// is the thing a writer can check. Empty when fewer than two turns clear the threshold:
/// one turn gives two parts, and calling that a three-act split would mean inventing the
/// second cut.
fn find_acts(
    segments: &[Seg],
    turns: &[TurningPoint],
    section_to_segment: &HashMap<String, usize>,
    total_chars: u64,
) -> Vec<StructureAct> {
    // One cut per segment, keeping the sharpest turn that lands there.
    let mut cuts: Vec<(usize, f64)> = Vec::new();
    for turn in turns {
        if turn.change < TURN_MIN_CHANGE {
            continue;
        }
        let Some(&segment) = section_to_segment.get(&turn.section) else {
            continue;
        };
        // A cut at the first or last segment would leave an empty act.
        if segment == 0 || segment >= segments.len() {
            continue;
        }
        if let Some(entry) = cuts.iter_mut().find(|(idx, _)| *idx == segment) {
            entry.1 = entry.1.max(turn.change);
        } else {
            cuts.push((segment, turn.change));
        }
    }
    if cuts.len() < 2 {
        return Vec::new();
    }

    // Sharpest first, earliest on a tie, so the same two cuts come out every run.
    cuts.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut chosen: Vec<usize> = cuts.iter().take(2).map(|(idx, _)| *idx).collect();
    chosen.sort_unstable();

    let bounds = [0, chosen[0], chosen[1], segments.len()];
    let names = ["act_one", "act_two", "act_three"];
    let mut acts = Vec::with_capacity(3);
    for (idx, name) in names.iter().enumerate() {
        let range = &segments[bounds[idx]..bounds[idx + 1]];
        let chars: u64 = range.iter().map(|s| s.chars).sum();
        let tensions: Vec<f64> = range.iter().filter_map(|s| s.tension).collect();
        let (mean, min, max) = if tensions.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            (
                tensions.iter().sum::<f64>() / tensions.len() as f64,
                tensions.iter().cloned().fold(f64::INFINITY, f64::min),
                tensions.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            )
        };
        acts.push(StructureAct {
            name: (*name).to_string(),
            from_section: range[0].label.clone(),
            to_section: range[range.len() - 1].label.clone(),
            char_count: chars,
            share: if total_chars == 0 {
                0.0
            } else {
                chars as f64 / total_chars as f64
            },
            mean_tension: mean,
            min_tension: min,
            max_tension: max,
        });
    }
    acts
}

/// What the shape does that a writer would want to know about.
///
/// These are observations about where the signals sit, not corrections: 张力最高点落在
/// 30% is a fact about the curve, and whether that is a problem is the writer's call.
fn find_issues(
    segments: &[Seg],
    landmarks: &[StructureLandmark],
    acts: &[StructureAct],
) -> Vec<StructureIssue> {
    let mut issues: Vec<StructureIssue> = Vec::new();
    if landmarks.is_empty() {
        return issues;
    }

    // Nothing changes in the first quarter.
    let early = landmarks
        .iter()
        .any(|l| (l.kind == "turn" || kind_answers("peak", &l.kind)) && l.position <= 0.25);
    if !early {
        issues.push(StructureIssue {
            kind: "no_early_turn".to_string(),
            section: String::new(),
            detail: "前 25% 内没有可辨的转折或张力高点：开头在信号上是平的".to_string(),
        });
    }

    // A middle with nothing in it.
    if !landmarks
        .iter()
        .any(|l| l.position >= 0.40 && l.position <= 0.62)
    {
        issues.push(StructureIssue {
            kind: "missing_midpoint".to_string(),
            section: String::new(),
            detail: "40%-62% 之间没有任何地标：中段在信号上没有变化".to_string(),
        });
    }

    // The curve's highest point sits well before the end.
    if let Some(climax) = landmarks.iter().find(|l| l.kind == "climax") {
        if climax.position < 0.60 {
            issues.push(StructureIssue {
                kind: "climax_early".to_string(),
                section: climax.section.clone(),
                detail: format!(
                    "张力最高点落在 {:.0}%（{}），其后还有 {:.0}% 的正文",
                    climax.position * 100.0,
                    climax.section,
                    (1.0 - climax.position) * 100.0
                ),
            });
        }
    }

    // The curve never comes back down: the last segment is still at its high point.
    let tensions: Vec<f64> = segments.iter().filter_map(|s| s.tension).collect();
    if tensions.len() >= 3 {
        let highest = tensions.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let last = tensions[tensions.len() - 1];
        if last >= highest - PEAK_BAND {
            issues.push(StructureIssue {
                kind: "no_descent".to_string(),
                section: segments[segments.len() - 1].label.clone(),
                detail: format!(
                    "结尾张力 {last:.0} 与全文最高点 {highest:.0} 相差不足 {:.0}：没有回落的段落",
                    PEAK_BAND
                ),
            });
        }
    }

    // The document's own acts are lopsided.
    if acts.len() == 3 && (acts[0].share > LONG_FIRST_ACT || acts[2].share < SHORT_LAST_ACT) {
        issues.push(StructureIssue {
            kind: "act_imbalance".to_string(),
            section: String::new(),
            detail: format!(
                "第一幕占 {:.0}%，第三幕占 {:.0}%",
                acts[0].share * 100.0,
                acts[2].share * 100.0
            ),
        });
    }

    // Not enough to place a shape on.
    if landmarks.len() < MIN_LANDMARKS as usize {
        issues.push(StructureIssue {
            kind: "too_few_landmarks".to_string(),
            section: String::new(),
            detail: format!("只有 {} 个地标，形状判断不可靠", landmarks.len()),
        });
    }

    issues.truncate(MAX_ISSUES);
    issues
}

/// Match the document against the classic shapes, from its own landmarks.
pub fn run_structure_analysis(
    doc: &DocumentData,
    pacing: &PacingStats,
    plot: &PlotStats,
) -> Result<StructureStats> {
    let (segments, chunked, section_to_segment) = collect_segments(doc, pacing);
    let char_count: u64 = segments.iter().map(|s| s.chars).sum();
    let landmarks = find_landmarks(
        &segments,
        &plot.turning_points,
        &section_to_segment,
        pacing.verdict == "flat",
    );
    let turn_count = landmarks.iter().filter(|l| l.kind == "turn").count() as u32;
    let has_curve = segments.iter().any(|s| s.tension.is_some());

    // The gate. Placing beats over a few hundred characters, or over a curve the pacing
    // pass never produced, would give a table that looks like an answer.
    let (readable, note) = if segments.is_empty() {
        (false, "文档没有可切分的正文。".to_string())
    } else if char_count < MIN_TOTAL_CHARS {
        (
            false,
            format!(
                "全文可见正文 {char_count} 字，不足以按位置匹配经典结构（需 {} 字以上）",
                MIN_TOTAL_CHARS
            ),
        )
    } else if !has_curve {
        (
            false,
            "节奏分析未产生张力曲线，无法判断高点与回落。".to_string(),
        )
    } else {
        (true, String::new())
    };

    let mut acts: Vec<StructureAct> = Vec::new();
    let mut models: Vec<StructureModel> = Vec::new();
    let mut issues: Vec<StructureIssue> = Vec::new();
    if readable {
        acts = find_acts(
            &segments,
            &plot.turning_points,
            &section_to_segment,
            char_count,
        );
        models = MODELS
            .iter()
            .map(|model| match_model(model, &landmarks))
            .collect();
        issues = find_issues(&segments, &landmarks, &acts);
    }

    Ok(StructureStats {
        landmarks,
        acts,
        models,
        issues,
        segment_count: segments.len() as u32,
        chunked,
        char_count,
        turn_count,
        readable,
        note,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pacing::run_pacing_analysis;
    use crate::plot::run_plot_analysis;
    use studio_core::document::SemanticBlock;
    use studio_core::entity::{Entity, EntityCategory};

    /// Narration with nothing that lifts tension: the floor of the curve.
    const FLAT: &str =
        "这是一家成立三十年的出版社的年度总结报告。它完整记录了全年所有的编辑与印刷发行工作。";
    /// Dialogue, exclamations and short sentences: a high point of the curve.
    const LOUD: &str = "“快走！”他喊道。“你听见了吗？”门外传来一阵急促的脚步声！";
    /// Dialogue that dominates the paragraph: the ceiling the signals can reach.
    const HOT: &str =
        "“别走！”她喊道，“外面在下雨，你听见了吗？”“我知道！”他停下脚步，回头看她。“我必须去。”";

    fn para(path: &str, body: &str, names: &[&str]) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: body.into(),
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

    /// One section: `blocks` paragraphs of `style` with `names` present in every
    /// paragraph, so each thread clears the mention floor inside the document.
    fn section(path: &str, style: &str, blocks: usize, names: &[&str]) -> Vec<SemanticBlock> {
        (0..blocks)
            .map(|_| para(path, &style.repeat(5), names))
            .collect()
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

    /// Six sections whose weights (3,3,3,5,4,2 paragraphs) put the segment midpoints at
    /// roughly 8%, 22%, 38%, 55%, 80%, 95% of the document, with the thread sets
    /// arranged so the turns land where each fixture needs them.
    struct Fixture {
        doc: DocumentData,
    }

    impl Fixture {
        fn build(styles: [&str; 6], weights: [usize; 6], casts: [Vec<&str>; 6]) -> Fixture {
            let paths = ["第一章", "第二章", "第三章", "第四章", "第五章", "第六章"];
            let mut blocks = Vec::new();
            for idx in 0..6 {
                blocks.extend(section(paths[idx], styles[idx], weights[idx], &casts[idx]));
            }
            Fixture {
                doc: doc_of(blocks),
            }
        }

        fn run(&self) -> StructureStats {
            let pacing = run_pacing_analysis(&self.doc).unwrap();
            let plot = run_plot_analysis(&self.doc).unwrap();
            run_structure_analysis(&self.doc, &pacing, &plot).unwrap()
        }
    }

    /// Turns at 2、4、5 and the curve's highest point in 第五章.
    fn shaped() -> Fixture {
        Fixture::build(
            [FLAT, LOUD, FLAT, FLAT, HOT, FLAT],
            [3, 3, 3, 5, 4, 2],
            [
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯", "沈墨"],
                vec!["林岚", "周伯", "沈墨"],
                vec!["林岚", "沈墨"],
                vec!["林岚", "雷远"],
                vec!["林岚", "雷远"],
            ],
        )
    }

    #[test]
    fn test_a_shaped_document_matches_three_acts() {
        let stats = shaped().run();
        assert!(stats.readable, "{}", stats.note);
        assert_eq!(stats.segment_count, 6);
        assert!(!stats.chunked);
        assert!(stats.char_count > 1500);

        let three = stats
            .models
            .iter()
            .find(|m| m.model == "three_act")
            .unwrap();
        assert_eq!(three.beats.len(), 6);
        assert_eq!(three.found, 6);
        assert_eq!(three.missing, 0);
        assert_eq!(three.fit, "matched");

        // The document's own split follows its two sharpest turns, and the three acts
        // together are the whole document.
        assert_eq!(stats.acts.len(), 3);
        assert_eq!(stats.acts[0].from_section, "第一章");
        assert_eq!(stats.acts[0].to_section, "第一章");
        assert_eq!(stats.acts[2].to_section, "第六章");
        let chars: u64 = stats.acts.iter().map(|a| a.char_count).sum();
        assert_eq!(chars, stats.char_count);

        // Nothing to remark on: the turns are early, the middle is occupied, the high
        // point is late and the curve comes back down.
        assert!(stats.issues.is_empty(), "{:?}", stats.issues);
    }

    #[test]
    fn test_landmarks_are_in_reading_order() {
        let stats = shaped().run();
        assert!(!stats.landmarks.is_empty());
        assert!(stats
            .landmarks
            .windows(2)
            .all(|w| w[0].position <= w[1].position));
        // The opening is always first, and it is the document's own first segment.
        assert_eq!(stats.landmarks[0].kind, "opening");
        assert_eq!(stats.landmarks[0].section, "第一章");
        // The climax is the curve's highest point, and it is the last high point here.
        let climax = stats
            .landmarks
            .iter()
            .filter(|l| l.kind == "climax")
            .collect::<Vec<_>>();
        assert_eq!(climax.len(), 1);
        assert_eq!(climax[0].section, "第五章");
        assert_eq!(stats.turn_count, 3);
    }

    #[test]
    fn test_the_highest_segment_is_not_named_twice() {
        let stats = shaped().run();
        let climax = stats
            .landmarks
            .iter()
            .filter(|l| l.kind == "climax")
            .collect::<Vec<_>>();
        assert_eq!(climax.len(), 1);
        // The highest segment is also locally high, but it is named once: a second row
        // with the same position and the same number would read like two findings.
        let peaks_at_climax = stats
            .landmarks
            .iter()
            .filter(|l| l.kind == "peak" && l.section == climax[0].section)
            .count();
        assert_eq!(peaks_at_climax, 0);
        // A beat asking for a high point still lands there: the highest point of the curve
        // answers a request for a peak.
        let hero = stats
            .models
            .iter()
            .find(|m| m.model == "hero_journey")
            .unwrap();
        let resurrection = hero
            .beats
            .iter()
            .find(|b| b.beat == "resurrection")
            .unwrap();
        assert_eq!(resurrection.status, "found");
        assert_eq!(resurrection.section, "第五章");
    }

    #[test]
    fn test_a_beat_can_be_displaced() {
        let stats = shaped().run();
        let kisho = stats
            .models
            .iter()
            .find(|m| m.model == "kisho_tenketsu")
            .unwrap();
        let sho = kisho.beats.iter().find(|b| b.beat == "sho").unwrap();
        // 承 is expected at 25%-52%; the nearest landmark is the turn just past the
        // window, so it is named as displaced with the distance kept rather than rounded
        // away.
        assert_eq!(sho.status, "displaced");
        assert_eq!(sho.section, "第四章");
        assert!(
            sho.offset > 0.0 && sho.offset <= DISPLACE_TOLERANCE,
            "{}",
            sho.offset
        );
        assert!(!sho.evidence.is_empty());
        assert_eq!(kisho.displaced, 1);
    }

    #[test]
    fn test_a_late_first_turn_is_missing_not_invented() {
        // Same document, but the cast only changes late: nothing answers 第一幕转折点.
        let late = Fixture::build(
            [FLAT, FLAT, FLAT, LOUD, HOT, FLAT],
            [3, 3, 3, 5, 4, 2],
            [
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯", "沈墨"],
                vec!["林岚", "沈墨", "雷远"],
                vec!["林岚", "沈墨", "雷远"],
            ],
        );
        let stats = late.run();
        let three = stats
            .models
            .iter()
            .find(|m| m.model == "three_act")
            .unwrap();
        let first = three.beats.iter().find(|b| b.beat == "first_turn").unwrap();
        assert_eq!(first.status, "missing");
        assert_eq!(first.section, "");
        assert_eq!(first.evidence, "");
        assert!(stats.issues.iter().any(|i| i.kind == "no_early_turn"));
    }

    #[test]
    fn test_an_early_climax_is_reported() {
        let early = Fixture::build(
            [HOT, LOUD, FLAT, FLAT, FLAT, FLAT],
            [3, 3, 3, 5, 4, 2],
            [
                vec!["林岚"],
                vec!["林岚", "沈墨"],
                vec!["林岚", "沈墨"],
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯"],
            ],
        );
        let stats = early.run();
        let climax = stats.landmarks.iter().find(|l| l.kind == "climax").unwrap();
        assert_eq!(climax.section, "第一章");
        let issue = stats
            .issues
            .iter()
            .find(|i| i.kind == "climax_early")
            .expect("climax at 8% should be reported");
        assert_eq!(issue.section, "第一章");
        // And the model that expects its climax late has to say it found nothing there.
        let three = stats
            .models
            .iter()
            .find(|m| m.model == "three_act")
            .unwrap();
        let beat = three.beats.iter().find(|b| b.beat == "climax").unwrap();
        assert_eq!(beat.status, "missing");
    }

    #[test]
    fn test_late_turns_make_the_acts_lopsided() {
        let late = Fixture::build(
            [FLAT, FLAT, FLAT, LOUD, FLAT, FLAT],
            [3, 3, 3, 5, 4, 2],
            [
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯"],
                vec!["林岚", "周伯", "沈墨"],
                vec!["林岚", "沈墨"],
            ],
        );
        let stats = late.run();
        assert_eq!(stats.acts.len(), 3);
        assert_eq!(stats.acts[0].to_section, "第四章");
        assert!(stats.acts[0].share > 0.5);
        let issue = stats
            .issues
            .iter()
            .find(|i| i.kind == "act_imbalance")
            .expect("a 70% first act should be reported");
        assert!(issue.detail.contains("第一幕"));
    }

    #[test]
    fn test_a_short_document_is_not_read_for_shape() {
        let doc = doc_of(vec![
            para("第一章", FLAT, &["林岚"]),
            para("第二章", LOUD, &["林岚"]),
            para("第三章", FLAT, &["林岚"]),
        ]);
        let pacing = run_pacing_analysis(&doc).unwrap();
        let plot = run_plot_analysis(&doc).unwrap();
        let stats = run_structure_analysis(&doc, &pacing, &plot).unwrap();
        assert!(!stats.readable);
        assert!(stats.note.contains("字"));
        // No table is produced rather than a meaningless one.
        assert!(stats.models.is_empty());
        assert!(stats.acts.is_empty());
        assert!(stats.issues.is_empty());
    }

    #[test]
    fn test_a_flat_curve_says_there_is_no_curve() {
        let fixture = shaped();
        let mut pacing = run_pacing_analysis(&fixture.doc).unwrap();
        pacing.points.clear();
        let plot = run_plot_analysis(&fixture.doc).unwrap();
        let stats = run_structure_analysis(&fixture.doc, &pacing, &plot).unwrap();
        assert!(!stats.readable);
        assert!(stats.note.contains("张力曲线"));
        assert!(stats.models.is_empty());
    }

    #[test]
    fn test_a_document_without_sections_says_its_positions_are_arithmetic() {
        // One long section: the curve runs over block chunks, so the beat windows are
        // placed over cuts the author never made.
        let mut blocks = Vec::new();
        for idx in 0..24 {
            let style = if idx < 6 { LOUD } else { FLAT };
            blocks.push(para("同一章", &style.repeat(5), &["林岚", "周伯"]));
        }
        let doc = doc_of(blocks);
        let pacing = run_pacing_analysis(&doc).unwrap();
        let plot = run_plot_analysis(&doc).unwrap();
        let stats = run_structure_analysis(&doc, &pacing, &plot).unwrap();
        assert!(stats.chunked);
        assert_eq!(stats.segment_count, 10);
        assert!(stats.readable, "{}", stats.note);
        // One section gives no turn, so there is no split to report.
        assert!(stats.acts.is_empty());
        assert_eq!(stats.turn_count, 0);
    }

    #[test]
    fn test_a_flat_document_has_too_few_landmarks() {
        // Nothing but narration: the opening is the only landmark, because a curve that
        // never moves has no high point to name.
        let flat = Fixture::build(
            [FLAT, FLAT, FLAT, FLAT, FLAT, FLAT],
            [3, 3, 3, 5, 4, 2],
            [
                vec!["林岚"],
                vec!["林岚"],
                vec!["林岚"],
                vec!["林岚"],
                vec!["林岚"],
                vec!["林岚"],
            ],
        );
        let stats = flat.run();
        assert_eq!(stats.landmarks.len(), 1);
        assert!(stats.issues.iter().any(|i| i.kind == "too_few_landmarks"));
    }

    #[test]
    fn test_there_is_no_score_anywhere() {
        let value: serde_json::Value = serde_json::to_value(shaped().run()).unwrap();
        assert!(value.get("score").is_none());
        for model in value["models"].as_array().unwrap() {
            assert!(model.get("score").is_none());
            for beat in model["beats"].as_array().unwrap() {
                assert!(beat.get("score").is_none());
            }
        }
    }

    #[test]
    fn test_the_same_document_always_yields_the_same_shape() {
        let a = serde_json::to_string(&shaped().run()).unwrap();
        let b = serde_json::to_string(&shaped().run()).unwrap();
        assert_eq!(a, b);
    }
}
