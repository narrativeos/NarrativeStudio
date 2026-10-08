//! Dialogue/narrative ratio: how much of the text is spoken, and where.
//!
//! Two signals carry speech in Chinese prose: quotation marks（“ ” 「 」 『 』 "）and the
//! 破折号 convention, where a line opens with `——` and the spoken words follow. The pass
//! scans both, keeps every piece it finds with its offset, and reports how the text
//! divides between spoken and narrated — segment by segment, so the curve reads against
//! the pacing, conflict, theme and setting curves on the same X axis.
//!
//! A ratio is not a grade. 30% speech is neither better nor worse than 5%, so this
//! dimension carries no score: it counts, and the reader decides what the number means.
//! What it can point at are the shapes that usually need a look — a 400-character
//! monologue, eight blocks without one line of speech, speech that never wears quotes.
//!
//! Deterministic: spans follow document order, segments follow [`plan_segments`],
//! distributions are fixed buckets, never HashMap order.

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;

use crate::segment::plan_segments;
use crate::text::visible_char_count;
use crate::Result;

/// A segment below this many visible characters is too short for its share to mean
/// anything: one 40-character line of speech in a 100-character segment reads as 40%.
const MIN_SEGMENT_CHARS: u64 = 300;
/// Share under which a segment counts as carrying no speech at all.
const SILENT_SHARE: f64 = 0.05;
/// Share above which a segment is almost entirely speech.
const FLOOD_SHARE: f64 = 0.80;
/// A block at or above this share counts as speech-dominant when runs are measured.
const DOMINANT_SHARE: f64 = 0.5;
/// Consecutive blocks with no speech before the stretch is worth flagging.
const QUIET_RUN_BLOCKS: u32 = 6;
/// One piece of speech this long is a monologue rather than a line.
const MONOLOGUE_CHARS: u32 = 300;
/// Below this average piece length, with enough pieces, the speech is telegraphic.
const TELEGRAPHIC_AVG: f64 = 8.0;
const MIN_TELEGRAPHIC_UNITS: u32 = 20;
/// Dash-opened lines needed before the absent quotation marks are worth flagging.
const MIN_DASH_UNITS: u32 = 10;
/// Segment share difference before the mix counts as uneven.
const UNEVEN_SPREAD: f64 = 0.6;
/// Verdict thresholds on the document's dialogue share.
const SPARSE_SHARE: f64 = 0.12;
const HEAVY_SHARE: f64 = 0.55;
/// Shorter documents than this say nothing about a missing line of speech.
const MIN_CHARS_FOR_QUIET: u64 = 200;
/// How many speech samples are reported, longest first.
const MAX_SAMPLES: usize = 8;
/// How many issues are reported, first found first.
const MAX_ISSUES: usize = 12;
/// Characters of one piece of speech kept for display.
const EXCERPT_RADIUS: usize = 40;

/// One segment of the dialogue curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialoguePoint {
    pub section: String,
    pub char_count: u64,
    pub dialogue_chars: u64,
    /// Share of the segment's visible characters that is spoken (0-1).
    pub share: f64,
    /// `share` on the 0-100 scale the other dimensions' curves use.
    pub intensity: f64,
    /// Pieces of speech found here.
    pub units: u32,
    /// Longest single piece of speech here.
    pub longest_unit: u32,
    pub blocks: u32,
    pub blocks_with_dialogue: u32,
}

/// One piece of speech; the longest of them are reported so the ratio has faces on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueSample {
    pub section: String,
    pub chars: u32,
    /// `quote` (between quotation marks) or `dash` (a line opened with ——).
    pub kind: String,
    pub excerpt: String,
}

/// Something the mix does to the reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueIssue {
    /// `no_dialogue` | `silent_segment` | `flooded_segment` | `monologue` |
    /// `telegraphic` | `unquoted` | `uneven`.
    pub kind: String,
    pub section: String,
    pub detail: String,
}

