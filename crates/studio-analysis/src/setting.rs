//! Setting analysis: where the story happens, and whether the reader is told.
//!
//! The engine reads two things the import already carries. Place entities
//! (`LOCATION`, `FACILITY`) say which places are named and how often; an
//! environment vocabulary (房间 / 街道 / 夜色 / 灯光 …) says how much of the text is
//! spent describing surroundings. From those it cuts the document into scenes —
//! a run of blocks that stays at one place — and reports where the reader is left
//! without an anchor.
//!
//! Like pacing and conflict, the curve is drawn over [`plan_segments`], so a
//! segment reads the same way across all three dimensions.
//!
//! Deterministic: scenes follow document order, places are ranked by mentions and
//! then by name, never by HashMap order.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;
use studio_core::entity::EntityCategory;

use crate::segment::{plan_segments, SegmentPlan};
use crate::text::{count_quoted_chars, visible_char_count};
use crate::Result;

/// Environment density (hits per 1000 visible chars) that fills the 0-100 scale.
const ENV_FULL: f64 = 12.0;
/// Below this the text is barely describing anything around the characters.
const THIN_ENV: f64 = 2.0;
/// Above this the surroundings are thick on the page.
const HEAVY_ENV: f64 = 10.0;
/// A place named in fewer than this many blocks is a passing mention, not a setting.
const MIN_PLACE_MENTIONS: u32 = 2;
/// How many places are reported, busiest first.
const MAX_PLACES: usize = 12;
/// How many scenes are reported; `scene_count` still counts all of them.
const MAX_SCENES: usize = 40;
/// How many issues are reported, first found first.
const MAX_ISSUES: usize = 12;
/// Segments a place stays gone for before its return is worth flagging.
const ABSENT_RUN: u32 = 3;
/// A scene with no place named and less environment than this leaves the reader
/// without an anchor.
const UNANCHORED_ENV: f64 = 3.0;
/// Distinct places named inside one segment before it counts as crowded.
const CROWDED_PLACES: u32 = 4;
/// A place needs this many mentions for the document to count as anchored.
const RECURRING_PLACE_MENTIONS: u32 = 3;

/// Environment vocabulary, matched as substrings (Chinese has no word boundaries,
/// so every term is at least two characters — a lone 门 or 风 would register on
/// ordinary speech).
const ENV_TERMS: &[&str] = &[
    // indoor
    "房间",
    "屋子",
    "屋里",
    "屋内",
    "客厅",
    "厨房",
    "卧室",
    "书房",
    "门口",
    "走廊",
    "楼梯",
    "电梯",
    "院子",
    "窗户",
    "窗外",
    "窗边",
    "阳台",
    "桌子",
    "椅子",
    "沙发",
    "地板",
    "墙壁",
    "墙面",
    "屋顶",
    "天花板",
    "灯光",
    "烛光",
    "台灯",
    // urban
    "街道",
    "马路",
    "巷子",
    "胡同",
    "路口",
    "人行道",
    "广场",
    "公园",
    "车站",
    "码头",
    "机场",
    "教室",
    "操场",
    "医院",
    "商店",
    "店铺",
    "市场",
    "工厂",
    "车间",
    "仓库",
    "办公室",
    "会议室",
    "酒店",
    "旅馆",
    "餐馆",
    "酒吧",
    "银行",
    "城墙",
    "铁路",
    "铁轨",
    // nature
    "山上",
    "山下",
    "山坡",
    "森林",
    "树林",
    "树木",
    "草地",
    "草原",
    "田野",
    "田间",
    "河边",
    "湖面",
    "湖边",
    "海边",
    "海岸",
    "沙滩",
    "泥土",
    "灰尘",
    "花朵",
    "树叶",
    "枝条",
    "雾气",
    // light and weather
    "阳光",
    "光线",
    "月光",
    "星光",
    "夜色",
    "夜幕",
    "黑暗",
    "阴影",
    "晨曦",
    "清晨",
    "早晨",
    "黄昏",
    "傍晚",
    "深夜",
    "午夜",
    "雨水",
    "雷雨",
    "雪花",
    "积雪",
    "寒风",
    "大风",
    "风声",
    "乌云",
    "雷声",
    "寒冷",
    "炎热",
    "潮湿",
    "干燥",
];

/// One segment of the environment curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingPoint {
    pub section: String,
    pub char_count: u64,
    /// Environment vocabulary hits in the segment.
    pub env_hits: u32,
    /// Hits per 1000 visible characters.
    pub density: f64,
    /// 0-100 on the absolute scale, so two projects' curves can be compared.
    pub intensity: f64,
    /// Distinct places named in the segment.
    pub place_count: u32,
    /// The place named most often here; empty when the segment names none.
    pub dominant_place: String,
}

