//! Explicit language (显式语言检测): what a reader might want warned about.
//!
//! The design doc asks for 敏感词库 + 上下文 + 分级, and that is what this module does.
//! It carries a curated library of terms in seven families — 粗口辱骂 / 性 / 性暴力 /
//! 暴力血腥 / 自伤 / 仇恨贬损 / 毒品 — finds where the text uses them, and reports each
//! hit with the characters around it so a person can decide what it means there.
//!
//! **This dimension has no score, on purpose.** A war novel is not worse than a
//! pastoral for carrying violence, and a romance is not worse than an essay for
//! carrying sex. What the pass measures is *presence and strength*, and what it offers
//! is an advisory level: the kind of thing a publisher or a platform puts a reader
//! warning in front of. Grading it would push an author toward writing milder books in
//! order to score better, which is the wrong instrument to hold to a manuscript.
//!
//! Three limits are stated in the output rather than hidden:
//! - Only library terms are found. Silence means the library found nothing, not that
//!   the content is gentle — 他从不动刀 is invisible here.
//! - A match is not an intent. 跳楼 may be a fall, 尸体 may be the detective's weekday,
//!   and the strongest word can sit in a line a villain says. The hit carries its
//!   context, so the scene stays a person's call.
//! - Strength here is a property of the wording, not of the event: 他死了 and
//!   他被割喉而死 report comparable events at very different strengths, and this pass
//!   can only see the wording.
//!
//! Deterministic: fixed library, segments in reading order, tiers in fixed order,
//! no clock and no randomness; the same document always yields the same hits and the
//! same advisory level.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;

use crate::lexicon::{self, Compiled, Index};
use crate::segment::plan_segments;
use crate::text::visible_char_count;
use crate::Result;

/// Hits per 10 000 visible characters at which a segment saturates the curve.
const EXPLICIT_FULL: f64 = 30.0;
/// A segment at or above this density is reported as a cluster.
const DENSE_SEGMENT_PER_10K: f64 = 20.0;
/// How many terms are reported, most used first.
const MAX_TERMS: usize = 20;
/// How many individual hits are reported, document order.
const MAX_HITS: usize = 60;
/// How many advisories are reported, strongest family first.
const MAX_ISSUES: usize = 12;

