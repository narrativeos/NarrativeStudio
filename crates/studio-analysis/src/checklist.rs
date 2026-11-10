//! 最终审查清单 — one pass through the numbers before the text is called done.
//!
//! This is not a fifteenth measurement. Every item here is a number some other
//! dimension already produced, gathered in one place so the last read has a
//! starting point instead of a blank page. Nothing is invented on the way: a rule
//! with no number to point at stays silent, so an empty list means *nothing that
//! was counted needs a decision* — not *the text is good*.
//!
//! Like 作者声音 the checklist carries **no score**, and for a related reason: the
//! items are questions. 一条只出现一次的线索 may be deliberate foreshadowing left
//! unharvested or a thread that fell out of the draft, and the count cannot tell
//! which. What the count can do is say where to look, which is the whole job.
//!
//! A tick is the reader's own progress and lives in the frontend; nothing here
//! changes because of it, so re-running the analysis keeps the list — and the
//! stable `id` each tick is stored under — identical. Deterministic by
//! construction: rules are checked in the fixed order written below, the groups
//! are the fixed [`CHECKLIST_GROUPS`] list, and no HashMap is read.

use serde::{Deserialize, Serialize};

use crate::cliche::ClicheStats;
use crate::conflict::ConflictStats;
use crate::dialogue::DialogueStats;
use crate::explicit::ExplicitStats;
use crate::pacing::PacingStats;
use crate::plot::PlotStats;
use crate::setting::SettingStats;
use crate::structure::StructureStats;
use crate::t0::T0Stats;
use crate::timeline::TimelineStats;
use crate::voice::VoiceStats;

/// The passes a final read is split into, in this fixed order. Every item belongs
/// to exactly one of them, so the list can be read one pass at a time.
pub const CHECKLIST_GROUPS: &[(&str, &str)] = &[
    ("consistency", "一致性与设定"),
    ("structure", "结构与完整"),
    ("language", "语言"),
    ("advisory", "读者提示"),
];

/// How many examples an item names before it switches to a count.
const MAX_EXAMPLES: usize = 3;
/// Share of scenes left without an anchor that is worth a question rather than a
/// shrug. Below this, the odd scene without a place is just prose moving through.
const UNANCHORED_SHARE: f64 = 0.3;
/// Consecutive segments with no conflict vocabulary at all before the run is long
/// enough to ask about. Two quiet segments in a row is a scene; five is a stretch.
const ABSENT_RUN_SEGMENTS: u32 = 3;
/// One adverb used this many times reads as a word the author keeps reaching for.
const ADVERB_OVERUSE: u32 = 20;
/// Below this readability score the text asks more of a reader than the number
/// alone explains, so the item asks whether that was the plan.
const READABILITY_FLOOR: f64 = 60.0;

/// One thing to look at.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    /// Stable key, and the key a tick is stored under. Rewording an item must not
    /// change this, or a reader's earlier ticks land on the wrong line.
    pub id: String,
    /// One of [`CHECKLIST_GROUPS`].
    pub group: String,
    /// What to confirm, in one line.
    pub label: String,
    /// The number behind it — an item is never printed without one.
    pub detail: String,
    /// `high` | `medium` | `low`: how far up its group the item sits. It is an
    /// ordering, not a fault: 需要确认 is not 有错.
    pub severity: String,
}

/// The whole list, derived from the other dimensions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReviewChecklist {
    pub items: Vec<ChecklistItem>,
    /// Item counts per group, always all four in [`CHECKLIST_GROUPS`] order — a group with
    /// nothing in it is printed as 0 rather than dropped.
    pub groups: Vec<(String, u32)>,
}

impl ReviewChecklist {
    /// True when no rule had a number to point at.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Everything the rules read. All borrowed: the checklist is a roll-up, so it never
/// owns a dimension's result and cannot drift from it.
pub struct ChecklistSource<'a> {
    pub t0: &'a T0Stats,
    pub plot: &'a PlotStats,
    pub pacing: &'a PacingStats,
    pub conflict: &'a ConflictStats,
    pub setting: &'a SettingStats,
    pub cliche: &'a ClicheStats,
    pub dialogue: &'a DialogueStats,
    pub explicit: &'a ExplicitStats,
    pub structure: &'a StructureStats,
    pub timeline: &'a TimelineStats,
    pub voice: &'a VoiceStats,
}