/// Dialogue/narrative analysis output for one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueStats {
    pub points: Vec<DialoguePoint>,
    pub segment_count: u32,
    /// True when the segments are block chunks rather than sections.
    pub chunked: bool,
    /// Visible characters the shares are measured over. Quotation marks themselves
    /// fall on the narrative side: they frame speech rather than speak it.
    pub char_count: u64,
    pub dialogue_chars: u64,
    pub narrative_chars: u64,
    pub dialogue_share: f64,
    pub narrative_share: f64,
    /// Pieces of speech found.
    pub unit_count: u32,
    pub quoted_units: u32,
    pub dash_units: u32,
    pub avg_unit_chars: f64,
    pub longest_unit: u32,
    /// Piece lengths in fixed buckets, short to long.
    pub unit_length_distribution: Vec<(String, u32)>,
    pub blocks: u32,
    pub blocks_with_dialogue: u32,
    /// Longest run of consecutive speech-dominant blocks.
    pub longest_dialogue_run: u32,
    /// Longest run of consecutive blocks carrying no speech at all.
    pub longest_quiet_run: u32,
    /// Visible characters inside that longest quiet run.
    pub longest_quiet_chars: u64,
    /// Segments with enough text and no speech worth speaking of.
    pub silent_segments: u32,
    pub flooded_segments: u32,
    /// `none` | `sparse` | `balanced` | `heavy` — the mix, not its quality.
    pub verdict: String,
    /// Highest segment share minus lowest, over segments with enough text.
    pub spread: f64,
    pub peak_section: String,
    pub peak_share: f64,
    pub quietest_section: String,
    pub quietest_share: f64,
    /// The longest pieces of speech, longest first.
    pub samples: Vec<DialogueSample>,
    pub issues: Vec<DialogueIssue>,
}

/// One detected piece of speech inside a block.
#[derive(Debug, Clone)]
struct Span {
    /// Char offset of the first spoken character.
    start: usize,
    /// Char offset just past the last spoken character.
    end: usize,
    /// Non-whitespace characters between them.
    chars: usize,
    /// `quote` or `dash`.
    kind: &'static str,
}

/// What one block contributes to the ratio.
#[derive(Debug, Default)]
struct BlockFacts {
    chars: u64,
    dialogue_chars: u64,
    spans: Vec<Span>,
    quoted_units: u32,
    dash_units: u32,
}

/// Per-segment counters.
#[derive(Debug, Default)]
struct Segment {
    label: String,
    chars: u64,
    dialogue_chars: u64,
    units: u32,
    longest: u32,
    blocks: u32,
    blocks_with_dialogue: u32,
}

fn is_opener(c: char) -> bool {
    matches!(c, '“' | '「' | '『' | '"')
}

fn is_closer(c: char) -> bool {
    matches!(c, '”' | '」' | '』' | '"')
}

/// Record a span unless it is empty, so an empty pair of quotes is not a line of speech.
fn push_span(spans: &mut Vec<Span>, chars: &[char], start: usize, end: usize, kind: &'static str) {
    if end <= start {
        return;
    }
    let visible = chars[start..end]
        .iter()
        .filter(|c| !c.is_whitespace())
        .count();
    if visible == 0 {
        return;
    }
    spans.push(Span {
        start,
        end,
        chars: visible,
        kind,
    });
}

/// Find the speech between quotation marks.
///
/// A quote left open at a line break is closed at that break: a long speech written as
/// several paragraphs opens each of them with `“` and closes only the last, so letting
/// the span run on would swallow the narration between those paragraphs.
fn quote_spans(chars: &[char]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut open: Option<usize> = None;

    for (i, &c) in chars.iter().enumerate() {
        match open {
            Some(start) => {
                // Closed by its mark, or by the end of the line it was opened on.
                if is_closer(c) || c == '\n' {
                    push_span(&mut spans, chars, start, i, "quote");
                    open = None;
                }
            }
            None => {
                if is_opener(c) {
                    open = Some(i + 1);
                }
            }
        }
    }

    // An opener with no closer anywhere runs to the end of the block: the text said
    // something was spoken, and dropping the tail would hide it.
    if let Some(start) = open {
        push_span(&mut spans, chars, start, chars.len(), "quote");
    }
    spans
}