/// The term library: `pattern, category, tier, note`.
///
/// Patterns use the shared matcher's language (`lexicon.rs`): literal characters
/// plus one parenthesized alternation group, so `(该死|该死的)` is two forms of one
/// entry and is counted as one entry.
///
/// Every pattern is at least two characters — Chinese has no word boundaries, and a
/// one-character pattern like 杀 matches half of every novel. Where a term is common
/// in a harmless sense its tier is low and the note says what to check.
///
/// Categories: `profanity` | `sexual` | `sexual_violence` | `violence` | `self_harm` |
/// `hate` | `substance`. Tiers: `mild` | `moderate` | `strong`.
const EXPLICIT_LIBRARY: &[(&str, &str, &str, &str)] = &[
    // 粗口辱骂 — the most common kind of hit, and the least worth warning a reader about.
    (
        "(该死|该死的)",
        "profanity",
        "mild",
        "语气粗，一般不需要读者提示",
    ),
    (
        "(混账|混蛋|王八蛋|狗东西|畜生|狗屎)",
        "profanity",
        "mild",
        "骂人话；是否碍眼取决于人物与场合",
    ),
    (
        "(蠢货|白痴|废物|饭桶|窝囊废)",
        "profanity",
        "mild",
        "贬低人物的骂法；是否为人物口径需要人核对",
    ),
    (
        "(他妈的|妈的|特么的|丫的)",
        "profanity",
        "moderate",
        "常见粗口；密度高时语气会盖过内容",
    ),
    (
        "(不要脸|无耻|下贱|贱骨头)",
        "profanity",
        "moderate",
        "带贬损的骂法，密度高时读者会感到刺耳",
    ),
    (
        "(傻逼|龟孙|狗娘养的|狗日的|杂种)",
        "profanity",
        "strong",
        "强烈辱骂；出版前值得确认是否为角色语言",
    ),
    (
        "(我操|我艹|去他妈|你妈的|妈卖批)",
        "profanity",
        "strong",
        "强烈粗口；多数平台会按露骨语言处理",
    ),
    // 仇恨贬损 — targeted slurs are the family platforms ask about first.
    (
        "(婊子|荡妇|骚货|破鞋|淫妇|贱货)",
        "hate",
        "strong",
        "贬损性称呼，多指向女性；读者可能需要注意",
    ),
    (
        "(黑鬼|支那人|劣等民族|优等种族|人渣)",
        "hate",
        "strong",
        "歧视或去人化的用语；几乎总是需要提示",
    ),
    ("(贱人|贱婢|下等人)", "hate", "moderate", "等级化的蔑称"),
    (
        "(残废|瘸子|瞎子|聋子)",
        "hate",
        "mild",
        "对残障的旧称；可能是时代或人物用语，需要人判断",
    ),
    // 性 — tiered by how explicit the wording is, not by what the scene means.
    (
        "(接吻|亲吻|热吻|拥吻)",
        "sexual",
        "mild",
        "亲吻本身一般不需要提示",
    ),
    (
        "强吻",
        "sexual",
        "moderate",
        "非自愿的身体接触：读者可能需要注意",
    ),
    ("(裸体|赤裸|一丝不挂)", "sexual", "moderate", "裸露描写"),
    (
        "(脱衣|宽衣|解带|扒光)",
        "sexual",
        "moderate",
        "脱衣描写；露骨与否要看下文",
    ),
    (
        "(巫山云雨|云雨巫山|鱼水之欢|周公之礼|春宵)",
        "sexual",
        "moderate",
        "含蓄的性行为暗示",
    ),
    (
        "(卖淫|嫖娼|妓院|窑子|狎妓|陪睡)",
        "sexual",
        "moderate",
        "性交易相关描写",
    ),
    (
        "(做爱|性交|性爱|口交|自慰|性高潮)",
        "sexual",
        "strong",
        "露骨性行为描写：多数平台需要读者提示",
    ),
    (
        "(阴茎|阴道|阴部|生殖器|性器|乳头|乳房|精液)",
        "sexual",
        "strong",
        "性器官直陈：露骨度的主要信号",
    ),
    // 性暴力 — its own family, because the advisory it carries differs in kind.
    (
        "(强奸|性侵|轮奸|性骚扰|猥亵|乱伦)",
        "sexual_violence",
        "strong",
        "性暴力：多数读者需要提示",
    ),
    (
        "(迷奸|采花|逼奸)",
        "sexual_violence",
        "strong",
        "性暴力：通常需要读者预警",
    ),
    // 暴力血腥 — tiered by how much the wording shows.
    (
        "(打架|斗殴|推搡|扭打|耳光|拳打脚踢)",
        "violence",
        "mild",
        "一般冲突场面",
    ),
    (
        "(殴打|拷打|酷刑|虐待|流血|鲜血淋漓|砍杀|刺杀|枪杀)",
        "violence",
        "moderate",
        "暴力场面；密度决定是否需要提示",
    ),
    (
        "(杀人|凶杀|谋杀|毒杀|虐杀|尸体|死尸|凶案)",
        "violence",
        "moderate",
        "死亡与暴力：类型常见，看写法与密度",
    ),
    (
        "(屠杀|灭口|分尸|碎尸|肢解|开膛|挖眼|割喉|斩首)",
        "violence",
        "strong",
        "血腥细节：读者提示的主要依据",
    ),
    (
        "(凌迟|剥皮|活埋|人彘|剁碎|血溅五步)",
        "violence",
        "strong",
        "极端血腥描写",
    ),
    // 自伤 — one hit is worth surfacing, whatever the density.
    (
        "(自杀|寻死|轻生|自我了断|自残|割腕|上吊|服毒|烧炭)",
        "self_harm",
        "strong",
        "自伤或自杀描写：多数平台建议读者提示",
    ),
    (
        "(自杀未遂|企图自杀)",
        "self_harm",
        "strong",
        "自伤未遂同样值得提示",
    ),
    (
        "(吞安眠药|大量服药|注射过量)",
        "self_harm",
        "strong",
        "自伤方式的具体写法：细节本身可能需要提示",
    ),
    (
        "(跳楼|坠楼)",
        "self_harm",
        "moderate",
        "可能是坠落而非自伤：需要人核对上下文",
    ),
    (
        "(自焚|殉情|绝食)",
        "self_harm",
        "moderate",
        "自我伤害的选择：类型常见，看写法",
    ),
    // 毒品
    (
        "(喝醉|烂醉|酗酒|买醉)",
        "substance",
        "mild",
        "饮酒描写一般不需要提示",
    ),
    (
        "灌醉",
        "substance",
        "moderate",
        "让人失去意识：常与侵害相连，需要人核对上下文",
    ),
    (
        "(鸦片|大烟|烟馆|鸦片烟)",
        "substance",
        "moderate",
        "历史语境中的麻醉品",
    ),
    (
        "(毒品|吸毒|贩毒|海洛因|冰毒|摇头丸|大麻|可卡因|吗啡|致幻剂|戒毒)",
        "substance",
        "strong",
        "毒品描写：读者提示的常见依据",
    ),
];