/// Gather the items. The order below is the order they are checked in, and the
/// order the list is printed in.
pub fn run_review_checklist(src: &ChecklistSource) -> ReviewChecklist {
    let mut items: Vec<ChecklistItem> = Vec::new();

    // ── 一致性与设定 ───────────────────────────────────────────────────────────
    if !src.plot.unresolved_threads.is_empty() {
        push(
            &mut items,
            "plot-unresolved",
            "consistency",
            "high",
            "确认只出现一次的线索是有意留白，不是漏掉的伏笔",
            format!(
                "{} 条线索只在单个章节出现（共收集 {} 条）：{}",
                src.plot.unresolved_threads.len(),
                src.plot.thread_count,
                examples(&src.plot.unresolved_threads)
            ),
        );
    }
    if !src.plot.late_arrivals.is_empty() {
        push(
            &mut items,
            "plot-late",
            "consistency",
            "high",
            "确认文末才出场的线索不是前文缺笔",
            format!(
                "{} 条线索第一次出现在全文最后五分之一：{}",
                src.plot.late_arrivals.len(),
                examples(&src.plot.late_arrivals)
            ),
        );
    }
    if let Some(first) = src.timeline.backsteps.first() {
        push(
            &mut items,
            "timeline-backstep",
            "consistency",
            "medium",
            "确认日历上倒退的日期是闪回，不是笔误",
            format!(
                "{} 处日期在后一处更早：{}「{}」→ {}「{}」",
                src.timeline.backsteps.len(),
                first.from_section,
                first.from_date,
                first.to_section,
                first.to_date
            ),
        );
    }
    if !src.timeline.readable && src.timeline.entity_date_count > 0 {
        push(
            &mut items,
            "timeline-unreadable",
            "consistency",
            "low",
            "核对系统读不出的日期写法",
            format!(
                "导入的实体标注里有 {} 处日期，但正文的写法不在可解析范围内：{}",
                src.timeline.entity_date_count, src.timeline.note
            ),
        );
    }
    if src.setting.scene_count > 0
        && src.setting.places_measured
        && src.setting.unanchored_share >= UNANCHORED_SHARE
    {
        push(
            &mut items,
            "setting-unanchored",
            "consistency",
            "medium",
            "确认没有地点锚点的场景是有意留白",
            format!(
                "{} / {} 个场景既没点名地点，也没有够用的环境词（{:.0}%）",
                src.setting.unanchored_scenes,
                src.setting.scene_count,
                src.setting.unanchored_share * 100.0
            ),
        );
    }
    let entity_total: u32 = src.t0.entity_counts.iter().map(|(_, c)| *c).sum();
    if entity_total > 0 {
        push(
            &mut items,
            "entity-alias",
            "consistency",
            "low",
            "核对同一实体在全文的写法一致",
            format!(
                "登记了 {} 处实体提及；系统只做精确匹配，没有合并别名，\
                 异名同写与同名异写都要人工确认",
                entity_total
            ),
        );
    }

    // ── 结构与完整 ─────────────────────────────────────────────────────────────
    if !src.structure.issues.is_empty() {
        let kinds: Vec<String> = src
            .structure
            .issues
            .iter()
            .map(|i| structure_kind_label(&i.kind))
            .collect();
        push(
            &mut items,
            "structure-issue",
            "structure",
            "high",
            "确认结构上的缺项是有意为之",
            format!("{} 项：{}", src.structure.issues.len(), join(&kinds)),
        );
    }
    if !src.structure.readable {
        push(
            &mut items,
            "structure-unreadable",
            "structure",
            "low",
            "确认结构没有被比对只是因为文本太短或切分太少",
            src.structure.note.clone(),
        );
    }
    if !src.pacing.drag_sections.is_empty() {
        let names: Vec<String> = src
            .pacing
            .drag_sections
            .iter()
            .take(MAX_EXAMPLES)
            .map(|d| format!("{}（{} 字）", d.section, d.char_count))
            .collect();
        push(
            &mut items,
            "pacing-drag",
            "structure",
            "medium",
            "确认张力持平的长段落要压缩，还是要并进相邻场景",
            format!(
                "{} 个段落长于平均而张力几乎不动：{}",
                src.pacing.drag_sections.len(),
                join(&names)
            ),
        );
    }
    if src.conflict.total_hits > 0 && src.conflict.longest_absent_run >= ABSENT_RUN_SEGMENTS {
        push(
            &mut items,
            "conflict-absent",
            "structure",
            "medium",
            "确认连续没有冲突用词的段落是留白",
            format!(
                "最长有 {} 个连续分段不带任何冲突用词（全文命中 {} 处）",
                src.conflict.longest_absent_run, src.conflict.total_hits
            ),
        );
    }
    if src.dialogue.silent_segments > 0 {
        push(
            &mut items,
            "dialogue-silent",
            "structure",
            "low",
            "确认整段没有对白的位置是叙述该占的地方",
            format!(
                "{} 个分段几乎没有对白；最长一段连续 {} 字没有出现引号内的话",
                src.dialogue.silent_segments, src.dialogue.longest_quiet_chars
            ),
        );
    }

    // ── 语言 ───────────────────────────────────────────────────────────────────
    if src.cliche.total_hits > 0 {
        push(
            &mut items,
            "cliche-hits",
            "language",
            "medium",
            "逐条确认陈词滥调是留着还是换掉",
            format!(
                "{} 处命中，涉及 {} 个短语；其中重复出现的短语 {} 个（词库 {} 条模式）",
                src.cliche.total_hits,
                src.cliche.distinct_phrases,
                src.cliche.repeated_phrases,
                src.cliche.lexicon_size
            ),
        );
    }
    if !src.t0.top_repeated_phrases.is_empty() {
        let names: Vec<String> = src
            .t0
            .top_repeated_phrases
            .iter()
            .take(MAX_EXAMPLES)
            .map(|p| format!("「{}」{} 次", p.text, p.count))
            .collect();
        push(
            &mut items,
            "t0-repeat",
            "language",
            "low",
            "确认反复出现的短语是有意的复沓",
            format!(
                "{} 个连续短语出现 5 次以上：{}",
                src.t0.top_repeated_phrases.len(),
                join(&names)
            ),
        );
    }
    if let Some((word, count)) = src.t0.top_adverbs.first() {
        if *count >= ADVERB_OVERUSE {
            push(
                &mut items,
                "t0-adverb",
                "language",
                "low",
                "确认最高频的副词不是顺手加的口癖",
                format!("「{}」出现 {} 次，为全文最高的一个副词", word, count),
            );
        }
    }
    if src.t0.sentence_count > 0 && src.t0.readability.score < READABILITY_FLOOR {
        push(
            &mut items,
            "t0-readability",
            "language",
            "medium",
            "确认语言难度对得上目标读者",
            format!(
                "可读性 {:.0}/100（等级「{}」，算法 {}），平均句长 {:.1} 字",
                src.t0.readability.score,
                src.t0.readability.level,
                src.t0.readability.method,
                src.t0.avg_sentence_chars
            ),
        );
    }
    if let Some((open, close)) = curly_quotes(src.voice) {
        if open != close {
            push(
                &mut items,
                "voice-quotes",
                "language",
                "medium",
                "确认引号成对闭合",
                format!(
                    "前引号“ {} 个，后引号” {} 个，相差 {} 个（只比对这一对，\
                     日式引号与书名号不参与配对）",
                    open,
                    close,
                    open.abs_diff(close)
                ),
            );
        }
    }
    if src.voice.stance.verdict == "hedging" {
        push(
            &mut items,
            "voice-hedging",
            "language",
            "low",
            "确认叙述者的保留态度是设定，不是行文习惯",
            format!(
                "犹疑用词 {} 处（每万字 {:.1}），断言用词 {} 处（每万字 {:.1}）",
                src.voice.stance.hedging,
                src.voice.stance.hedging_per_10k,
                src.voice.stance.assertive,
                src.voice.stance.assertive_per_10k
            ),
        );
    }

    // ── 读者提示 ───────────────────────────────────────────────────────────────
    if src.explicit.total_hits > 0 && src.explicit.advisory != "none" {
        let cats = &src.explicit.advisory_categories;
        push(
            &mut items,
            "explicit-advisory",
            "advisory",
            "high",
            "确认要不要给读者一句提示",
            format!(
                "{} 处命中，用词强度 {}；{}（词库 {} 条模式）",
                src.explicit.total_hits,
                advisory_label(&src.explicit.advisory),
                if cats.is_empty() {
                    "没有需要点名的类别".to_string()
                } else {
                    format!("涉及类别 {}", join(cats))
                },
                src.explicit.lexicon_size
            ),
        );
    }
    let self_harm: Vec<String> = src
        .explicit
        .issues
        .iter()
        .filter(|i| i.kind == "self_harm")
        .map(|i| i.section.clone())
        .collect();
    if !self_harm.is_empty() {
        push(
            &mut items,
            "explicit-self-harm",
            "advisory",
            "high",
            "确认自伤描写旁要不要附求助信息",
            format!("{} 处自伤类命中：{}", self_harm.len(), join(&self_harm)),
        );
    }

    let groups = CHECKLIST_GROUPS
        .iter()
        .map(|(key, _)| {
            let count = items.iter().filter(|i| i.group == *key).count() as u32;
            ((*key).to_string(), count)
        })
        .collect();

    ReviewChecklist { items, groups }
}