/// Find the speech that wears no quotes: a line opened with `—` or `——`.
///
/// Only lines no quotation span already covers are counted, so a quoted line that
/// happens to start with a dash is not tallied twice.
fn dash_spans(chars: &[char], covered: &[Span]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut line_start = 0usize;

    loop {
        let line_end = chars[line_start..]
            .iter()
            .position(|c| *c == '\n')
            .map(|p| line_start + p)
            .unwrap_or(chars.len());

        let mut pos = line_start;
        while pos < line_end && chars[pos].is_whitespace() {
            pos += 1;
        }
        let mut dash_end = pos;
        while dash_end < line_end && (chars[dash_end] == '—' || chars[dash_end] == '–') {
            dash_end += 1;
        }
        if dash_end > pos {
            let overlaps = covered.iter().any(|s| s.start < line_end && s.end > pos);
            if !overlaps {
                push_span(&mut spans, chars, dash_end, line_end, "dash");
            }
        }

        if line_end == chars.len() {
            break;
        }
        line_start = line_end + 1;
    }
    spans
}

/// Every piece of speech in one block, in document order.
fn scan_spans(content: &str) -> Vec<Span> {
    let chars: Vec<char> = content.chars().collect();
    let mut spans = quote_spans(&chars);
    spans.extend(dash_spans(&chars, &spans));
    spans.sort_by_key(|s| s.start);
    spans
}

/// The first [`EXCERPT_RADIUS`] characters of one piece of speech.
fn excerpt(chars: &[char], span: &Span) -> String {
    let end = span.end.min(span.start + EXCERPT_RADIUS);
    let mut text: String = chars[span.start..end].iter().collect();
    if end < span.end {
        text.push('…');
    }
    text
}

/// How much of the text is spoken. `none` says no speech was *detected*, which is not
/// the same as no speech: the source may use no quotation marks at all.
fn verdict(share: f64, unit_count: u32) -> &'static str {
    if unit_count == 0 {
        "none"
    } else if share < SPARSE_SHARE {
        "sparse"
    } else if share > HEAVY_SHARE {
        "heavy"
    } else {
        "balanced"
    }
}

/// Fixed buckets, so two projects' distributions line up row for row.
fn buckets(units: &[u32]) -> Vec<(String, u32)> {
    const RANGES: [(&str, u32, u32); 5] = [
        ("1-9", 1, 9),
        ("10-29", 10, 29),
        ("30-99", 30, 99),
        ("100-299", 100, 299),
        ("300+", 300, u32::MAX),
    ];
    RANGES
        .iter()
        .map(|(label, lo, hi)| {
            let count = units.iter().filter(|u| **u >= *lo && **u <= *hi).count() as u32;
            (label.to_string(), count)
        })
        .collect()
}

/// Longest run of consecutive blocks meeting a predicate, and the visible characters
/// inside that run.
fn longest_run(flags: &[bool], chars: &[u64]) -> (u32, u64) {
    let mut best_len = 0u32;
    let mut best_chars = 0u64;
    let mut run_len = 0u32;
    let mut run_chars = 0u64;

    for (i, flag) in flags.iter().enumerate() {
        if *flag {
            run_len += 1;
            run_chars += chars[i];
        } else {
            if run_len > best_len || (run_len == best_len && run_chars > best_chars) {
                best_len = run_len;
                best_chars = run_chars;
            }
            run_len = 0;
            run_chars = 0;
        }
    }
    if run_len > best_len || (run_len == best_len && run_chars > best_chars) {
        best_len = run_len;
        best_chars = run_chars;
    }
    (best_len, best_chars)
}

/// Share of a block's characters that is spoken.
fn block_share(facts: &BlockFacts) -> f64 {
    if facts.chars == 0 {
        0.0
    } else {
        facts.dialogue_chars as f64 / facts.chars as f64
    }
}

/// One detected piece of speech, placed in the document.
#[derive(Debug, Clone)]
struct Speech {
    segment: usize,
    chars: u32,
    kind: &'static str,
    excerpt: String,
}