/// One segment of the explicit-language curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitPoint {
    pub section: String,
    pub char_count: u64,
    /// Library hits in the segment.
    pub hits: u32,
    /// Hits per 10 000 visible characters.
    pub per_10k: f64,
    /// 0-100 on the absolute scale, so two projects' curves can be compared.
    pub intensity: f64,
    /// The term used most here; empty when the segment has none.
    pub top_term: String,
}

/// One library term and how much of it the document carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitTerm {
    /// The library pattern, e.g. `强奸|性侵|轮奸`.
    pub term: String,
    /// `profanity` | `sexual` | `sexual_violence` | `violence` | `self_harm` | `hate` | `substance`.
    pub category: String,
    /// `mild` | `moderate` | `strong`.
    pub tier: String,
    pub count: u32,
    /// Share of all hits this term accounts for (0-1).
    pub share: f64,
    /// The forms actually found, busiest first.
    pub variants: Vec<(String, u32)>,
    /// Curve segments that contain at least one hit.
    pub segments_touched: u32,
    /// Share of the curve's segments this term reaches (0-1).
    pub coverage: f64,
    pub first_section: String,
    pub last_section: String,
    /// What the library advises about it.
    pub note: String,
}

/// One place in the text where a library term was found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitHit {
    /// The curve segment the hit sits in.
    pub section: String,
    /// The text actually matched.
    pub term: String,
    /// The library pattern it came from.
    pub pattern: String,
    pub category: String,
    /// `mild` | `moderate` | `strong`.
    pub tier: String,
    /// Character offset inside the block.
    pub offset: usize,
    /// The surrounding characters, so the hit can be judged in context.
    pub excerpt: String,
    /// The hit is inside quotation marks: a character says it rather than narrates it.
    pub in_dialogue: bool,
    pub note: String,
}

/// A content family worth a reader advisory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitIssue {
    /// `self_harm` | `sexual_violence` | `sexual_content` | `gore` | `hate_slur` |
    /// `substance` | `dense_segment`.
    pub kind: String,
    pub section: String,
    pub detail: String,
}

/// Explicit-language analysis output for one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitStats {
    pub points: Vec<ExplicitPoint>,
    pub segment_count: u32,
    /// True when the segments are block chunks rather than sections.
    pub chunked: bool,
    /// Visible characters the rate is measured over.
    pub char_count: u64,
    pub total_hits: u32,
    /// Library patterns found at least once.
    pub distinct_terms: u32,
    /// Hits per 10 000 visible characters across the document.
    pub per_10k: f64,
    /// `none` | `mild` | `moderate` | `strong` — the strength of the words found,
    /// not a judgment on the text.
    pub advisory: String,
    /// Hit counts per tier, always all three tiers in this order.
    pub tiers: Vec<(String, u32)>,
    /// Categories present, most hits first.
    pub categories: Vec<(String, u32)>,
    /// Categories that carry at least one `strong` hit: what an advisory would name.
    pub advisory_categories: Vec<String>,
    /// Reported terms, most used first.
    pub terms: Vec<ExplicitTerm>,
    /// Reported hits, document order.
    pub hits: Vec<ExplicitHit>,
    /// Hits inside quotation marks.
    pub dialogue_hits: u32,
    /// Share of all hits held by the single most used term (0-1).
    pub top_share: f64,
    pub top_term: String,
    pub peak_section: String,
    pub peak_intensity: f64,
    pub issues: Vec<ExplicitIssue>,
    /// How many patterns the library carries — coverage the reader can weigh.
    pub lexicon_size: usize,
    /// How many surface forms those patterns expand to.
    pub lexicon_variants: usize,
}

/// A hit before it is placed in the output.
#[derive(Debug, Clone)]
struct RawHit {
    pattern_idx: usize,
    matched: String,
    char_offset: usize,
    excerpt: String,
    in_dialogue: bool,
    segment: usize,
}

/// Per-segment counters.
#[derive(Debug, Default)]
struct Segment {
    label: String,
    chars: u64,
    hits: u32,
    terms: HashMap<usize, u32>,
}

/// Per-term counters.
#[derive(Debug, Default)]
struct TermTally {
    count: u32,
    variants: HashMap<String, u32>,
    /// Variant texts in first-appearance order, so the list is not HashMap order.
    variant_order: Vec<String>,
    segments: HashMap<usize, u32>,
    first_segment: usize,
    last_segment: usize,
}

/// The term library, compiled for the shared matcher.
fn compile_library() -> Vec<Compiled> {
    lexicon::compile_graded(EXPLICIT_LIBRARY)
}