/// One named place and how far it reaches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    /// `LOCATION` or `FACILITY`.
    pub kind: String,
    /// Blocks that name it.
    pub mentions: u32,
    pub segments_touched: u32,
    /// Fraction of the document's segments this place appears in (0-1).
    pub coverage: f64,
    pub first_section: String,
    pub last_section: String,
    /// Longest run of segments where the place is absent between two appearances.
    pub longest_absence: u32,
    /// Times it comes back after [`ABSENT_RUN`] absent segments.
    pub returns: u32,
    /// Blocks naming it per segment, aligned with the other curves' X axis.
    pub curve: Vec<u32>,
}

/// A scene: a run of consecutive blocks that stays at one place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Zero-based position in document order.
    pub index: u32,
    /// The segment where the scene starts.
    pub section: String,
    /// The place the scene is at; empty when the text never says.
    pub anchor: String,
    /// `LOCATION` / `FACILITY`, empty with no anchor.
    pub anchor_kind: String,
    pub blocks: u32,
    pub char_count: u64,
    pub env_hits: u32,
    /// Environment hits per 1000 visible characters in the scene.
    pub env_density: f64,
    /// Share of the scene's characters inside quotation marks.
    pub dialogue_share: f64,
    /// The scene carries a date expression — usually a time jump.
    pub time_mark: bool,
    /// People present, busiest first (`name, mentions`).
    pub persons: Vec<(String, u32)>,
}

/// Something the setting does for the reader, or fails to do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingIssue {
    /// `unanchored` | `crowded` | `long_absence` | `thin_environment`.
    pub kind: String,
    pub section: String,
    pub detail: String,
}

/// Setting analysis output for one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingStats {
    pub points: Vec<SettingPoint>,
    pub segment_count: u32,
    /// True when the segments are block chunks rather than sections.
    pub chunked: bool,
    /// Reported scenes, document order.
    pub scenes: Vec<Scene>,
    /// All scenes, including unreported ones.
    pub scene_count: u32,
    /// Scenes that name no place and describe too little to stand in for one.
    pub unanchored_scenes: u32,
    /// Share of scenes left without an anchor.
    pub unanchored_share: f64,
    pub places: Vec<Place>,
    /// Distinct places named anywhere in the document.
    pub place_count: u32,
    /// Places named in at least [`RECURRING_PLACE_MENTIONS`] blocks.
    pub recurring_places: u32,
    pub total_env_hits: u32,
    /// Busiest environment terms across the document, most first.
    pub top_env_terms: Vec<(String, u32)>,
    /// Environment hits per 1000 visible characters across the document.
    pub env_density: f64,
    /// `none` | `thin` | `present` | `heavy` — description vocabulary, not quality.
    pub verdict: String,
    pub dominant_place: String,
    pub peak_section: String,
    pub peak_intensity: f64,
    pub issues: Vec<SettingIssue>,
    pub longest_absence: u32,
    /// 0-100, or `None` when the document names no place and carries no
    /// environment vocabulary — there is nothing to grade.
    pub setting_score: Option<f64>,
}