/// One item, appended. Kept as a function so a rule stays one call and the order of
/// the list remains the order of the checks above.
fn push(
    items: &mut Vec<ChecklistItem>,
    id: &str,
    group: &str,
    severity: &str,
    label: &str,
    detail: String,
) {
    items.push(ChecklistItem {
        id: id.to_string(),
        group: group.to_string(),
        label: label.to_string(),
        detail,
        severity: severity.to_string(),
    });
}

/// The first few names, then how many more there were — a list that hides the rest
/// says that it hid them.
fn examples(names: &[String]) -> String {
    let shown: Vec<String> = names.iter().take(MAX_EXAMPLES).cloned().collect();
    let mut text = join(&shown);
    if names.len() > MAX_EXAMPLES {
        text.push_str(&format!(" 等 {} 项", names.len()));
    }
    text
}

fn join(names: &[String]) -> String {
    names.join("、")
}

/// The 前引号 / 后引号 counts, when the voice pass reported both.
fn curly_quotes(voice: &VoiceStats) -> Option<(u32, u32)> {
    let count = |label: &str| {
        voice
            .marks
            .iter()
            .find(|m| m.label == label)
            .map(|m| m.count)
    };
    Some((count("前引号")?, count("后引号")?))
}

fn structure_kind_label(kind: &str) -> String {
    match kind {
        "no_early_turn" => "前段没有第一次转向".to_string(),
        "missing_midpoint" => "中点没有转向".to_string(),
        "climax_early" => "高潮来得偏早".to_string(),
        "no_descent" => "高潮之后没有回落".to_string(),
        "act_imbalance" => "三幕长度失衡".to_string(),
        "too_few_landmarks" => "可定位的转折点太少".to_string(),
        other => other.to_string(),
    }
}