/// Scan every block, tagging each hit with the curve segment it falls in.
fn collect_hits(
    doc: &DocumentData,
    index: &Index,
    lexicon: &[Compiled],
) -> (Vec<Segment>, Vec<RawHit>, bool) {
    let plan = plan_segments(doc);
    let mut segments: Vec<Segment> = plan
        .labels
        .iter()
        .map(|label| Segment {
            label: label.clone(),
            ..Default::default()
        })
        .collect();

    let mut all_hits: Vec<RawHit> = Vec::new();

    for (block_idx, block) in doc.blocks.iter().enumerate() {
        // `segment_of` has one entry per block whenever there are blocks at all.
        let segment = plan.segment_of[block_idx];
        let chars = visible_char_count(&block.content) as u64;
        segments[segment].chars += chars;

        for found in lexicon::scan_block(&block.content, index, lexicon) {
            let hit = RawHit {
                pattern_idx: found.pattern_idx,
                matched: found.matched,
                char_offset: found.char_offset,
                excerpt: found.excerpt,
                in_dialogue: found.in_dialogue,
                segment,
            };
            let seg = &mut segments[segment];
            seg.hits += 1;
            *seg.terms.entry(hit.pattern_idx).or_insert(0) += 1;
            all_hits.push(hit);
        }
    }

    (segments, all_hits, plan.chunked)
}

/// The busiest library term inside one segment; ties go to the term that sorts first,
/// so the label never depends on HashMap order.
fn busiest_term(segment: &Segment, lexicon: &[Compiled]) -> String {
    let mut best: Option<(usize, u32)> = None;
    for (idx, count) in &segment.terms {
        if best.map(|(_, c)| *count > c).unwrap_or(true) {
            best = Some((*idx, *count));
        }
    }
    best.map(|(i, _)| lexicon[i].pattern.to_string())
        .unwrap_or_default()
}

/// Terms ranked by use, then by pattern text: two documents with the same counts list
/// the same terms in the same order.
fn rank_terms(tallies: &[TermTally], lexicon: &[Compiled]) -> Vec<(usize, u32)> {
    let mut ranked: Vec<(usize, u32)> = tallies
        .iter()
        .enumerate()
        .filter(|(_, t)| t.count > 0)
        .map(|(i, t)| (i, t.count))
        .collect();
    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| lexicon[a.0].pattern.cmp(lexicon[b.0].pattern))
    });
    ranked
}

/// One library term as the report shows it.
fn term_view(
    idx: usize,
    tally: &TermTally,
    lexicon: &[Compiled],
    segments: &[Segment],
    total_hits: u32,
    segment_count: u32,
) -> ExplicitTerm {
    let mut variants: Vec<(String, u32)> = tally
        .variant_order
        .iter()
        .map(|v| (v.clone(), tally.variants[v]))
        .collect();
    // Busiest first; ties keep the order the forms first appeared in.
    variants.sort_by(|a, b| {
        b.1.cmp(&a.1).then_with(|| {
            tally
                .variant_order
                .iter()
                .position(|v| v == &a.0)
                .cmp(&tally.variant_order.iter().position(|v| v == &b.0))
        })
    });

    ExplicitTerm {
        term: lexicon[idx].pattern.to_string(),
        category: lexicon[idx].category.to_string(),
        tier: lexicon[idx].tier.to_string(),
        count: tally.count,
        share: if total_hits == 0 {
            0.0
        } else {
            tally.count as f64 / total_hits as f64
        },
        variants,
        segments_touched: tally.segments.len() as u32,
        coverage: if segment_count == 0 {
            0.0
        } else {
            tally.segments.len() as f64 / segment_count as f64
        },
        first_section: segments[tally.first_segment].label.clone(),
        last_section: segments[tally.last_segment].label.clone(),
        note: lexicon[idx].note.to_string(),
    }
}

/// Where a tier ranks, so an entry's strength can be compared against a threshold.
fn tier_rank(tier: &str) -> u8 {
    match tier {
        "strong" => 3,
        "moderate" => 2,
        "mild" => 1,
        _ => 0,
    }
}

/// The advisory level is the strongest tier found, because one strong passage is what
/// a warning is written for. `none` says the library found nothing — it does not say
/// the content is gentle.
fn advisory(mild: u32, moderate: u32, strong: u32) -> &'static str {
    if strong > 0 {
        "strong"
    } else if moderate > 0 {
        "moderate"
    } else if mild > 0 {
        "mild"
    } else {
        "none"
    }
}