/// Shapes the mix tends to produce: whole-document ones first, then the per-segment and
/// per-speech findings in reading order.
fn find_issues(
    segments: &[Segment],
    speeches: &[Speech],
    labels: &[String],
    stats: &DialogueStats,
) -> Vec<DialogueIssue> {
    let mut issues: Vec<DialogueIssue> = Vec::new();

    if stats.unit_count == 0 && stats.char_count >= MIN_CHARS_FOR_QUIET {
        issues.push(DialogueIssue {
            kind: "no_dialogue".into(),
            section: String::new(),
            detail: format!(
                "全文 {} 字没有检测到对白：可能确实无人开口，也可能是原文没有使用引号",
                stats.char_count
            ),
        });
    }

    if stats.quoted_units == 0 && stats.dash_units >= MIN_DASH_UNITS {
        issues.push(DialogueIssue {
            kind: "unquoted".into(),
            section: String::new(),
            detail: format!(
                "{} 处对白全部以破折号开头，一处引号也没有：引号用法不一致，或导入时丢了引号",
                stats.dash_units
            ),
        });
    }

    if stats.unit_count >= MIN_TELEGRAPHIC_UNITS
        && stats.avg_unit_chars < TELEGRAPHIC_AVG
        && stats.verdict != "none"
    {
        issues.push(DialogueIssue {
            kind: "telegraphic".into(),
            section: String::new(),
            detail: format!(
                "{} 处对白平均每处 {:.1} 字：一句接一句地说，中间几乎没有叙述",
                stats.unit_count, stats.avg_unit_chars
            ),
        });
    }

    if stats.spread >= UNEVEN_SPREAD && stats.segment_count >= 3 {
        issues.push(DialogueIssue {
            kind: "uneven".into(),
            section: String::new(),
            detail: format!(
                "各段对白占比相差 {:.0}%：最密 {}（{:.0}%）、最疏 {}（{:.0}%）",
                stats.spread * 100.0,
                if stats.peak_section.is_empty() {
                    "—".to_string()
                } else {
                    stats.peak_section.clone()
                },
                stats.peak_share * 100.0,
                if stats.quietest_section.is_empty() {
                    "—".to_string()
                } else {
                    stats.quietest_section.clone()
                },
                stats.quietest_share * 100.0
            ),
        });
    }

    if stats.longest_quiet_run >= QUIET_RUN_BLOCKS {
        issues.push(DialogueIssue {
            kind: "silent_segment".into(),
            section: String::new(),
            detail: format!(
                "连续 {} 个文本块、{} 字没有对白",
                stats.longest_quiet_run, stats.longest_quiet_chars
            ),
        });
    }

    // Per-segment shapes, in reading order.
    for s in segments {
        if s.chars < MIN_SEGMENT_CHARS {
            continue;
        }
        let share = s.dialogue_chars as f64 / s.chars.max(1) as f64;
        if share < SILENT_SHARE {
            issues.push(DialogueIssue {
                kind: "silent_segment".into(),
                section: s.label.clone(),
                detail: format!("本段 {} 字，对白只占 {:.0}%", s.chars, share * 100.0),
            });
        } else if share > FLOOD_SHARE {
            issues.push(DialogueIssue {
                kind: "flooded_segment".into(),
                section: s.label.clone(),
                detail: format!(
                    "本段 {} 字，对白占 {:.0}%：几乎没有叙述落脚",
                    s.chars,
                    share * 100.0
                ),
            });
        }
    }

    // Long speeches, in reading order.
    for sp in speeches {
        if sp.chars >= MONOLOGUE_CHARS {
            issues.push(DialogueIssue {
                kind: "monologue".into(),
                section: labels[sp.segment].clone(),
                detail: format!("{} 字的一段话：「{}」", sp.chars, sp.excerpt),
            });
        }
    }

    issues.truncate(MAX_ISSUES);
    issues
}