fn advisory_label(advisory: &str) -> &'static str {
    match advisory {
        "mild" => "轻",
        "moderate" => "中",
        "strong" => "重",
        _ => "无",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        run_cliche_analysis, run_conflict_analysis, run_dialogue_analysis, run_explicit_analysis,
        run_pacing_analysis, run_plot_analysis, run_setting_analysis, run_structure_analysis,
        run_t0_analysis, run_timeline_analysis, run_voice_analysis,
    };
    use studio_core::document::SemanticBlock;
    use studio_core::noun_signal::NounSignal;

    fn signal(text: &str) -> NounSignal {
        NounSignal {
            text: text.into(),
            pos: "NOUN".into(),
            syntactic_role: None,
            score: 0.5,
            span: (0, 0),
            evidence: None,
        }
    }

    fn block(path: &str, content: &str, signals: &[&str]) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: content.into(),
            section_path: path.into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: signals.iter().map(|t| signal(t)).collect(),
        }
    }

    fn doc_of(blocks: &[SemanticBlock]) -> studio_core::document::DocumentData {
        studio_core::document::DocumentData {
            doc_id: uuid::Uuid::nil(),
            title: "Test".into(),
            blocks: blocks.to_vec(),
            total_word_count: 0,
            total_char_count: 0,
        }
    }

    /// The roll-up over a real set of dimensions — the same wiring the analysis
    /// command uses, so a rule cannot pass here and miss there.
    fn checklist_of(blocks: &[SemanticBlock]) -> ReviewChecklist {
        let doc = doc_of(blocks);
        let t0 = run_t0_analysis(&doc).unwrap();
        let plot = run_plot_analysis(&doc).unwrap();
        let pacing = run_pacing_analysis(&doc).unwrap();
        let conflict = run_conflict_analysis(&doc).unwrap();
        let setting = run_setting_analysis(&doc).unwrap();
        let cliche = run_cliche_analysis(&doc).unwrap();
        let dialogue = run_dialogue_analysis(&doc).unwrap();
        let explicit = run_explicit_analysis(&doc).unwrap();
        let structure = run_structure_analysis(&doc, &pacing, &plot).unwrap();
        let timeline = run_timeline_analysis(&doc).unwrap();
        let voice = run_voice_analysis(&doc).unwrap();
        run_review_checklist(&ChecklistSource {
            t0: &t0,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &voice,
        })
    }

    /// `count` sections of `blocks_per` paragraphs each, long enough for the other
    /// dimensions to have something to say.
    fn sections(count: usize, blocks_per: usize) -> Vec<SemanticBlock> {
        (0..count)
            .flat_map(|s| {
                (0..blocks_per).map(move |_| {
                    let body: String = (0..90)
                        .map(|i| if i % 9 == 8 { '，' } else { '字' })
                        .collect();
                    block(&format!("第{}章", s + 1), &format!("{body}。{body}。"), &[])
                })
            })
            .collect()
    }

    fn ids(list: &ReviewChecklist) -> Vec<String> {
        list.items.iter().map(|i| i.id.clone()).collect()
    }

    #[test]
    fn test_the_checklist_carries_no_score() {
        let json = serde_json::to_string(&checklist_of(&sections(4, 3))).unwrap();
        assert!(
            !json.contains("score"),
            "an item is a question, not a grade: {json}"
        );
    }

    #[test]
    fn test_every_item_states_what_to_confirm_and_with_which_number() {
        for item in checklist_of(&sections(3, 4)).items {
            assert!(
                !item.label.trim().is_empty(),
                "{} has nothing to do",
                item.id
            );
            assert!(!item.detail.trim().is_empty(), "{} has no number", item.id);
            assert!(
                !item.detail.contains("NaN") && !item.detail.contains("inf"),
                "{} printed a non-finite number: {}",
                item.id,
                item.detail
            );
            assert!(
                CHECKLIST_GROUPS.iter().any(|(k, _)| *k == item.group),
                "{} sits in an unknown group {}",
                item.id,
                item.group
            );
        }
    }

    #[test]
    fn test_an_empty_document_still_lists_the_four_groups() {
        let list = checklist_of(&[]);
        let keys: Vec<&str> = list.groups.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            vec!["consistency", "structure", "language", "advisory"]
        );
        let counted: u32 = list.groups.iter().map(|(_, c)| *c).sum();
        assert_eq!(counted as usize, list.items.len());
        // A document with no shape to read is itself the thing to confirm.
        assert!(ids(&list).contains(&"structure-unreadable".to_string()));
    }

    #[test]
    fn test_a_thread_that_never_pays_off_is_listed_with_its_name() {
        let mut blocks = sections(4, 3);
        // Three mentions: below the plot pass's own floor a name is noise, not a thread.
        for _ in 0..3 {
            blocks[1].noun_signals.push(signal("铜钥匙"));
        }
        let list = checklist_of(&blocks);
        let item = list
            .items
            .iter()
            .find(|i| i.id == "plot-unresolved")
            .unwrap_or_else(|| panic!("a one-section thread should be listed: {:?}", ids(&list)));
        assert_eq!(item.group, "consistency");
        assert_eq!(item.severity, "high");
        assert!(
            item.detail.contains("铜钥匙"),
            "the item has to say which thread: {}",
            item.detail
        );
    }

    #[test]
    fn test_ids_are_unique_so_a_tick_lands_on_one_item() {
        let list = checklist_of(&sections(5, 4));
        let mut seen = ids(&list);
        let total = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), total, "duplicate item id in {:?}", ids(&list));
    }

    #[test]
    fn test_an_unclosed_quote_is_a_line_on_the_list() {
        let mut blocks = sections(3, 3);
        blocks[0].content = "他说：“第一句。然后：“第二句。".into();
        let list = checklist_of(&blocks);
        assert!(ids(&list).contains(&"voice-quotes".to_string()));

        let balanced = checklist_of(&[block("第一章", "他说：“走了。”她答：“嗯。”", &[])]);
        assert!(
            !ids(&balanced).contains(&"voice-quotes".to_string()),
            "paired quotes are not a finding: {:?}",
            ids(&balanced)
        );
    }

    #[test]
    fn test_the_same_text_gives_the_same_list() {
        let blocks = sections(4, 3);
        let a = serde_json::to_string(&checklist_of(&blocks)).unwrap();
        let b = serde_json::to_string(&checklist_of(&blocks)).unwrap();
        assert_eq!(a, b);
    }
}