/// The families worth naming in an advisory, in the order they should be named, with
/// the tier at which the family starts to say something: a moderate 打架 is an ordinary
/// scene while a strong 割喉 is not, and 自伤 is worth surfacing at moderate because it
/// is a family where one hit matters more than the rate.
///
/// `category, issue kind, threshold tier, label`.
const ADVISORY_FAMILIES: &[(&str, &str, &str, &str)] = &[
    ("self_harm", "self_harm", "moderate", "自伤或自杀描写"),
    ("sexual_violence", "sexual_violence", "strong", "性暴力用语"),
    ("hate", "hate_slur", "strong", "贬损或歧视用语"),
    ("sexual", "sexual_content", "strong", "露骨性描写用词"),
    ("violence", "gore", "strong", "血腥细节描写"),
    ("substance", "substance", "strong", "毒品相关用词"),
];

/// What the document carries an advisory for, strongest family first, then the segments
/// that sit far above the document's own rate.
fn find_issues(
    segments: &[Segment],
    all_hits: &[RawHit],
    lexicon: &[Compiled],
    per_10k: f64,
) -> Vec<ExplicitIssue> {
    let mut issues: Vec<ExplicitIssue> = Vec::new();

    for (category, kind, threshold, label) in ADVISORY_FAMILIES {
        let matching: Vec<&RawHit> = all_hits
            .iter()
            .filter(|h| {
                let entry = &lexicon[h.pattern_idx];
                entry.category == *category && tier_rank(entry.tier) >= tier_rank(threshold)
            })
            .collect();
        if matching.is_empty() {
            continue;
        }

        // Where the family sits thickest, so the advisory points at a place.
        let mut per_segment: HashMap<usize, u32> = HashMap::new();
        for h in &matching {
            *per_segment.entry(h.segment).or_insert(0) += 1;
        }
        let mut ranked: Vec<(usize, u32)> = per_segment.into_iter().collect();
        ranked.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| segments[a.0].label.cmp(&segments[b.0].label))
        });
        let (segment_idx, count) = ranked[0];

        issues.push(ExplicitIssue {
            kind: (*kind).to_string(),
            section: segments[segment_idx].label.clone(),
            detail: format!(
                "{} 处{}，最集中于 {}（{} 处）",
                matching.len(),
                label,
                segments[segment_idx].label,
                count
            ),
        });
    }

    // A segment far above the document's own rate is worth naming whatever the mix: a
    // scene where everything arrives at once reads differently from the book around it.
    for s in segments {
        let chars = s.chars.max(1) as f64;
        let seg_per_10k = s.hits as f64 / chars * 10000.0;
        if seg_per_10k >= DENSE_SEGMENT_PER_10K {
            issues.push(ExplicitIssue {
                kind: "dense_segment".to_string(),
                section: s.label.clone(),
                detail: format!(
                    "每万字 {:.0} 处（{} 处 / {} 字），全文平均 {:.0}",
                    seg_per_10k, s.hits, s.chars, per_10k
                ),
            });
        }
    }

    issues.truncate(MAX_ISSUES);
    issues
}