/// What one block contributes: the place it names, its environment hits, and who
/// is in it.
#[derive(Debug, Default)]
struct BlockFacts {
    /// The place named most in this block (`name, kind`); ties go to the first named.
    anchor: Option<(String, &'static str)>,
    /// Distinct places named here with their entity kind, in reading order.
    places: Vec<(String, &'static str)>,
    env_hits: u32,
    chars: u64,
    quoted: u64,
    time_mark: bool,
    persons: Vec<(String, u32)>,
}

/// Per-segment counters.
#[derive(Debug, Default)]
struct Segment {
    label: String,
    chars: u64,
    env_hits: u32,
    place_counts: HashMap<String, u32>,
    /// Place names in first-appearance order, so the dominant place is a count
    /// question rather than a HashMap question.
    place_order: Vec<String>,
}

/// Read what each block says about place, surroundings and people, totalling the
/// environment vocabulary while the text is already open.
fn collect_facts(doc: &DocumentData) -> (Vec<BlockFacts>, Vec<(String, u32)>) {
    let mut facts = Vec::with_capacity(doc.blocks.len());
    let mut term_hits: HashMap<String, u32> = HashMap::new();
    for block in &doc.blocks {
        let mut env_hits = 0u32;
        for term in ENV_TERMS {
            let hits = block.content.match_indices(term).count() as u32;
            if hits > 0 {
                env_hits += hits;
                *term_hits.entry((*term).to_string()).or_insert(0) += hits;
            }
        }
        let mut f = BlockFacts {
            chars: visible_char_count(&block.content) as u64,
            quoted: count_quoted_chars(&block.content) as u64,
            env_hits,
            ..Default::default()
        };

        let mut place_counts: HashMap<String, (u32, &'static str)> = HashMap::new();
        let mut person_counts: HashMap<String, u32> = HashMap::new();
        for entity in &block.entities {
            match entity.category {
                EntityCategory::Location | EntityCategory::Facility => {
                    let kind = entity.category.as_str();
                    if !f.places.iter().any(|(name, _)| name == &entity.text) {
                        f.places.push((entity.text.clone(), kind));
                    }
                    let entry = place_counts.entry(entity.text.clone()).or_insert((0, kind));
                    entry.0 += 1;
                }
                EntityCategory::Date => f.time_mark = true,
                EntityCategory::Person => {
                    *person_counts.entry(entity.text.clone()).or_insert(0) += 1;
                }
                _ => {}
            }
        }

        // The anchor is the place this block names most; a block that mentions 码头
        // twice and 上海 once is at the 码头. Ties go to the name that appears first.
        let mut best: Option<(u32, &str, &'static str)> = None;
        for (name, kind) in &f.places {
            let (count, _) = place_counts[name.as_str()];
            if best.map(|b| count > b.0).unwrap_or(true) {
                best = Some((count, name.as_str(), kind));
            }
        }
        f.anchor = best.map(|(_, name, kind)| (name.to_string(), kind));

        let mut persons: Vec<(String, u32)> = person_counts.into_iter().collect();
        // Count first, name second: HashMap order must not reach the report.
        persons.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        f.persons = persons;

        facts.push(f);
    }

    let mut terms: Vec<(String, u32)> = term_hits.into_iter().collect();
    // Count first, term second: HashMap order must not reach the report.
    terms.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    (facts, terms)
}

/// Total places and environment hits per segment, reusing the per-block counts.
fn count_segments(plan: &SegmentPlan, facts: &[BlockFacts]) -> Vec<Segment> {
    if plan.labels.is_empty() {
        return Vec::new();
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
        s.env_hits += f.env_hits;
        for (name, _) in &f.places {
            let entry = s.place_counts.entry(name.clone()).or_insert(0);
            if *entry == 0 {
                s.place_order.push(name.clone());
            }
            *entry += 1;
        }
    }

    segments
}

/// The place named most in a segment; ties go to the first named there.
fn dominant_place(segment: &Segment) -> String {
    let mut best: Option<(u32, &str)> = None;
    for name in &segment.place_order {
        let count = segment.place_counts[name];
        if best.map(|b| count > b.0).unwrap_or(true) {
            best = Some((count, name.as_str()));
        }
    }
    best.map(|(_, name)| name.to_string()).unwrap_or_default()
}

/// Read where the document happens: its scenes, the places it names, how much of
/// the page is spent on surroundings, and the spots where the reader is left
/// without an anchor.
pub fn run_setting_analysis(doc: &DocumentData) -> Result<SettingStats> {
    let plan = plan_segments(doc);
    let (facts, env_terms) = collect_facts(doc);
    let segments = count_segments(&plan, &facts);

    let total_chars: u64 = segments.iter().map(|s| s.chars).sum();
    let total_env_hits: u32 = segments.iter().map(|s| s.env_hits).sum();
    let env_density = if total_chars == 0 {
        0.0
    } else {
        total_env_hits as f64 / total_chars as f64 * 1000.0
    };

    let points: Vec<SettingPoint> = segments
        .iter()
        .map(|s| {
            let chars = s.chars.max(1) as f64;
            let density = s.env_hits as f64 / chars * 1000.0;
            SettingPoint {
                section: s.label.clone(),
                char_count: s.chars,
                env_hits: s.env_hits,
                density,
                intensity: (density / ENV_FULL).min(1.0) * 100.0,
                place_count: s.place_order.len() as u32,
                dominant_place: dominant_place(s),
            }
        })
        .collect();

    let all_places = place_totals(&plan, &facts, &segments);
    let places: Vec<Place> = all_places
        .iter()
        .filter(|p| p.mentions >= MIN_PLACE_MENTIONS)
        .take(MAX_PLACES)
        .cloned()
        .collect();
    let recurring_places = all_places
        .iter()
        .filter(|p| p.mentions >= RECURRING_PLACE_MENTIONS)
        .count() as u32;
    let longest_absence = all_places
        .iter()
        .map(|p| p.longest_absence)
        .max()
        .unwrap_or(0);

    let all_scenes = group_scenes(doc, &plan, &facts);
    let scene_count = all_scenes.len() as u32;
    let unanchored_scenes = all_scenes.iter().filter(|s| is_unanchored(s)).count() as u32;
    let unanchored_share = if scene_count == 0 {
        0.0
    } else {
        unanchored_scenes as f64 / scene_count as f64
    };
    let scenes = all_scenes.iter().take(MAX_SCENES).cloned().collect();

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

    let issues = find_issues(
        &all_scenes,
        &segments,
        &all_places,
        env_density,
        total_env_hits,
    );

    let mut stats = SettingStats {
        points,
        segment_count: segments.len() as u32,
        chunked: plan.chunked,
        scenes,
        scene_count,
        unanchored_scenes,
        unanchored_share,
        places,
        place_count: all_places.len() as u32,
        recurring_places,
        total_env_hits,
        env_density,
        top_env_terms: env_terms.into_iter().take(10).collect(),
        verdict: verdict(env_density, total_env_hits).to_string(),
        dominant_place: all_places
            .first()
            .map(|p| p.name.clone())
            .unwrap_or_default(),
        peak_section,
        peak_intensity,
        issues,
        longest_absence,
        setting_score: None,
    };
    // The score reads the stats rather than a long parameter list: every number it
    // charges for is one the caller can see in the output.
    stats.setting_score = setting_score(&stats);
    Ok(stats)
}

/// A scene the text never places: no name, and not enough description to stand in
/// for one.
fn is_unanchored(scene: &Scene) -> bool {
    scene.anchor.is_empty() && scene.env_density < UNANCHORED_ENV
}

/// How much environment vocabulary the document carries. `none` deliberately does
/// not say "no setting": it says no setting *vocabulary*.
fn verdict(density: f64, total_hits: u32) -> &'static str {
    if total_hits == 0 {
        "none"
    } else if density < THIN_ENV {
        "thin"
    } else if density > HEAVY_ENV {
        "heavy"
    } else {
        "present"
    }
}

/// Cut the document into scenes: runs of blocks that stay at one place.
///
/// A block naming a different place opens a new scene. A block naming no place
/// continues the current one — prose rarely re-states where it is — and a place
/// named later fills in an anchor the opening block left blank. A title block
/// always opens a scene, because a heading is the author's own scene break.
fn group_scenes(doc: &DocumentData, plan: &SegmentPlan, facts: &[BlockFacts]) -> Vec<Scene> {
    let mut scenes: Vec<Scene> = Vec::new();
    let mut quoted_totals: Vec<u64> = Vec::new();
    let mut person_maps: Vec<HashMap<String, u32>> = Vec::new();

    for (idx, block) in doc.blocks.iter().enumerate() {
        let f = &facts[idx];
        let opens_new = match scenes.last() {
            None => true,
            Some(current) => {
                block.block_type == "title"
                    || match &f.anchor {
                        Some((name, _)) => !current.anchor.is_empty() && *name != current.anchor,
                        None => false,
                    }
            }
        };
        if opens_new {
            scenes.push(Scene {
                index: scenes.len() as u32,
                section: plan
                    .labels
                    .get(plan.segment_of.get(idx).copied().unwrap_or(0))
                    .cloned()
                    .unwrap_or_default(),
                anchor: f
                    .anchor
                    .as_ref()
                    .map(|(n, _)| n.clone())
                    .unwrap_or_default(),
                anchor_kind: f
                    .anchor
                    .as_ref()
                    .map(|(_, k)| (*k).to_string())
                    .unwrap_or_default(),
                blocks: 0,
                char_count: 0,
                env_hits: 0,
                env_density: 0.0,
                dialogue_share: 0.0,
                time_mark: false,
                persons: Vec::new(),
            });
            quoted_totals.push(0);
            person_maps.push(HashMap::new());
        }

        let position = scenes.len() - 1;
        let scene = &mut scenes[position];
        if scene.anchor.is_empty() {
            if let Some((name, kind)) = &f.anchor {
                scene.anchor = name.clone();
                scene.anchor_kind = (*kind).to_string();
            }
        }
        scene.blocks += 1;
        scene.char_count += f.chars;
        scene.env_hits += f.env_hits;
        scene.time_mark |= f.time_mark;
        quoted_totals[position] += f.quoted;
        for (name, count) in &f.persons {
            *person_maps[position].entry(name.clone()).or_insert(0) += count;
        }
    }

    for (idx, scene) in scenes.iter_mut().enumerate() {
        let chars = scene.char_count.max(1) as f64;
        scene.env_density = scene.env_hits as f64 / chars * 1000.0;
        scene.dialogue_share = if scene.char_count == 0 {
            0.0
        } else {
            quoted_totals[idx] as f64 / chars
        };
        let mut persons: Vec<(String, u32)> = person_maps[idx].clone().into_iter().collect();
        persons.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        scene.persons = persons.into_iter().take(5).collect();
    }
    scenes
}

/// Per-place totals across the document: how many blocks name it, which segments,
/// and how long it stays gone between appearances.
fn place_totals(plan: &SegmentPlan, facts: &[BlockFacts], segments: &[Segment]) -> Vec<Place> {
    if plan.labels.is_empty() {
        return Vec::new();
    }

    let mut mentions: HashMap<String, u32> = HashMap::new();
    let mut kinds: HashMap<String, &str> = HashMap::new();
    for f in facts {
        for (name, kind) in &f.places {
            *mentions.entry(name.clone()).or_insert(0) += 1;
            kinds.entry(name.clone()).or_insert(*kind);
        }
    }

    let mut places: Vec<Place> = mentions
        .keys()
        .map(|name| {
            let curve: Vec<u32> = segments
                .iter()
                .map(|s| s.place_counts.get(name).copied().unwrap_or(0))
                .collect();
            let touched = curve.iter().filter(|c| **c > 0).count() as u32;
            let (longest_absence, returns) = absence(&curve);
            Place {
                name: name.clone(),
                kind: kinds[name].to_string(),
                mentions: mentions[name],
                segments_touched: touched,
                coverage: if curve.is_empty() {
                    0.0
                } else {
                    touched as f64 / curve.len() as f64
                },
                first_section: curve
                    .iter()
                    .position(|c| *c > 0)
                    .map(|i| plan.labels[i].clone())
                    .unwrap_or_default(),
                last_section: curve
                    .iter()
                    .rposition(|c| *c > 0)
                    .map(|i| plan.labels[i].clone())
                    .unwrap_or_default(),
                longest_absence,
                returns,
                curve,
            }
        })
        .collect();
    // Mentions first, name second: HashMap order must not reach the report.
    places.sort_by(|a, b| {
        b.mentions
            .cmp(&a.mentions)
            .then_with(|| a.name.cmp(&b.name))
    });
    places
}

/// Longest run of segments between two appearances of a place, and how many such
/// runs are long enough to read as a return.
///
/// Leading and trailing absences are not counted: a place that has not appeared
/// yet has not been left, and one that never comes back is a dropped thread, which
/// is what plot analysis reports.
fn absence(curve: &[u32]) -> (u32, u32) {
    let mut longest = 0u32;
    let mut returns = 0u32;
    let mut gap = 0u32;
    let mut seen = false;
    for count in curve {
        if *count == 0 {
            if seen {
                gap += 1;
            }
        } else {
            if seen {
                longest = longest.max(gap);
                if gap >= ABSENT_RUN {
                    returns += 1;
                }
            }
            gap = 0;
            seen = true;
        }
    }
    (longest, returns)
}

/// What the setting gives the reader, and what it withholds.
fn find_issues(
    scenes: &[Scene],
    segments: &[Segment],
    places: &[Place],
    env_density: f64,
    total_env_hits: u32,
) -> Vec<SettingIssue> {
    let mut issues: Vec<SettingIssue> = Vec::new();

    for scene in scenes.iter().filter(|s| is_unanchored(s)).take(MAX_ISSUES) {
        issues.push(SettingIssue {
            kind: "unanchored".into(),
            section: scene.section.clone(),
            detail: format!(
                "第 {} 场没有点名地点，环境用词也只有 {:.1} 次/千字",
                scene.index + 1,
                scene.env_density
            ),
        });
    }

    for s in segments
        .iter()
        .filter(|s| s.place_order.len() as u32 >= CROWDED_PLACES)
        .take(MAX_ISSUES)
    {
        let names = s.place_order.iter().take(6).cloned().collect::<Vec<_>>();
        issues.push(SettingIssue {
            kind: "crowded".into(),
            section: s.label.clone(),
            detail: format!("本段出现 {} 个不同地点：{}", names.len(), names.join("、")),
        });
    }

    for p in places.iter().filter(|p| p.returns > 0).take(MAX_ISSUES) {
        issues.push(SettingIssue {
            kind: "long_absence".into(),
            section: p.first_section.clone(),
            detail: format!(
                "{} 最长缺席 {} 段，之后又出现 {} 次",
                p.name, p.longest_absence, p.returns
            ),
        });
    }

    if total_env_hits == 0 {
        issues.push(SettingIssue {
            kind: "thin_environment".into(),
            section: String::new(),
            detail: "全文没有环境用词：场景完全靠人物对话支撑。".into(),
        });
    } else if env_density < THIN_ENV {
        issues.push(SettingIssue {
            kind: "thin_environment".into(),
            section: String::new(),
            detail: format!("环境用词偏少（{:.1} 次/千字）。", env_density),
        });
    }

    issues
}

/// What the setting does for the reader, scored from what the output already shows.
///
/// Nothing is charged for a thin description of a story that names its places, and
/// nothing for unnamed places in a story thick with description: either one alone
/// can anchor a scene. What costs is having neither.
fn setting_score(stats: &SettingStats) -> Option<f64> {
    if stats.place_count == 0 && stats.total_env_hits == 0 {
        return None;
    }

    let unanchored = if stats.unanchored_share > 0.3 {
        ((stats.unanchored_share - 0.3) * 60.0).min(25.0)
    } else {
        0.0
    };
    let thin_env = if stats.env_density < THIN_ENV {
        (THIN_ENV - stats.env_density) / THIN_ENV * 20.0
    } else {
        0.0
    };
    let no_recurring_place = if stats.recurring_places > 0 || stats.env_density >= HEAVY_ENV {
        0.0
    } else {
        20.0
    };
    let crowded =
        (stats.issues.iter().filter(|i| i.kind == "crowded").count() as f64 * 5.0).min(15.0);

    Some((100.0 - unanchored - thin_env - no_recurring_place - crowded).clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;
    use studio_core::entity::Entity;

    fn entity(text: &str, category: EntityCategory) -> Entity {
        Entity {
            text: text.into(),
            category,
            confidence: 0.9,
            source: "test".into(),
            keep: true,
            filter: None,
            filter_reason: None,
            span: (0, 0),
            stable_id: None,
        }
    }

    fn para(path: &str, content: &str, entities: Vec<Entity>) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: content.into(),
            section_path: path.into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities,
            noun_signals: vec![],
        }
    }

    fn heading(path: &str, content: &str, entities: Vec<Entity>) -> SemanticBlock {
        let mut block = para(path, content, entities);
        block.block_type = "title".into();
        block
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
    fn test_a_change_of_place_opens_a_new_scene() {
        let doc = doc_of(vec![
            para(
                "S1",
                "码头边。",
                vec![entity("码头", EntityCategory::Facility)],
            ),
            para("S1", "他把绳子系紧。", vec![]),
            para(
                "S2",
                "车站里很吵。",
                vec![entity("车站", EntityCategory::Location)],
            ),
            para("S3", "他等了一夜。", vec![]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        // The block that names no place stays in the scene it was already in: prose
        // does not re-state where it is.
        assert_eq!(stats.scene_count, 2);
        assert_eq!(stats.scenes.len(), 2);
        assert_eq!(stats.scenes[0].anchor, "码头");
        assert_eq!(stats.scenes[0].anchor_kind, "FACILITY");
        assert_eq!(stats.scenes[0].blocks, 2);
        assert_eq!(stats.scenes[0].section, "S1");
        assert_eq!(stats.scenes[1].anchor, "车站");
        assert_eq!(stats.scenes[1].anchor_kind, "LOCATION");
        assert_eq!(stats.scenes[1].section, "S2");
        assert_eq!(stats.segment_count, 3);
        assert!(!stats.chunked);
    }

    #[test]
    fn test_a_heading_opens_a_scene_even_at_the_same_place() {
        let doc = doc_of(vec![
            para(
                "S1",
                "码头边。",
                vec![entity("码头", EntityCategory::Facility)],
            ),
            heading(
                "S2",
                "第二天。",
                vec![entity("码头", EntityCategory::Facility)],
            ),
            para("S3", "他回来了。", vec![]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        assert_eq!(stats.scene_count, 2);
        assert_eq!(stats.scenes[1].anchor, "码头");
        assert_eq!(stats.scenes[1].section, "S2");
    }

    #[test]
    fn test_a_place_named_later_fills_in_the_scene() {
        let doc = doc_of(vec![
            para("S1", "他推开门。", vec![]),
            para(
                "S1",
                "厨房里没有灯。",
                vec![entity("厨房", EntityCategory::Facility)],
            ),
            para("S2", "他站着不动。", vec![]),
            para("S3", "外面下雨了。", vec![]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        // One scene, anchored by the place named in its second block — the opening
        // block did not say where, but the scene does.
        assert_eq!(stats.scene_count, 1);
        assert_eq!(stats.scenes[0].anchor, "厨房");
    }

    #[test]
    fn test_environment_vocabulary_is_counted_per_segment() {
        let doc = doc_of(vec![
            para("S1", "房间里灯光很暗。", vec![]),
            para("S2", "街道上空无一人。", vec![]),
            para("S3", "夜色很深。", vec![]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        assert_eq!(stats.total_env_hits, 4);
        assert_eq!(stats.points.len(), 3);
        assert_eq!(stats.points[0].env_hits, 2);
        assert_eq!(stats.points[1].env_hits, 1);
        assert!(stats.env_density > 0.0);
        assert_eq!(stats.verdict, "heavy");
        // Every term was seen once, so the list falls back to name order rather
        // than the order the vocabulary happens to be written in.
        let terms: Vec<&str> = stats
            .top_env_terms
            .iter()
            .map(|(term, _)| term.as_str())
            .collect();
        assert_eq!(terms, ["夜色", "房间", "灯光", "街道"]);
        assert_eq!(stats.peak_section, "S1");
        assert!((stats.peak_intensity - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_a_passing_place_name_is_not_a_setting() {
        let doc = doc_of(vec![
            para(
                "S1",
                "码头边。",
                vec![entity("码头", EntityCategory::Facility)],
            ),
            para(
                "S2",
                "他去了北京。",
                vec![entity("北京", EntityCategory::Location)],
            ),
            para("S3", "他回来了。", vec![]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        // Both are named once: they are counted, but neither is a setting.
        assert_eq!(stats.place_count, 2);
        assert!(stats.places.is_empty());
        assert_eq!(stats.recurring_places, 0);
    }

    #[test]
    fn test_places_are_ranked_by_mentions_then_name() {
        let doc = doc_of(vec![
            para("S1", "甲。", vec![entity("乙地", EntityCategory::Location)]),
            para("S2", "甲。", vec![entity("甲地", EntityCategory::Location)]),
            para("S3", "甲。", vec![entity("甲地", EntityCategory::Location)]),
            para("S3", "甲。", vec![entity("甲地", EntityCategory::Location)]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        // 乙地 is named once: counted in the total, too passing to report.
        assert_eq!(stats.place_count, 2);
        assert_eq!(stats.places.len(), 1);
        assert_eq!(stats.places[0].name, "甲地");
        assert_eq!(stats.places[0].mentions, 3);
        assert_eq!(stats.places[0].segments_touched, 2);
        assert!((stats.places[0].coverage - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(stats.places[0].first_section, "S2");
        assert_eq!(stats.places[0].last_section, "S3");
        assert_eq!(stats.places[0].curve, vec![0, 1, 2]);
        assert_eq!(stats.dominant_place, "甲地");
    }

    #[test]
    fn test_a_place_that_returns_after_a_long_gap_is_flagged() {
        let doc = doc_of(vec![
            para("S1", "甲。", vec![entity("码头", EntityCategory::Facility)]),
            para("S2", "甲。", vec![]),
            para("S3", "甲。", vec![]),
            para("S4", "甲。", vec![]),
            para("S5", "甲。", vec![entity("码头", EntityCategory::Facility)]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        assert_eq!(stats.places[0].curve, vec![1, 0, 0, 0, 1]);
        assert_eq!(stats.places[0].longest_absence, 3);
        assert_eq!(stats.places[0].returns, 1);
        assert_eq!(stats.longest_absence, 3);
        assert!(stats
            .issues
            .iter()
            .any(|i| i.kind == "long_absence" && i.detail.contains("码头")));
    }

    #[test]
    fn test_a_segment_with_too_many_places_is_flagged() {
        let doc = doc_of(vec![
            para("S1", "甲。", vec![entity("甲地", EntityCategory::Location)]),
            para("S1", "乙。", vec![entity("乙地", EntityCategory::Location)]),
            para("S1", "丙。", vec![entity("丙地", EntityCategory::Location)]),
            para("S1", "丁。", vec![entity("丁地", EntityCategory::Location)]),
            para("S2", "戊。", vec![]),
            para("S3", "己。", vec![]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        assert!(stats
            .issues
            .iter()
            .any(|i| i.kind == "crowded" && i.section == "S1"));
        assert_eq!(stats.points[0].place_count, 4);
    }

    #[test]
    fn test_a_document_without_places_or_description_is_unscored() {
        let doc = doc_of(vec![
            para("S1", "他说了句话。", vec![]),
            para("S2", "她点了头。", vec![]),
            para("S3", "没有人回答。", vec![]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        assert_eq!(stats.place_count, 0);
        assert_eq!(stats.total_env_hits, 0);
        assert_eq!(stats.verdict, "none");
        // None of the three blocks names a place, so they read as one scene that
        // never says where it is.
        assert_eq!(stats.scene_count, 1);
        assert_eq!(stats.unanchored_scenes, 1);
        assert!((stats.unanchored_share - 1.0).abs() < 1e-9);
        assert!(stats
            .issues
            .iter()
            .any(|i| i.kind == "unanchored" && i.section == "S1"));
        assert!(stats
            .issues
            .iter()
            .any(|i| i.kind == "thin_environment" && i.detail.contains("没有环境用词")));
        // Nothing to grade: no name and no description anywhere in the document.
        assert!(stats.setting_score.is_none());
    }

    #[test]
    fn test_description_alone_anchors_a_scene() {
        let doc = doc_of(vec![
            para("S1", "房间里灯光很暗，窗外是街道。", vec![]),
            para("S2", "夜色很深。", vec![]),
            para("S3", "雨水打在窗户上。", vec![]),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        assert!(stats.scenes.iter().all(|s| s.anchor.is_empty()));
        assert_eq!(stats.place_count, 0);
        assert_eq!(stats.total_env_hits, 7);
        assert_eq!(stats.unanchored_scenes, 0);
        // Thick description carries the scene, so a document that never names a
        // place is not charged for it.
        assert!((stats.setting_score.expect("scored") - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_named_places_are_enough_without_description() {
        let doc = doc_of(vec![
            para(
                "S1",
                "北京。",
                vec![entity("北京", EntityCategory::Location)],
            ),
            para(
                "S2",
                "他在北京等。",
                vec![entity("北京", EntityCategory::Location)],
            ),
            para(
                "S3",
                "他到了北京。",
                vec![entity("北京", EntityCategory::Location)],
            ),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        assert_eq!(stats.recurring_places, 1);
        assert_eq!(stats.unanchored_scenes, 0);
        // A proper name locates a scene without describing it, so only the missing
        // description is charged (20 of 100).
        let score = stats.setting_score.expect("scored");
        assert!((score - 80.0).abs() < 1e-9, "score was {score}");
    }

    #[test]
    fn test_a_place_word_counts_as_description_too() {
        let doc = doc_of(vec![
            para(
                "S1",
                "码头。",
                vec![entity("码头", EntityCategory::Facility)],
            ),
            para(
                "S2",
                "他在码头等。",
                vec![entity("码头", EntityCategory::Facility)],
            ),
            para(
                "S3",
                "码头关门了。",
                vec![entity("码头", EntityCategory::Facility)],
            ),
        ]);
        let stats = run_setting_analysis(&doc).expect("setting analysis");

        // 码头 is both a facility the importer names and an environment word: a text
        // that keeps standing at the 码头 is describing its surroundings, so the
        // density sees it and nothing is charged.
        assert_eq!(stats.total_env_hits, 3);
        assert_eq!(stats.verdict, "heavy");
        let score = stats.setting_score.expect("scored");
        assert!((score - 100.0).abs() < 1e-9, "score was {score}");
    }

    #[test]
    fn test_a_document_without_sections_is_cut_into_chunks() {
        let blocks: Vec<SemanticBlock> = (0..12)
            .map(|i| {
                let content = if i % 2 == 0 {
                    "房间里。"
                } else {
                    "他说了句话。"
                };
                para("同一个章节", content, vec![])
            })
            .collect();
        let stats = run_setting_analysis(&doc_of(blocks)).expect("setting analysis");

        assert!(stats.chunked);
        assert_eq!(stats.segment_count, 10);
        assert_eq!(stats.points[0].section, "1/10");
        assert_eq!(stats.total_env_hits, 6);
    }

    #[test]
    fn test_an_empty_document_has_nothing_to_report() {
        let stats = run_setting_analysis(&doc_of(vec![])).expect("setting analysis");

        assert_eq!(stats.segment_count, 0);
        assert_eq!(stats.scene_count, 0);
        assert!(stats.points.is_empty());
        assert!(stats.places.is_empty());
        assert_eq!(stats.verdict, "none");
        assert!(stats.setting_score.is_none());
    }

    #[test]
    fn test_same_document_yields_identical_output() {
        let doc = doc_of(vec![
            para(
                "S1",
                "码头边。",
                vec![entity("码头", EntityCategory::Facility)],
            ),
            para("S2", "房间里灯光很暗。", vec![]),
            para(
                "S3",
                "车站。",
                vec![entity("车站", EntityCategory::Location)],
            ),
            para("S3", "他等了一夜。", vec![]),
        ]);
        let a = serde_json::to_string(&run_setting_analysis(&doc).unwrap()).unwrap();
        let b = serde_json::to_string(&run_setting_analysis(&doc).unwrap()).unwrap();
        assert_eq!(a, b);
    }
}