/// Count what is spoken and what is narrated, segment by segment.
pub fn run_dialogue_analysis(doc: &DocumentData) -> Result<DialogueStats> {
    let plan = plan_segments(doc);

    // Per-block facts, and every piece of speech with its segment attached.
    let mut facts: Vec<BlockFacts> = Vec::with_capacity(doc.blocks.len());
    let mut speeches: Vec<Speech> = Vec::new();
    for (idx, block) in doc.blocks.iter().enumerate() {
        let chars: Vec<char> = block.content.chars().collect();
        let spans = scan_spans(&block.content);
        let mut f = BlockFacts {
            chars: visible_char_count(&block.content) as u64,
            spans: spans.clone(),
            ..Default::default()
        };
        for sp in &spans {
            f.dialogue_chars += sp.chars as u64;
            match sp.kind {
                "quote" => f.quoted_units += 1,
                _ => f.dash_units += 1,
            }
            speeches.push(Speech {
                segment: plan.segment_of[idx],
                chars: sp.chars as u32,
                kind: sp.kind,
                excerpt: excerpt(&chars, sp),
            });
        }
        facts.push(f);
    }

    let mut segments: Vec<Segment> = plan
        .labels
        .iter()
        .map(|label| Segment {
            label: label.clone(),
            ..Default::default()
        })
        .collect();
    for (idx, f) in facts.iter().enumerate() {
        let s = &mut segments[plan.segment_of[idx]];
        s.chars += f.chars;
        s.dialogue_chars += f.dialogue_chars;
        s.units += f.spans.len() as u32;
        s.longest = s
            .longest
            .max(f.spans.iter().map(|sp| sp.chars as u32).max().unwrap_or(0));
        s.blocks += 1;
        if !f.spans.is_empty() {
            s.blocks_with_dialogue += 1;
        }
    }

    let char_count: u64 = segments.iter().map(|s| s.chars).sum();
    let dialogue_chars: u64 = segments.iter().map(|s| s.dialogue_chars).sum();
    let narrative_chars = char_count.saturating_sub(dialogue_chars);
    let dialogue_share = if char_count == 0 {
        0.0
    } else {
        dialogue_chars as f64 / char_count as f64
    };

    let unit_count = speeches.len() as u32;
    let lengths: Vec<u32> = speeches.iter().map(|s| s.chars).collect();
    let avg_unit_chars = if unit_count == 0 {
        0.0
    } else {
        lengths.iter().sum::<u32>() as f64 / unit_count as f64
    };
    let longest_unit = lengths.iter().max().copied().unwrap_or(0);

    // Runs are measured over blocks, not segments: a quiet stretch is about the reading
    // distance the text puts between two lines of speech. An empty block counts as
    // neither side — it carries no speech, but it also carries no distance.
    let block_chars: Vec<u64> = facts.iter().map(|f| f.chars).collect();
    let dominant: Vec<bool> = facts
        .iter()
        .map(|f| !f.spans.is_empty() && block_share(f) >= DOMINANT_SHARE)
        .collect();
    let quiet: Vec<bool> = facts
        .iter()
        .map(|f| f.spans.is_empty() && f.chars > 0)
        .collect();
    let (longest_dialogue_run, _) = longest_run(&dominant, &block_chars);
    let (longest_quiet_run, longest_quiet_chars) = longest_run(&quiet, &block_chars);

    let points: Vec<DialoguePoint> = segments
        .iter()
        .map(|s| {
            let share = if s.chars == 0 {
                0.0
            } else {
                s.dialogue_chars as f64 / s.chars as f64
            };
            DialoguePoint {
                section: s.label.clone(),
                char_count: s.chars,
                dialogue_chars: s.dialogue_chars,
                share,
                intensity: share * 100.0,
                units: s.units,
                longest_unit: s.longest,
                blocks: s.blocks,
                blocks_with_dialogue: s.blocks_with_dialogue,
            }
        })
        .collect();

    // Only segments with enough text take part in the spread, the peak and the quietest:
    // a 40-character fragment holding one quoted word would otherwise out-rank a chapter.
    let measured: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.chars >= MIN_SEGMENT_CHARS)
        .collect();
    let share_of = |s: &Segment| s.dialogue_chars as f64 / s.chars.max(1) as f64;
    let silent_segments = measured
        .iter()
        .filter(|s| share_of(s) < SILENT_SHARE)
        .count() as u32;
    let flooded_segments = measured
        .iter()
        .filter(|s| share_of(s) > FLOOD_SHARE)
        .count() as u32;

    let (peak_section, peak_share, quietest_section, quietest_share, spread) =
        if measured.is_empty() {
            (String::new(), 0.0, String::new(), 0.0, 0.0)
        } else {
            // `max_by` keeps the last of equal shares and `min_by` the first, so the
            // winner is decided by segment order rather than by iteration chance.
            let peak = measured
                .iter()
                .max_by(|a, b| {
                    share_of(a)
                        .partial_cmp(&share_of(b))
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .expect("measured is not empty");
            let quietest = measured
                .iter()
                .min_by(|a, b| {
                    share_of(a)
                        .partial_cmp(&share_of(b))
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .expect("measured is not empty");
            (
                peak.label.clone(),
                share_of(peak),
                quietest.label.clone(),
                share_of(quietest),
                share_of(peak) - share_of(quietest),
            )
        };

    let mut ordered: Vec<Speech> = speeches.clone();
    // Stable sort: pieces of equal length keep their document order.
    ordered.sort_by(|a, b| b.chars.cmp(&a.chars));

    let mut stats = DialogueStats {
        points,
        segment_count: segments.len() as u32,
        chunked: plan.chunked,
        char_count,
        dialogue_chars,
        narrative_chars,
        dialogue_share,
        narrative_share: 1.0 - dialogue_share,
        unit_count,
        quoted_units: facts.iter().map(|f| f.quoted_units).sum(),
        dash_units: facts.iter().map(|f| f.dash_units).sum(),
        avg_unit_chars,
        longest_unit,
        unit_length_distribution: buckets(&lengths),
        blocks: facts.len() as u32,
        blocks_with_dialogue: facts.iter().filter(|f| !f.spans.is_empty()).count() as u32,
        longest_dialogue_run,
        longest_quiet_run,
        longest_quiet_chars,
        silent_segments,
        flooded_segments,
        verdict: verdict(dialogue_share, unit_count).to_string(),
        spread,
        peak_section,
        peak_share,
        quietest_section,
        quietest_share,
        samples: ordered
            .iter()
            .take(MAX_SAMPLES)
            .map(|s| DialogueSample {
                section: plan.labels[s.segment].clone(),
                chars: s.chars,
                kind: s.kind.to_string(),
                excerpt: s.excerpt.clone(),
            })
            .collect(),
        issues: Vec::new(),
    };
    // The issues read the stats rather than a parameter list: every number they quote is
    // one the caller can see in the output.
    stats.issues = find_issues(&segments, &speeches, &plan.labels, &stats);
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

    fn para_at(path: &str, content: &str) -> SemanticBlock {
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

    fn para(content: &str) -> SemanticBlock {
        para_at("第一章", content)
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
    fn test_quoted_speech_is_counted() {
        let s = run_dialogue_analysis(&doc_of(vec![para("他说：“你好。”然后走了。")])).unwrap();
        // 你好。 inside the marks; the marks and the colon fall on the narrative side.
        assert_eq!(s.dialogue_chars, 3);
        assert_eq!(s.char_count, 13);
        assert_eq!(s.narrative_chars, 10);
        assert_eq!(s.unit_count, 1);
        assert_eq!(s.quoted_units, 1);
        assert_eq!(s.dash_units, 0);
        assert!((s.dialogue_share - 3.0 / 13.0).abs() < 1e-9);
        assert_eq!(s.verdict, "balanced");
        assert_eq!(s.samples[0].excerpt, "你好。");
    }

    #[test]
    fn test_dash_lines_count_as_dialogue() {
        let s = run_dialogue_analysis(&doc_of(vec![para("——你来了。\n他点头。")])).unwrap();
        assert_eq!(s.unit_count, 1);
        assert_eq!(s.dash_units, 1);
        assert_eq!(s.quoted_units, 0);
        assert_eq!(s.dialogue_chars, 4);
        assert!((s.dialogue_share - 0.4).abs() < 1e-9);
        assert_eq!(s.samples[0].kind, "dash");
    }

    #[test]
    fn test_dash_line_already_covered_by_quotes_is_not_counted_twice() {
        let s = run_dialogue_analysis(&doc_of(vec![para("——“你来了。”他说。")])).unwrap();
        assert_eq!(s.unit_count, 1);
        assert_eq!(s.quoted_units, 1);
        assert_eq!(s.dash_units, 0);
        assert_eq!(s.dialogue_chars, 4);
    }

    #[test]
    fn test_long_speech_is_cut_at_the_line_break_not_run_on() {
        // The multi-paragraph convention: each paragraph opens with “, only the last
        // closes. One span across them would call the narration between them dialogue.
        let s =
            run_dialogue_analysis(&doc_of(vec![para("“第一句。\n他停下来。“第二句。”")])).unwrap();
        assert_eq!(s.unit_count, 2);
        assert_eq!(s.dialogue_chars, 8);
        assert_eq!(s.samples[0].excerpt, "第一句。");
        assert_eq!(s.samples[1].excerpt, "第二句。");
    }

    #[test]
    fn test_unclosed_quote_runs_to_the_end_of_the_block() {
        let s = run_dialogue_analysis(&doc_of(vec![para("她说：“没有下文")])).unwrap();
        assert_eq!(s.unit_count, 1);
        assert_eq!(s.dialogue_chars, 4);
    }

    #[test]
    fn test_empty_quotes_are_not_speech() {
        let s = run_dialogue_analysis(&doc_of(vec![para("他说：“”然后走了")])).unwrap();
        assert_eq!(s.unit_count, 0);
        assert_eq!(s.dialogue_chars, 0);
        assert_eq!(s.verdict, "none");
    }

    #[test]
    fn test_narration_without_speech_says_so() {
        let text = "他走在长街上，脚步很慢。".repeat(20);
        let s = run_dialogue_analysis(&doc_of(vec![para(&text)])).unwrap();
        assert_eq!(s.unit_count, 0);
        assert_eq!(s.verdict, "none");
        assert_eq!(s.dialogue_share, 0.0);
        assert!(
            s.issues.iter().any(|i| i.kind == "no_dialogue"),
            "a long text with no speech should say so"
        );
    }

    #[test]
    fn test_the_two_sides_add_up_to_the_text() {
        let s = run_dialogue_analysis(&doc_of(vec![para("他说：“你好。”然后走了。")])).unwrap();
        assert_eq!(s.dialogue_chars + s.narrative_chars, s.char_count);
        assert!((s.dialogue_share + s.narrative_share - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_empty_document_produces_nothing() {
        let s = run_dialogue_analysis(&doc_of(vec![])).unwrap();
        assert_eq!(s.segment_count, 0);
        assert!(s.points.is_empty());
        assert_eq!(s.char_count, 0);
        assert_eq!(s.verdict, "none");
        assert!(s.issues.is_empty());
    }

    /// 12 characters of plain narration, repeated to a usable length.
    fn narration(times: usize) -> String {
        "他走在长街上，脚步很慢。".repeat(times)
    }

    /// Speech of `times` × 7 characters, wrapped in quotation marks.
    fn speech(times: usize) -> String {
        format!("“{}”", "他说了一句话。".repeat(times))
    }

    #[test]
    fn test_the_curve_follows_the_sections() {
        let doc = doc_of(vec![
            para_at("第一章", &speech(45)),
            para_at("第二章", &format!("{}{}", narration(13), speech(22))),
            para_at("第三章", &narration(26)),
        ]);
        let s = run_dialogue_analysis(&doc).unwrap();

        assert_eq!(s.segment_count, 3);
        assert!(!s.chunked);
        assert_eq!(s.points.len(), 3);
        assert_eq!(s.points[0].section, "第一章");
        assert_eq!(s.points[0].units, 1);
        assert_eq!(s.points[2].units, 0);
        assert_eq!(s.points[2].dialogue_chars, 0);
        // The curve is the share itself on a 0-100 scale: a segment that is all speech
        // reads as 100, one with none as 0.
        assert!(s.points[0].intensity > 99.0);
        assert_eq!(s.points[2].intensity, 0.0);

        assert_eq!(s.peak_section, "第一章");
        assert_eq!(s.quietest_section, "第三章");
        assert!(s.spread > 0.9);
        assert_eq!(s.flooded_segments, 1);
        assert_eq!(s.silent_segments, 1);
        assert!(
            s.issues.iter().any(|i| i.kind == "uneven"),
            "one loud chapter and one silent chapter is a mix worth naming"
        );
    }

    #[test]
    fn test_segments_too_short_to_judge_are_left_out() {
        let s = run_dialogue_analysis(&doc_of(vec![para(&format!(
            "{}{}",
            narration(10),
            speech(1)
        ))]))
        .unwrap();

        assert_eq!(s.unit_count, 1);
        assert_eq!(s.peak_section, "");
        assert_eq!(s.quietest_section, "");
        assert_eq!(s.spread, 0.0);
        assert_eq!(s.silent_segments, 0);
        assert_eq!(s.flooded_segments, 0);
        assert!(!s.issues.iter().any(|i| i.kind == "uneven"));
    }

    #[test]
    fn test_a_long_quiet_stretch_is_measured_in_blocks_and_characters() {
        let blocks = (0..8).map(|_| para(&narration(4))).collect();
        let s = run_dialogue_analysis(&doc_of(blocks)).unwrap();

        assert_eq!(s.longest_quiet_run, 8);
        assert_eq!(s.longest_quiet_chars, 384);
        let issue = s
            .issues
            .iter()
            .find(|i| i.kind == "silent_segment")
            .expect("eight blocks without one line of speech");
        assert!(issue.detail.contains("8 个文本块"), "{issue:?}");
    }

    #[test]
    fn test_a_run_of_speech_is_measured() {
        let blocks = (0..4)
            .map(|_| para(&format!("他说：{}", speech(4))))
            .collect();
        let s = run_dialogue_analysis(&doc_of(blocks)).unwrap();
        assert_eq!(s.longest_dialogue_run, 4);
        assert_eq!(s.blocks_with_dialogue, 4);
    }

    #[test]
    fn test_a_long_speech_is_flagged_as_a_monologue() {
        let doc = doc_of(vec![
            para_at("第一章", &narration(4)),
            para_at("第二章", &speech(46)),
            para_at("第三章", &narration(4)),
        ]);
        let s = run_dialogue_analysis(&doc).unwrap();
        let issue = s
            .issues
            .iter()
            .find(|i| i.kind == "monologue")
            .expect("a 322-character speech");
        assert_eq!(issue.section, "第二章");
        assert!(issue.detail.starts_with("322 字的一段话"), "{issue:?}");
    }

    #[test]
    fn test_telegraphic_speech_is_flagged() {
        let blocks = (0..24).map(|_| para("他说：“好。”")).collect();
        let s = run_dialogue_analysis(&doc_of(blocks)).unwrap();

        assert_eq!(s.unit_count, 24);
        assert!((s.avg_unit_chars - 2.0).abs() < 1e-9);
        assert!(
            s.issues.iter().any(|i| i.kind == "telegraphic"),
            "24 lines of two characters each is a rhythm, not a mix"
        );
        assert_eq!(s.unit_length_distribution[0], ("1-9".to_string(), 24));
    }

    #[test]
    fn test_speech_without_quotation_marks_is_flagged() {
        let blocks = (0..12).map(|_| para("——他来了。")).collect();
        let s = run_dialogue_analysis(&doc_of(blocks)).unwrap();

        assert_eq!(s.dash_units, 12);
        assert_eq!(s.quoted_units, 0);
        assert!(
            s.issues.iter().any(|i| i.kind == "unquoted"),
            "all speech unquoted is worth telling the reader about"
        );
    }

    #[test]
    fn test_samples_are_reported_longest_first() {
        let doc = doc_of(vec![para(&speech(2)), para(&speech(6)), para(&speech(4))]);
        let s = run_dialogue_analysis(&doc).unwrap();

        assert_eq!(s.samples.len(), 3);
        assert_eq!(s.samples[0].chars, 42);
        assert_eq!(s.samples[1].chars, 28);
        assert_eq!(s.samples[2].chars, 14);
        assert_eq!(s.longest_unit, 42);
    }

    #[test]
    fn test_the_distribution_is_fixed_buckets() {
        let doc = doc_of(vec![
            para("他说：“好。”"),
            para(&speech(3)),
            para(&speech(20)),
        ]);
        let s = run_dialogue_analysis(&doc).unwrap();

        let labels: Vec<&str> = s
            .unit_length_distribution
            .iter()
            .map(|(l, _)| l.as_str())
            .collect();
        assert_eq!(labels, ["1-9", "10-29", "30-99", "100-299", "300+"]);
        let total: u32 = s.unit_length_distribution.iter().map(|(_, c)| c).sum();
        assert_eq!(total, s.unit_count);
    }

    #[test]
    fn test_the_same_text_gives_the_same_numbers() {
        let doc = doc_of(vec![
            para_at("第一章", &speech(12)),
            para_at("第二章", &narration(20)),
            para_at("第三章", &format!("{}{}", narration(8), speech(9))),
        ]);
        let a = run_dialogue_analysis(&doc).unwrap();
        let b = run_dialogue_analysis(&doc).unwrap();
        assert_eq!(format!("{:?}", a), format!("{:?}", b));
    }
}