/// Count what the text says that a reader might want warned about.
///
/// The output carries no score: presence and strength are reported, and what to do
/// about them — cut a detail, add a warning, leave it alone — is the author's call.
pub fn run_explicit_analysis(doc: &DocumentData) -> Result<ExplicitStats> {
    let lexicon = compile_library();
    let index = lexicon::build_index(&lexicon);
    let (segments, all_hits, chunked) = collect_hits(doc, &index, &lexicon);

    let total_hits = all_hits.len() as u32;
    let total_chars: u64 = segments.iter().map(|s| s.chars).sum();
    let per_10k = if total_chars == 0 {
        0.0
    } else {
        total_hits as f64 / total_chars as f64 * 10000.0
    };
    let segment_count = segments.len() as u32;

    // Per-term tallies, indexed by position in the library, so the order is the
    // library's own rather than a HashMap's.
    let mut tallies: Vec<TermTally> = (0..lexicon.len()).map(|_| TermTally::default()).collect();
    for hit in &all_hits {
        let t = &mut tallies[hit.pattern_idx];
        if t.count == 0 {
            t.first_segment = hit.segment;
        }
        t.count += 1;
        t.last_segment = hit.segment;
        *t.variants.entry(hit.matched.clone()).or_insert(0) += 1;
        if !t.variant_order.contains(&hit.matched) {
            t.variant_order.push(hit.matched.clone());
        }
        *t.segments.entry(hit.segment).or_insert(0) += 1;
    }

    let ranked = rank_terms(&tallies, &lexicon);
    let terms: Vec<ExplicitTerm> = ranked
        .iter()
        .take(MAX_TERMS)
        .map(|&(idx, _)| {
            term_view(
                idx,
                &tallies[idx],
                &lexicon,
                &segments,
                total_hits,
                segment_count,
            )
        })
        .collect();

    // Tiers in a fixed order: the advisory level reads off these counts, and an absent
    // tier still has to appear so the list never changes shape between documents.
    let tiers: Vec<(String, u32)> = ["mild", "moderate", "strong"]
        .iter()
        .map(|tier| {
            let count = all_hits
                .iter()
                .filter(|h| lexicon[h.pattern_idx].tier == *tier)
                .count() as u32;
            (tier.to_string(), count)
        })
        .collect();
    let (mild, moderate, strong) = (tiers[0].1, tiers[1].1, tiers[2].1);

    let mut category_counts: HashMap<&str, u32> = HashMap::new();
    for (idx, count) in &ranked {
        *category_counts.entry(lexicon[*idx].category).or_insert(0) += count;
    }
    let mut categories: Vec<(String, u32)> = category_counts
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    categories.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Which families carry strong material — the list an advisory would be written from.
    let mut strong_counts: HashMap<&str, u32> = HashMap::new();
    for hit in &all_hits {
        let entry = &lexicon[hit.pattern_idx];
        if entry.tier == "strong" {
            *strong_counts.entry(entry.category).or_insert(0) += 1;
        }
    }
    let mut advisory_categories: Vec<String> =
        strong_counts.keys().map(|k| k.to_string()).collect();
    advisory_categories.sort_by(|a, b| {
        strong_counts[b.as_str()]
            .cmp(&strong_counts[a.as_str()])
            .then_with(|| a.cmp(b))
    });

    let points: Vec<ExplicitPoint> = segments
        .iter()
        .map(|s| {
            let chars = s.chars.max(1) as f64;
            let seg_per_10k = s.hits as f64 / chars * 10000.0;
            ExplicitPoint {
                section: s.label.clone(),
                char_count: s.chars,
                hits: s.hits,
                per_10k: seg_per_10k,
                intensity: (seg_per_10k / EXPLICIT_FULL).min(1.0) * 100.0,
                top_term: busiest_term(s, &lexicon),
            }
        })
        .collect();

    let (peak_section, peak_intensity) = points
        .iter()
        .fold(None::<(String, f64)>, |acc, p| {
            if acc.as_ref().map(|(_, v)| p.intensity > *v).unwrap_or(true) {
                Some((p.section.clone(), p.intensity))
            } else {
                acc
            }
        })
        .unwrap_or_default();

    let hits: Vec<ExplicitHit> = all_hits
        .iter()
        .take(MAX_HITS)
        .map(|h| ExplicitHit {
            section: segments[h.segment].label.clone(),
            term: h.matched.clone(),
            pattern: lexicon[h.pattern_idx].pattern.to_string(),
            category: lexicon[h.pattern_idx].category.to_string(),
            tier: lexicon[h.pattern_idx].tier.to_string(),
            offset: h.char_offset,
            excerpt: h.excerpt.clone(),
            in_dialogue: h.in_dialogue,
            note: lexicon[h.pattern_idx].note.to_string(),
        })
        .collect();

    let dialogue_hits = all_hits.iter().filter(|h| h.in_dialogue).count() as u32;
    let (top_share, top_term) = ranked
        .first()
        .map(|(idx, count)| {
            (
                if total_hits == 0 {
                    0.0
                } else {
                    *count as f64 / total_hits as f64
                },
                lexicon[*idx].pattern.to_string(),
            )
        })
        .unwrap_or_default();

    Ok(ExplicitStats {
        points,
        segment_count,
        chunked,
        char_count: total_chars,
        total_hits,
        distinct_terms: ranked.len() as u32,
        per_10k,
        advisory: advisory(mild, moderate, strong).to_string(),
        tiers,
        categories,
        advisory_categories,
        terms,
        hits,
        dialogue_hits,
        top_share,
        top_term,
        peak_section,
        peak_intensity,
        issues: find_issues(&segments, &all_hits, &lexicon, per_10k),
        lexicon_size: EXPLICIT_LIBRARY.len(),
        lexicon_variants: lexicon.iter().map(|e| e.variants.len()).sum(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

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

    /// A sentence with no library term in it.
    const FILLER: &str = "他们把话说完，事情就算定了下来。";

    #[test]
    fn test_a_term_is_reported_with_its_category_tier_and_context() {
        let doc = doc_of(vec![
            para("第一章", "他拔刀割喉。"),
            para("第二章", FILLER),
            para("第三章", FILLER),
        ]);
        let stats = run_explicit_analysis(&doc).unwrap();

        assert_eq!(stats.total_hits, 1);
        let hit = &stats.hits[0];
        assert_eq!(hit.term, "割喉");
        assert_eq!(hit.category, "violence");
        assert_eq!(hit.tier, "strong");
        assert_eq!(hit.section, "第一章");
        assert!(hit.excerpt.contains("割喉"), "{hit:?}");
        assert_eq!(stats.advisory, "strong");
        assert_eq!(stats.advisory_categories, vec!["violence".to_string()]);
    }

    #[test]
    fn test_alternation_variants_roll_up_under_one_pattern() {
        let doc = doc_of(vec![para("第一章", "他骂她是婊子，又骂她是荡妇。")]);
        let stats = run_explicit_analysis(&doc).unwrap();

        assert_eq!(stats.total_hits, 2);
        assert_eq!(stats.distinct_terms, 1);
        assert_eq!(stats.terms[0].count, 2);
        let variants: Vec<&str> = stats.terms[0]
            .variants
            .iter()
            .map(|(v, _)| v.as_str())
            .collect();
        assert_eq!(variants, ["婊子", "荡妇"]);
    }

    #[test]
    fn test_the_longest_pattern_wins_at_a_position() {
        let stats =
            run_explicit_analysis(&doc_of(vec![para("第一章", "他自杀未遂，被救了回来。")]))
                .unwrap();

        assert_eq!(stats.total_hits, 1);
        assert_eq!(stats.hits[0].term, "自杀未遂");
    }

    #[test]
    fn test_the_advisory_level_is_the_strongest_tier_found() {
        let mild =
            run_explicit_analysis(&doc_of(vec![para("第一章", "他该死地摔了一跤。")])).unwrap();
        assert_eq!(mild.advisory, "mild");
        assert!(mild.advisory_categories.is_empty());

        let moderate = run_explicit_analysis(&doc_of(vec![para(
            "第一章",
            "她脱衣进城，路上看见一具尸体。",
        )]))
        .unwrap();
        assert_eq!(moderate.advisory, "moderate");
        assert!(moderate.advisory_categories.is_empty());

        let strong =
            run_explicit_analysis(&doc_of(vec![para("第一章", "那是一场强奸。")])).unwrap();
        assert_eq!(strong.advisory, "strong");
        assert_eq!(
            strong.advisory_categories,
            vec!["sexual_violence".to_string()]
        );
    }

    #[test]
    fn test_the_tiers_are_listed_even_when_nothing_was_found() {
        let stats = run_explicit_analysis(&doc_of(vec![para("第一章", FILLER)])).unwrap();

        let labels: Vec<&str> = stats.tiers.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, ["mild", "moderate", "strong"]);
        assert_eq!(stats.tiers.iter().map(|(_, c)| *c).sum::<u32>(), 0);
        assert_eq!(stats.advisory, "none");
        assert!(stats.issues.is_empty());
    }

    #[test]
    fn test_self_harm_is_surfaced_from_a_single_mention() {
        let doc = doc_of(vec![
            para("第一章", "他站在窗边，想到轻生。"),
            para("第二章", FILLER),
            para("第三章", FILLER),
        ]);
        let stats = run_explicit_analysis(&doc).unwrap();
        let issue = stats
            .issues
            .iter()
            .find(|i| i.kind == "self_harm")
            .expect("one self-harm mention is worth surfacing");

        assert_eq!(issue.section, "第一章");
        assert!(issue.detail.contains("1 处自伤或自杀描写"), "{issue:?}");
    }

    #[test]
    fn test_an_ordinary_scene_is_not_an_advisory() {
        // 打架 and 耳光 are ordinary fiction: naming them would cry wolf.
        let doc = doc_of(vec![para("第一章", "两个人打架，他挨了一耳光。")]);
        let stats = run_explicit_analysis(&doc).unwrap();

        assert_eq!(stats.total_hits, 2);
        assert_eq!(stats.advisory, "mild");
        assert!(!stats.issues.iter().any(|i| i.kind == "gore"));
        assert!(stats.advisory_categories.is_empty());
    }

    #[test]
    fn test_a_packed_segment_is_named() {
        let doc = doc_of(vec![para(
            "第一章",
            "屠杀、割喉、分尸、肢解、挖眼、开膛、灭口、剥皮。",
        )]);
        let stats = run_explicit_analysis(&doc).unwrap();

        assert_eq!(stats.total_hits, 8);
        assert!(
            stats.issues.iter().any(|i| i.kind == "dense_segment"),
            "eight hits in a short segment: {:?}",
            stats.issues
        );
    }

    #[test]
    fn test_a_hit_inside_quotation_marks_is_flagged_as_speech() {
        let doc = doc_of(vec![para("第一章", "他低声说：“你这个傻逼。”")]);
        let stats = run_explicit_analysis(&doc).unwrap();

        assert_eq!(stats.total_hits, 1);
        assert!(stats.hits[0].in_dialogue);
        assert_eq!(stats.dialogue_hits, 1);
    }

    #[test]
    fn test_categories_are_ranked_by_hits_then_name() {
        let doc = doc_of(vec![para("第一章", "他杀人，又杀人；她还骂他混蛋。")]);
        let stats = run_explicit_analysis(&doc).unwrap();

        assert_eq!(stats.categories[0], ("violence".to_string(), 2));
        assert_eq!(stats.categories[1], ("profanity".to_string(), 1));
    }

    #[test]
    fn test_the_curve_uses_the_shared_segment_axis() {
        let doc = doc_of(vec![
            para("第一章", "他杀人了。"),
            para("第二章", FILLER),
            para("第三章", "她割喉了事。"),
        ]);
        let stats = run_explicit_analysis(&doc).unwrap();

        let labels: Vec<&str> = stats.points.iter().map(|p| p.section.as_str()).collect();
        assert_eq!(labels, ["第一章", "第二章", "第三章"]);
        assert_eq!(stats.points[0].hits, 1);
        assert_eq!(stats.points[1].hits, 0);
        assert_eq!(stats.points[1].intensity, 0.0);
        assert_eq!(stats.peak_section, "第一章");
    }

    #[test]
    fn test_documents_without_sections_are_cut_into_chunks() {
        let blocks: Vec<SemanticBlock> = (0..25)
            .map(|_| para("同一个章节", "他杀了一个人。"))
            .collect();
        let stats = run_explicit_analysis(&doc_of(blocks)).unwrap();

        assert!(stats.chunked);
        assert_eq!(
            stats.segment_count,
            crate::segment::FALLBACK_SEGMENTS as u32
        );
        assert_eq!(stats.points[2].section, "3/10");
    }

    #[test]
    fn test_hits_follow_document_order() {
        let doc = doc_of(vec![
            para("第一章", "他杀人了。她骂他混蛋。"),
            para("第二章", "第二天，尸体被找到。"),
            para("第三章", "没有人再说话。"),
        ]);
        let stats = run_explicit_analysis(&doc).unwrap();

        assert_eq!(stats.hits.len(), 3);
        assert_eq!(stats.hits[0].term, "杀人");
        assert_eq!(stats.hits[1].term, "混蛋");
        assert_eq!(stats.hits[2].section, "第二章");
        assert_eq!(stats.hits[2].term, "尸体");
    }

    #[test]
    fn test_a_term_spanning_segments_reports_both_ends() {
        let doc = doc_of(vec![
            para("第一章", "他杀人了。"),
            para("第二章", FILLER),
            para("第三章", "她又杀人了。"),
        ]);
        let stats = run_explicit_analysis(&doc).unwrap();
        let term = stats
            .terms
            .iter()
            .find(|t| t.count == 2)
            .expect("杀人 twice");

        assert_eq!(term.first_section, "第一章");
        assert_eq!(term.last_section, "第三章");
        assert_eq!(term.segments_touched, 2);
        assert_eq!(term.category, "violence");
        assert_eq!(term.tier, "moderate");
    }

    #[test]
    fn test_an_empty_document_reports_nothing_and_no_advisory() {
        let stats = run_explicit_analysis(&doc_of(vec![])).unwrap();

        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.advisory, "none");
        assert_eq!(stats.per_10k, 0.0);
        assert!(stats.issues.is_empty());
        assert!(stats.advisory_categories.is_empty());
        assert_eq!(stats.peak_section, "");
    }

    #[test]
    fn test_the_library_size_is_reported_so_coverage_is_visible() {
        let stats = run_explicit_analysis(&doc_of(vec![para("第一章", FILLER)])).unwrap();

        assert!(
            stats.lexicon_size >= 30,
            "the library is small, and the report says so"
        );
        assert!(stats.lexicon_variants > stats.lexicon_size);
    }

    #[test]
    fn test_the_same_text_gives_the_same_numbers() {
        let doc = doc_of(vec![
            para("第一章", "他杀了一个人，骂了两句混蛋。"),
            para("第二章", "她想到轻生，后来放弃了。"),
        ]);
        let a = run_explicit_analysis(&doc).unwrap();
        let b = run_explicit_analysis(&doc).unwrap();
        assert_eq!(format!("{:?}", a), format!("{:?}", b));
    }
}
