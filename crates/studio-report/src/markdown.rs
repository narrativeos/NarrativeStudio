//! Markdown report rendering.
//!
//! A pure function over the analysis output: the same `T0Stats`/`T1Stats`/
//! `PlotStats`/`Assessment` plus the same timestamp produce byte-identical Markdown.
//! That is what makes the report testable against a golden string, and it is why the
//! timestamp is an input rather than read from the clock inside.

use chrono::{DateTime, Utc};

use studio_analysis::{
    Assessment, ClicheStats, ConflictStats, DialogueStats, ExplicitStats, PacingStats, PlotStats,
    SettingStats, StructureStats, T0Stats, T1Stats, ThemeStats, TimelineIssue, TimelineStats,
    VoiceMark, VoiceStats, ANALYSIS_VERSION,
};

/// Everything a report is rendered from. All borrowed: the renderer never owns an
/// analysis, so a report can be produced straight from cached rows.
pub struct ReportInput<'a> {
    pub project_name: &'a str,
    pub generated_at: DateTime<Utc>,
    pub t0: &'a T0Stats,
    pub t1: &'a T1Stats,
    pub plot: &'a PlotStats,
    pub pacing: &'a PacingStats,
    pub conflict: &'a ConflictStats,
    pub theme: &'a ThemeStats,
    pub setting: &'a SettingStats,
    pub cliche: &'a ClicheStats,
    pub dialogue: &'a DialogueStats,
    pub explicit: &'a ExplicitStats,
    pub structure: &'a StructureStats,
    pub timeline: &'a TimelineStats,
    pub voice: &'a VoiceStats,
    pub assessment: &'a Assessment,
}

/// Long lists are cut off so the document stays readable; the count of what was
/// left out is always stated, so nothing disappears silently.
const MAX_ROWS: usize = 20;

/// Render the full report.
pub fn render_markdown(input: &ReportInput) -> String {
    let mut out = String::new();
    write_header(&mut out, input);
    write_overall(&mut out, input.assessment);
    write_recommendations(&mut out, input.assessment);
    write_text_stats(&mut out, input.t0);
    write_diagnostics(&mut out, input.t0);
    write_dialogue(&mut out, input.dialogue);
    write_structure(&mut out, input.t1);
    write_narrative(&mut out, input.t1);
    write_plot(&mut out, input.plot);
    write_pacing(&mut out, input.pacing);
    write_conflict(&mut out, input.conflict);
    write_theme(&mut out, input.theme);
    write_setting(&mut out, input.setting);
    write_cliche(&mut out, input.cliche);
    write_explicit(&mut out, input.explicit);
    write_story_structure(&mut out, input.structure);
    write_timeline(&mut out, input.timeline);
    write_voice(&mut out, input.voice);
    write_footer(&mut out);
    out
}

fn write_header(out: &mut String, input: &ReportInput) {
    out.push_str(&format!("# {} · 叙事分析报告\n\n", input.project_name));
    out.push_str(&format!(
        "- 生成时间：{} UTC\n- 分析版本：{}\n\n",
        input.generated_at.format("%Y-%m-%d %H:%M"),
        ANALYSIS_VERSION
    ));
}

fn write_overall(out: &mut String, assessment: &Assessment) {
    out.push_str("## 总体评估\n\n");
    out.push_str(&format!(
        "总体评分：**{:.1} / 100**\n\n",
        assessment.overall
    ));
    out.push_str("| 维度 | 评分 | 说明 |\n| --- | ---: | --- |\n");
    for d in &assessment.dimensions {
        out.push_str(&format!(
            "| {} | {:.1} | {} |\n",
            esc(&d.label),
            d.score,
            esc(&d.detail)
        ));
    }
    out.push('\n');
}

fn write_recommendations(out: &mut String, assessment: &Assessment) {
    out.push_str("## 关键建议\n\n");
    if assessment.recommendations.is_empty() {
        out.push_str("本次分析未发现需要处理的问题。\n\n");
        return;
    }
    let count = |level: &str| {
        assessment
            .recommendations
            .iter()
            .filter(|r| r.severity == level)
            .count()
    };
    out.push_str(&format!(
        "共 {} 项（高 {} · 中 {} · 低 {}），按优先级排列：\n\n",
        assessment.recommendations.len(),
        count("high"),
        count("medium"),
        count("low")
    ));
    for r in &assessment.recommendations {
        out.push_str(&format!(
            "- **{} · {}**（{}）— {}\n",
            severity_label(&r.severity),
            esc(&r.title),
            esc(&r.dimension),
            esc(&r.detail)
        ));
    }
    out.push('\n');
}

fn write_text_stats(out: &mut String, t0: &T0Stats) {
    out.push_str("## 文本统计\n\n");
    out.push_str("| 指标 | 数值 |\n| --- | ---: |\n");
    let rows: [(&str, String); 8] = [
        ("字数", t0.char_count.to_string()),
        ("词数", t0.word_count.to_string()),
        ("块数", t0.block_count.to_string()),
        ("句子数", t0.sentence_count.to_string()),
        ("平均句长（字）", format!("{:.1}", t0.avg_sentence_chars)),
        ("平均块长（字）", format!("{:.1}", t0.avg_block_length)),
        (
            "可读性",
            format!("{:.1}（{}）", t0.readability.score, t0.readability.level),
        ),
        (
            "副词 / 形容词",
            format!("{} / {}", t0.adverb_count, t0.adjective_count),
        ),
    ];
    for (label, value) in rows {
        out.push_str(&format!("| {} | {} |\n", label, value));
    }
    out.push('\n');

    if !t0.sentence_length_distribution.is_empty() {
        out.push_str("句子长度分布：\n\n");
        out.push_str("| 句长 | 句子数 | 占比 |\n| --- | ---: | ---: |\n");
        let total: u64 = t0
            .sentence_length_distribution
            .iter()
            .map(|(_, c)| *c as u64)
            .sum();
        for (label, count) in &t0.sentence_length_distribution {
            let share = if total > 0 {
                *count as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            out.push_str(&format!("| {} | {} | {:.1}% |\n", esc(label), count, share));
        }
        out.push('\n');
    }
}

fn write_diagnostics(out: &mut String, t0: &T0Stats) {
    out.push_str("## 文本诊断\n\n");
    write_pair_section(out, "高频词", &t0.top_words, 15);
    write_pair_section(out, "高频实体", &t0.top_entities, 15);
    write_pair_section(out, "高频名词信号", &t0.top_noun_signals, 15);
    write_pair_section(out, "副词", &t0.top_adverbs, 10);
    write_pair_section(out, "形容词", &t0.top_adjectives, 10);

    out.push_str(&format!(
        "### 重复短语（{} 项）\n\n",
        t0.top_repeated_phrases.len()
    ));
    if t0.top_repeated_phrases.is_empty() {
        out.push_str("未发现高频重复短语。\n\n");
    } else {
        out.push_str("| 短语 | 出现次数 |\n| --- | ---: |\n");
        for p in t0.top_repeated_phrases.iter().take(MAX_ROWS) {
            out.push_str(&format!("| {} | {} |\n", esc(&p.text), p.count));
        }
        out.push_str(&omitted(t0.top_repeated_phrases.len(), MAX_ROWS));
        out.push('\n');
    }
}

fn write_dialogue(out: &mut String, dialogue: &DialogueStats) {
    out.push_str("## 对话与叙事\n\n");
    out.push_str(&format!(
        "对白占比：**{:.1}%** · 叙述 {:.1}% · 对白 {} 处（引号 {} · 破折号 {}）· 判定：**{}**（`{}`）\n\n",
        dialogue.dialogue_share * 100.0,
        dialogue.narrative_share * 100.0,
        dialogue.unit_count,
        dialogue.quoted_units,
        dialogue.dash_units,
        dialogue_verdict_label(&dialogue.verdict),
        dialogue.verdict
    ));
    out.push_str(&format!(
        "全文 {} 字：对白 {} 字 · 叙述 {} 字 · 平均每处 {:.1} 字 · 最长一处 {} 字 · 含对白的文本块 {} / {}\n\n",
        dialogue.char_count,
        dialogue.dialogue_chars,
        dialogue.narrative_chars,
        dialogue.avg_unit_chars,
        dialogue.longest_unit,
        dialogue.blocks_with_dialogue,
        dialogue.blocks
    ));

    if dialogue.peak_section.is_empty() {
        // No segment long enough to compare: a share computed over 40 characters is a
        // rounding error, so the comparison is withheld rather than invented.
        out.push_str("各段均不足 300 字，不比较段落间的对白差异。\n\n");
    } else {
        out.push_str(&format!(
            "最密：{}（{:.0}%）· 最疏：{}（{:.0}%）· 相差 {:.0}% · 连续无对白最长 {} 块 / {} 字 · 连续对白最长 {} 块\n\n",
            esc(&dialogue.peak_section),
            dialogue.peak_share * 100.0,
            esc(&dialogue.quietest_section),
            dialogue.quietest_share * 100.0,
            dialogue.spread * 100.0,
            dialogue.longest_quiet_run,
            dialogue.longest_quiet_chars,
            dialogue.longest_dialogue_run
        ));
    }

    out.push_str(&format!(
        "### 对白占比曲线（{} 段）\n\n",
        dialogue.segment_count
    ));
    if dialogue.points.is_empty() {
        out.push_str("无数据。\n\n");
    } else {
        out.push_str(
            "| 段落 | 占比 | 对白字数 | 总字数 | 处数 | 最长一处 |\n| --- | --- | ---: | ---: | ---: | ---: |\n",
        );
        for p in dialogue.points.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} {:.2} | {} | {} | {} | {} |\n",
                esc(&p.section),
                bar(p.share),
                p.share,
                p.dialogue_chars,
                p.char_count,
                p.units,
                p.longest_unit
            ));
        }
        out.push_str(&omitted(dialogue.points.len(), MAX_ROWS));
        out.push('\n');
        if dialogue.chunked {
            out.push_str("_章节不足 3 个：曲线按等长块切分。_\n\n");
        }
    }

    if !dialogue.unit_length_distribution.is_empty() {
        out.push_str("### 对白长度分布\n\n");
        out.push_str("| 每处字数 | 处数 |\n| --- | ---: |\n");
        for (label, count) in &dialogue.unit_length_distribution {
            out.push_str(&format!("| {} | {} |\n", esc(label), count));
        }
        out.push('\n');
    }

    if !dialogue.samples.is_empty() {
        out.push_str(&format!(
            "### 最长的几处对白（{} 处）\n\n",
            dialogue.samples.len()
        ));
        for s in dialogue.samples.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- **{} 字** · {} · {} · 「{}」\n",
                s.chars,
                esc(&s.section),
                dialogue_kind_label(&s.kind),
                esc(&s.excerpt)
            ));
        }
        out.push('\n');
    }

    if !dialogue.issues.is_empty() {
        out.push_str("### 对白问题\n\n");
        for i in dialogue.issues.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- **{}**（{}）— {}\n",
                dialogue_issue_label(&i.kind),
                if i.section.is_empty() {
                    "全文".to_string()
                } else {
                    esc(&i.section)
                },
                esc(&i.detail)
            ));
        }
        out.push('\n');
    }

    out.push_str("_对白按引号与破折号识别：原文不用引号时会漏判，引号本身计入叙述一侧。比例只说明对白与叙述的多少，不说明好坏。_\n\n");
}

fn write_structure(out: &mut String, t1: &T1Stats) {
    out.push_str("## 章节结构\n\n");
    out.push_str(&format!(
        "共 {} 个章节；最长：{}；最短：{}\n\n",
        t1.section_count,
        t1.longest_section.as_deref().unwrap_or("—"),
        t1.shortest_section.as_deref().unwrap_or("—")
    ));
    write_pair_section(out, "块类型分布", &t1.block_type_distribution, MAX_ROWS);

    out.push_str("### 章节一览\n\n");
    if t1.sections.is_empty() {
        out.push_str("未识别到章节结构。\n\n");
    } else {
        out.push_str("| 章节 | 块数 | 字数 |\n| --- | ---: | ---: |\n");
        for s in t1.sections.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                esc(&s.path),
                s.block_count,
                s.char_count
            ));
        }
        out.push_str(&omitted(t1.sections.len(), MAX_ROWS));
        out.push('\n');
    }
}

fn write_narrative(out: &mut String, t1: &T1Stats) {
    out.push_str("## 叙事分析\n\n");
    out.push_str(&format!(
        "叙事弧线形态：**{}**（`{}`）\n\n",
        arc_shape_label(&t1.arc.shape),
        t1.arc.shape
    ));
    if !t1.arc.points.is_empty() {
        out.push_str("| 章节 | 强度 | 字数 |\n| --- | --- | ---: |\n");
        for p in t1.arc.points.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} {:.2} | {} |\n",
                esc(&p.section),
                bar(p.intensity),
                p.intensity,
                p.char_count
            ));
        }
        out.push_str(&omitted(t1.arc.points.len(), MAX_ROWS));
        out.push('\n');
    }

    write_pair_section(out, "实体共现", &t1.entity_cooccurrences, 15);

    out.push_str(&format!("### 人物（{} 位）\n\n", t1.characters.len()));
    if t1.characters.is_empty() {
        out.push_str("未识别到人物。\n\n");
    } else {
        out.push_str(
            "| 人物 | 提及 | 覆盖跨度 | 首次出现 | 末次出现 |\n| --- | ---: | ---: | --- | --- |\n",
        );
        for c in t1.characters.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} | {:.0}% | {} | {} |\n",
                esc(&c.name),
                c.mentions,
                c.span_ratio * 100.0,
                esc(&c.first_section),
                esc(&c.last_section)
            ));
        }
        out.push_str(&omitted(t1.characters.len(), MAX_ROWS));
        out.push('\n');
    }
}

/// Plotlines, the sections where the cast changes, and the threads that never pay
/// off. The shape and score come straight from `studio_analysis::plot`; the report
/// only translates the keys into labels.
fn write_plot(out: &mut String, plot: &PlotStats) {
    out.push_str("## 情节线\n\n");
    out.push_str(&format!(
        "情节连贯性：**{:.1} / 100**；共 {} 条线索（{} 个章节）\n\n",
        plot.coherence_score, plot.thread_count, plot.section_count
    ));
    if plot.plotlines.is_empty() {
        out.push_str("未识别到情节线：没有实体或名词信号达到 3 次的提及门槛。\n\n");
        return;
    }
    out.push_str(
        "| 线索 | 类型 | 主线 | 形态 | 提及 | 覆盖 | 首次 → 末次 |\n| --- | --- | --- | --- | ---: | ---: | --- |\n",
    );
    for p in plot.plotlines.iter().take(MAX_ROWS) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.0}% | {} → {} |\n",
            esc(&p.subject),
            p.kind,
            if p.is_main { "是" } else { "—" },
            plot_shape_label(&p.shape),
            p.mentions,
            p.coverage * 100.0,
            esc(&p.first_section),
            esc(&p.last_section)
        ));
    }
    out.push_str(&omitted(plot.plotlines.len(), MAX_ROWS));
    out.push('\n');

    if !plot.turning_points.is_empty() {
        out.push_str(&format!(
            "### 转折点（{} 处）\n\n",
            plot.turning_points.len()
        ));
        out.push_str("| 章节 | 变化 | 登场 | 退场 |\n| --- | ---: | --- | --- |\n");
        for t in plot.turning_points.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {:.0}% | {} | {} |\n",
                esc(&t.section),
                t.change * 100.0,
                esc(&t.appeared.join("、")),
                esc(&t.dropped.join("、"))
            ));
        }
        out.push('\n');
    }

    // Both lists are the actionable part of this section: threads that were set up
    // and never picked up, and threads that arrive too late to be earned.
    if !plot.unresolved_threads.is_empty() {
        out.push_str(&format!(
            "铺垫未回收（{}）：{}\n\n",
            plot.unresolved_threads.len(),
            esc(&plot.unresolved_threads.join("、"))
        ));
    }
    if !plot.late_arrivals.is_empty() {
        out.push_str(&format!(
            "结尾登场（{}）：{}\n\n",
            plot.late_arrivals.len(),
            esc(&plot.late_arrivals.join("、"))
        ));
    }
}

/// The pacing curve, the stretches that never move and the chapters that drag.
///
/// The verdict is not the arc's shape: the arc says where the peak sits, this says
/// whether the curve moves at all. Tension is printed on the same 0-100 scale the
/// UI draws, so a number read on screen can be found in the report.
fn write_pacing(out: &mut String, pacing: &PacingStats) {
    out.push_str("## 节奏\n\n");
    out.push_str(&format!(
        "节奏评分：**{:.1} / 100**；判定：**{}**（`{}`）· {} 段 · 张力标准差 {:.1}\n\n",
        pacing.pace_score,
        pacing_verdict_label(&pacing.verdict),
        pacing.verdict,
        pacing.segment_count,
        pacing.tension_stdev
    ));
    if pacing.points.is_empty() {
        out.push_str("未识别到节奏曲线：文档没有可切分的段落。\n\n");
        return;
    }
    out.push_str(
        "| 章节 | 张力 | 字数 | 句子 | 平均句长 | 对话占比 |\n| --- | --- | ---: | ---: | ---: | ---: |\n",
    );
    for p in pacing.points.iter().take(MAX_ROWS) {
        out.push_str(&format!(
            "| {} | {} {:.0} | {} | {} | {:.1} | {:.0}% |\n",
            esc(&p.section),
            bar(p.tension / 100.0),
            p.tension,
            p.char_count,
            p.sentence_count,
            p.avg_sentence_chars,
            p.dialogue_share * 100.0
        ));
    }
    out.push_str(&omitted(pacing.points.len(), MAX_ROWS));
    out.push('\n');

    if !pacing.flat_stretches.is_empty() {
        out.push_str(&format!(
            "### 平淡段落（{} 处）\n\n",
            pacing.flat_stretches.len()
        ));
        out.push_str("| 起 | 止 | 段数 | 平均张力 | 字数 |\n| --- | --- | ---: | ---: | ---: |\n");
        for s in pacing.flat_stretches.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} | {} | {:.0} | {} |\n",
                esc(&s.from_section),
                esc(&s.to_section),
                s.sections,
                s.mean_tension,
                s.char_count
            ));
        }
        out.push('\n');
    }

    if !pacing.drag_sections.is_empty() {
        out.push_str(&format!(
            "### 拖沓章节（{} 处）\n\n",
            pacing.drag_sections.len()
        ));
        out.push_str("| 章节 | 字数 | 相对均长 | 张力 |\n| --- | ---: | ---: | ---: |\n");
        for d in pacing.drag_sections.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} | {:.1}× | {:.0} |\n",
                esc(&d.section),
                d.char_count,
                d.length_ratio,
                d.tension
            ));
        }
        out.push('\n');
    }

    if pacing.chunked {
        out.push_str(&format!(
            "文档章节不足，曲线按 {} 个等长块切分，不参与评分。\n\n",
            pacing.segment_count
        ));
    }
}

fn write_conflict(out: &mut String, conflict: &ConflictStats) {
    out.push_str("## 冲突\n\n");
    match conflict.conflict_score {
        Some(score) => out.push_str(&format!(
            "冲突评分：**{:.1} / 100** · 判定：**{}**（`{}`）· 命中 {} 处 · {:.1} 处/千字 · 主类型：{}\n\n",
            score,
            conflict_verdict_label(&conflict.verdict),
            conflict.verdict,
            conflict.total_hits,
            conflict.density,
            conflict_type_label(&conflict.dominant_type)
        )),
        // Absence is reported as absence: the score would be a guess about whether
        // the text is non-narrative or the lexicon is simply missing its words.
        None => out.push_str(
            "未检测到冲突用词，因此**不给出评分**：文本可能不是叙事文本，也可能是词库尚未覆盖。\n\n",
        ),
    }

    out.push_str(&format!("### 冲突类型（{} 类）\n\n", conflict.types.len()));
    if conflict.types.is_empty() {
        out.push_str("无数据。\n\n");
    } else {
        out.push_str(
            "| 类型 | 命中 | 占比 | 千字密度 | 高频词 |\n| --- | ---: | ---: | ---: | --- |\n",
        );
        for t in conflict.types.iter().take(MAX_ROWS) {
            let terms: Vec<String> = t
                .top_terms
                .iter()
                .take(6)
                .map(|(term, hits)| format!("{term}×{hits}"))
                .collect();
            out.push_str(&format!(
                "| {} | {} | {:.0}% | {:.1} | {} |\n",
                conflict_type_label(&t.kind),
                t.hits,
                t.share * 100.0,
                t.density,
                esc(&terms.join("、"))
            ));
        }
        out.push_str(&omitted(conflict.types.len(), MAX_ROWS));
        out.push('\n');
    }

    if conflict.points.is_empty() {
        out.push_str("未识别到冲突曲线：文档没有可切分的段落。\n\n");
        return;
    }
    out.push_str(&format!(
        "峰值：{}（{:.0}，位于全文 {:.0}%）· 结尾三分之一平均 {:.0} · 最长无冲突区间 {} 段\n\n",
        esc(&conflict.peak_section),
        conflict.peak_intensity,
        conflict.peak_position * 100.0,
        conflict.tail_intensity,
        conflict.longest_absent_run
    ));
    out.push_str("| 章节 | 强度 | 命中 | 千字密度 | 主类型 |\n| --- | --- | ---: | ---: | --- |\n");
    for p in conflict.points.iter().take(MAX_ROWS) {
        out.push_str(&format!(
            "| {} | {} {:.0} | {} | {:.1} | {} |\n",
            esc(&p.section),
            bar(p.intensity / 100.0),
            p.intensity,
            p.hits,
            p.density,
            conflict_type_label(&p.dominant)
        ));
    }
    out.push_str(&omitted(conflict.points.len(), MAX_ROWS));
    out.push('\n');

    if !conflict.absent_runs.is_empty() {
        out.push_str(&format!(
            "### 无冲突区间（{} 处）\n\n",
            conflict.absent_runs.len()
        ));
        out.push_str("| 起 | 止 | 段数 | 字数 |\n| --- | --- | ---: | ---: |\n");
        for r in conflict.absent_runs.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                esc(&r.from_section),
                esc(&r.to_section),
                r.sections,
                r.char_count
            ));
        }
        out.push('\n');
    }

    if conflict.chunked {
        out.push_str(&format!(
            "文档章节不足，曲线按 {} 个等长块切分。\n\n",
            conflict.segment_count
        ));
    }
    out.push_str("_冲突用词按词面计数：每个词权重相同，词库未覆盖的写法不计入。\n\n");
}

fn write_theme(out: &mut String, theme: &ThemeStats) {
    out.push_str("## 主题\n\n");
    match theme.theme_score {
        Some(score) => out.push_str(&format!(
            "主题评分：**{:.1} / 100** · 词汇来源：**{}**（`{}`）· {} 个主题 · 候选词命中 {} 次\n\n",
            score,
            theme_source_label(&theme.source),
            theme.source,
            theme.candidate_count,
            theme.total_mentions
        )),
        // No usable noun, or nothing that ever repeats: there is no theme list to
        // grade, and saying so beats a low score that reads as a weak subject.
        None => out.push_str(
            "没有可用于主题分析的词汇，或没有任何词反复出现，因此**不给出评分**。\n\n",
        ),
    }

    if theme.themes.is_empty() {
        out.push_str("未识别到主题：没有词达到 3 次提及且跨越 2 段以上。\n\n");
        return;
    }

    out.push_str(&format!(
        "最大主题占比 {:.0}% · 前三主题合计 {:.0}% · 仅出现一次的词贡献 {:.0}%\n\n",
        theme.top_share * 100.0,
        theme.top3_share * 100.0,
        theme.singleton_share * 100.0
    ));
    out.push_str(&format!("### 主题清单（{} 个）\n\n", theme.themes.len()));
    out.push_str(
        "| 主题 | 提及 | 覆盖 | 占比 | 首次 | 末次 | 贯穿度 | 分布 |\n| --- | ---: | ---: | ---: | --- | --- | --- | --- |\n",
    );
    for t in theme.themes.iter().take(MAX_ROWS) {
        out.push_str(&format!(
            "| {} | {} | {:.0}% | {:.1}% | {} | {} | {} | {} |\n",
            esc(&t.term),
            t.mentions,
            t.coverage * 100.0,
            t.share * 100.0,
            esc(&t.first_section),
            esc(&t.last_section),
            theme_reach_label(&t.reach),
            theme_concentration_label(&t.concentration)
        ));
    }
    out.push_str(&omitted(theme.themes.len(), MAX_ROWS));
    out.push('\n');

    let folded: Vec<String> = theme
        .themes
        .iter()
        .filter(|t| t.members.len() > 1)
        .take(6)
        .map(|t| {
            let parts: Vec<String> = t
                .members
                .iter()
                .map(|(m, count)| format!("{m}×{count}"))
                .collect();
            format!("{}：{}", esc(&t.term), parts.join("、"))
        })
        .collect();
    if !folded.is_empty() {
        out.push_str("同义与包含关系已合并：\n\n");
        for line in folded {
            out.push_str(&format!("- {line}\n"));
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "_主题按名词词汇的复现与分布计算（来源：{}）；反复出现不等于母题，是否有意为之需要人来判断。_\n\n",
        theme_source_label(&theme.source)
    ));
}

fn write_setting(out: &mut String, setting: &SettingStats) {
    out.push_str("## 场景/设定\n\n");
    match setting.setting_score {
        Some(score) => out.push_str(&format!(
            "场景评分：**{:.1} / 100** · {} 场 · {} 个地点 · 环境用词 {} 次（{:.1} 次/千字，判定：**{}**（`{}`））\n\n",
            score,
            setting.scene_count,
            setting.place_count,
            setting.total_env_hits,
            setting.env_density,
            setting_verdict_label(&setting.verdict),
            setting.verdict
        )),
        // Neither a named place nor a described one: the text never says where it
        // is, and there is nothing to grade.
        None => out
            .push_str("没有点名地点，也没有环境用词，因此**不给出评分**。\n\n"),
    }

    if setting.scenes.is_empty() {
        out.push_str("未识别到场景：文档没有可切分的段落。\n\n");
        return;
    }

    out.push_str(&format!(
        "没有地点锚点的场景：{}/{}（{:.0}%）· 主导地点：{} · 环境峰值：{}（{:.0}）\n\n",
        setting.unanchored_scenes,
        setting.scene_count,
        setting.unanchored_share * 100.0,
        if setting.dominant_place.is_empty() {
            "—".to_string()
        } else {
            esc(&setting.dominant_place)
        },
        if setting.peak_section.is_empty() {
            "—".to_string()
        } else {
            esc(&setting.peak_section)
        },
        setting.peak_intensity
    ));

    out.push_str(&format!("### 地点（{} 个）\n\n", setting.places.len()));
    if setting.places.is_empty() {
        out.push_str("没有反复出现的地点：所有地名都只出现过一次。\n\n");
    } else {
        out.push_str(
            "| 地点 | 类型 | 提及 | 覆盖 | 首次 | 末次 | 最长缺席 |\n| --- | --- | ---: | ---: | --- | --- | ---: |\n",
        );
        for p in setting.places.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} | {} | {:.0}% | {} | {} | {} |\n",
                esc(&p.name),
                place_kind_label(&p.kind),
                p.mentions,
                p.coverage * 100.0,
                esc(&p.first_section),
                esc(&p.last_section),
                p.longest_absence
            ));
        }
        out.push_str(&omitted(setting.places.len(), MAX_ROWS));
        out.push('\n');
    }

    out.push_str(&format!("### 场景清单（{} 场）\n\n", setting.scenes.len()));
    out.push_str(
        "| # | 段落 | 地点 | 块 | 字数 | 环境/千字 | 对话 | 时间标记 |\n| --- | --- | --- | ---: | ---: | ---: | ---: | --- |\n",
    );
    for s in setting.scenes.iter().take(MAX_ROWS) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.1} | {:.0}% | {} |\n",
            s.index + 1,
            esc(&s.section),
            if s.anchor.is_empty() {
                "（未点名）".to_string()
            } else {
                esc(&s.anchor)
            },
            s.blocks,
            s.char_count,
            s.env_density,
            s.dialogue_share * 100.0,
            if s.time_mark { "是" } else { "—" }
        ));
    }
    out.push_str(&omitted(setting.scene_count as usize, MAX_ROWS));
    out.push('\n');

    if !setting.issues.is_empty() {
        out.push_str("### 场景问题\n\n");
        for i in setting.issues.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- **{}**（{}）— {}\n",
                setting_issue_label(&i.kind),
                if i.section.is_empty() {
                    "全文".to_string()
                } else {
                    esc(&i.section)
                },
                esc(&i.detail)
            ));
        }
        out.push('\n');
    }

    if !setting.top_env_terms.is_empty() {
        let parts: Vec<String> = setting
            .top_env_terms
            .iter()
            .take(8)
            .map(|(term, count)| format!("{term}×{count}"))
            .collect();
        out.push_str(&format!("环境用词：{}\n\n", parts.join("、")));
    }

    out.push_str("_场景按地点实体与环境用词切分：地名之外的地点只能靠描写判断，是否真的换了场景需要人来核对。_\n\n");
}

fn write_cliche(out: &mut String, cliche: &ClicheStats) {
    out.push_str("## 陈词滥调\n\n");
    match cliche.cliche_score {
        Some(score) => {
            // The library size is printed only when it found something: "150 项"
            // means nothing next to a document with no hits, and keeping it out of
            // that case also keeps the empty report stable as the library grows.
            let library = if cliche.total_hits > 0 {
                format!(" · 词库 {} 项", cliche.lexicon_size)
            } else {
                String::new()
            };
            out.push_str(&format!(
                "用词评分：**{:.1} / 100** · 命中 {} 处（{:.1} 处/万字，判定：**{}**（`{}`））· 涉及 {} 个短语 · 对白内 {} 处{library}\n\n",
                score,
                cliche.total_hits,
                cliche.per_10k,
                cliche_verdict_label(&cliche.verdict),
                cliche.verdict,
                cliche.distinct_phrases,
                cliche.dialogue_hits
            ))
        }
        // Too little text for a rate per 10 000 characters to mean anything. The
        // hits are still listed below; only the grade is withheld.
        None => out.push_str(&format!(
            "全文 {} 字，不足 1000 字，按万字密度评分没有意义，因此**不给出评分**。\n\n",
            cliche.char_count
        )),
    }

    out.push_str(&format!(
        "最常用：{}（占命中 {:.0}%）· 峰值段落：{}（{:.0}）· 反复使用的短语 {} 个\n\n",
        if cliche.top_phrase.is_empty() {
            "—".to_string()
        } else {
            format!("「{}」", esc(&cliche.top_phrase))
        },
        cliche.top_share * 100.0,
        if cliche.peak_section.is_empty() {
            "—".to_string()
        } else {
            esc(&cliche.peak_section)
        },
        cliche.peak_intensity,
        cliche.repeated_phrases
    ));

    if !cliche.categories.is_empty() {
        let parts: Vec<String> = cliche
            .categories
            .iter()
            .map(|(k, v)| format!("{}×{}", cliche_category_label(k), v))
            .collect();
        out.push_str(&format!("类型分布：{}\n\n", parts.join("、")));
    }

    out.push_str(&format!("### 高频套话（{} 个）\n\n", cliche.phrases.len()));
    if cliche.phrases.is_empty() {
        out.push_str("没有命中词库里的表达。\n\n");
    } else {
        out.push_str(
            "| 短语 | 类型 | 次数 | 占比 | 覆盖 | 首次 | 末次 |\n| --- | --- | ---: | ---: | ---: | --- | --- |\n",
        );
        for p in cliche.phrases.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} | {} | {:.0}% | {:.0}% | {} | {} |\n",
                esc(&p.phrase),
                cliche_category_label(&p.category),
                p.count,
                p.share * 100.0,
                p.coverage * 100.0,
                esc(&p.first_section),
                esc(&p.last_section)
            ));
        }
        out.push_str(&omitted(cliche.distinct_phrases as usize, MAX_ROWS));
        out.push('\n');

        out.push_str("### 怎么改\n\n");
        for p in cliche.phrases.iter().take(6) {
            out.push_str(&format!("- **{}** — {}\n", esc(&p.phrase), esc(&p.note)));
        }
        out.push('\n');
    }

    if !cliche.hits.is_empty() {
        out.push_str(&format!(
            "### 命中位置（前 {} 处）\n\n",
            cliche.hits.len().min(MAX_ROWS)
        ));
        for h in cliche.hits.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- **{}** · {} · {}（第 {} 字）{}\n",
                esc(&h.section),
                esc(&h.phrase),
                esc(&h.excerpt),
                h.offset,
                if h.in_dialogue { "· 对白内" } else { "" }
            ));
        }
        out.push('\n');
    }

    if !cliche.issues.is_empty() {
        out.push_str("### 用词问题\n\n");
        for i in cliche.issues.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- **{}**（{}）— {}\n",
                cliche_issue_label(&i.kind),
                if i.section.is_empty() {
                    "全文".to_string()
                } else {
                    esc(&i.section)
                },
                esc(&i.detail)
            ));
        }
        out.push('\n');
    }

    out.push_str("_命中只说明字面撞上词库里的说法，不等于写得差；是否真的是陈词滥调、要不要改，由人来判断。_\n\n");
}

fn write_explicit(out: &mut String, explicit: &ExplicitStats) {
    out.push_str("## 显式语言\n\n");
    let library = if explicit.total_hits > 0 {
        format!(
            " · 词库 {} 项 / {} 个形变",
            explicit.lexicon_size, explicit.lexicon_variants
        )
    } else {
        String::new()
    };
    out.push_str(&format!(
        "内容分级：**{}**（`{}`）· 命中 {} 处（{:.1} 处/万字）· 涉及 {} 个词条 · 对白内 {} 处{library}\n\n",
        explicit_advisory_label(&explicit.advisory),
        explicit.advisory,
        explicit.total_hits,
        explicit.per_10k,
        explicit.distinct_terms,
        explicit.dialogue_hits
    ));

    // The three tiers always print, including when a document carries none of them:
    // the shape of the line is what tells a reader that a zero is a zero and not a
    // missing measurement.
    let tiers: Vec<String> = explicit
        .tiers
        .iter()
        .map(|(tier, count)| format!("{} {} 处", explicit_tier_label(tier), count))
        .collect();
    out.push_str(&format!("按强度：{}\n\n", tiers.join(" · ")));

    out.push_str(&format!(
        "最常用：{}（占命中 {:.0}%）· 峰值段落：{}（{:.0}）\n\n",
        if explicit.top_term.is_empty() {
            "—".to_string()
        } else {
            format!("「{}」", esc(&explicit.top_term))
        },
        explicit.top_share * 100.0,
        if explicit.peak_section.is_empty() {
            "—".to_string()
        } else {
            esc(&explicit.peak_section)
        },
        explicit.peak_intensity
    ));

    if !explicit.advisory_categories.is_empty() {
        let parts: Vec<String> = explicit
            .advisory_categories
            .iter()
            .map(|k| explicit_category_label(k).to_string())
            .collect();
        out.push_str(&format!(
            "需要读者提示的内容：{}（按词库分级，只说明词面）\n\n",
            parts.join("、")
        ));
    }

    if !explicit.categories.is_empty() {
        let parts: Vec<String> = explicit
            .categories
            .iter()
            .map(|(k, v)| format!("{}×{}", explicit_category_label(k), v))
            .collect();
        out.push_str(&format!("类型分布：{}\n\n", parts.join("、")));
    }

    out.push_str(&format!("### 命中词条（{} 个）\n\n", explicit.terms.len()));
    if explicit.terms.is_empty() {
        out.push_str("没有命中词库里的用词。\n\n");
    } else {
        out.push_str(
            "| 词条 | 类型 | 强度 | 次数 | 占比 | 覆盖 | 首次 | 末次 |\n| --- | --- | --- | ---: | ---: | ---: | --- | --- |\n",
        );
        for t in explicit.terms.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {:.0}% | {:.0}% | {} | {} |\n",
                esc(&t.term),
                explicit_category_label(&t.category),
                explicit_tier_label(&t.tier),
                t.count,
                t.share * 100.0,
                t.coverage * 100.0,
                esc(&t.first_section),
                esc(&t.last_section)
            ));
        }
        out.push_str(&omitted(explicit.distinct_terms as usize, MAX_ROWS));
        out.push('\n');

        out.push_str("### 词库说明\n\n");
        for t in explicit.terms.iter().take(6) {
            out.push_str(&format!("- **{}** — {}\n", esc(&t.term), esc(&t.note)));
        }
        out.push('\n');
    }

    if !explicit.hits.is_empty() {
        out.push_str(&format!(
            "### 命中位置（前 {} 处）\n\n",
            explicit.hits.len().min(MAX_ROWS)
        ));
        for h in explicit.hits.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- **{}** · {} · {}（第 {} 字）{} · {}\n",
                esc(&h.section),
                esc(&h.term),
                esc(&h.excerpt),
                h.offset,
                explicit_tier_label(&h.tier),
                if h.in_dialogue { "对白内" } else { "叙述" }
            ));
        }
        out.push('\n');
    }

    if !explicit.issues.is_empty() {
        out.push_str("### 内容提示\n\n");
        for i in explicit.issues.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- **{}**（{}）— {}\n",
                explicit_issue_label(&i.kind),
                if i.section.is_empty() {
                    "全文".to_string()
                } else {
                    esc(&i.section)
                },
                esc(&i.detail)
            ));
        }
        out.push('\n');
    }

    out.push_str("_本维度不给出评分：一部战争题材并不比田园题材更差，露骨与否是内容属性而不是质量。命中只说明词面撞上词库，不等于剧情如此；分级只反映用词强度，不反映事件本身，是否需要读者提示由人决定。_\n\n");
}

/// The document's own landmarks and how the classic shapes fit them.
///
/// No score line: the section describes where things sit. Every judgement word (吻合,
/// 未见) is defined by the counts next to it, and the closing note says what the method
/// cannot see, so a reader cannot take 未见 for 缺少.
fn write_story_structure(out: &mut String, structure: &StructureStats) {
    out.push_str("## 故事结构\n\n");
    let cut = if structure.chunked {
        "按字数等分，非作者章节"
    } else {
        "按作者章节"
    };
    out.push_str(&format!(
        "地标 {} 个 · 转折点 {} 个 · 曲线段落 {} 段（{}）· 可见正文 {} 字\n\n",
        structure.landmarks.len(),
        structure.turn_count,
        structure.segment_count,
        cut,
        structure.char_count
    ));

    if !structure.readable {
        out.push_str(&format!("{}本次不做模型对照。\n\n", esc(&structure.note)));
        out.push_str(&structure_limits(structure));
        return;
    }

    out.push_str(&format!("### 地标（{} 个）\n\n", structure.landmarks.len()));
    if structure.landmarks.is_empty() {
        out.push_str("线程变化与张力曲线都没有指出任何地标。\n\n");
    } else {
        out.push_str("| 位置 | 段落 | 类型 | 依据 |\n| ---: | --- | --- | --- |\n");
        for l in structure.landmarks.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {:.0}% | {} | {} | {} |\n",
                l.position * 100.0,
                esc(&l.section),
                structure_landmark_label(&l.kind),
                esc(&l.detail)
            ));
        }
        out.push_str(&omitted(structure.landmarks.len(), MAX_ROWS));
        out.push('\n');
    }

    out.push_str("### 幕的切分（按本文自己的转折点）\n\n");
    if structure.acts.is_empty() {
        out.push_str("可辨的转折点不足两个，本文没有可切的三幕。\n\n");
    } else {
        out.push_str(
            "| 幕 | 起 | 止 | 字数 | 占比 | 张力（低/均/高） |\n| --- | --- | --- | ---: | ---: | --- | --- |\n",
        );
        for a in &structure.acts {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {:.0}% | {:.0} / {:.0} / {:.0} |\n",
                structure_act_label(&a.name),
                esc(&a.from_section),
                esc(&a.to_section),
                a.char_count,
                a.share * 100.0,
                a.min_tension,
                a.mean_tension,
                a.max_tension
            ));
        }
        out.push('\n');
    }

    out.push_str("### 经典模型对照\n\n");
    for m in &structure.models {
        out.push_str(&format!(
            "- **{}**：{}（命中 {} · 错位 {} · 未见 {}）\n",
            structure_model_label(&m.model),
            structure_fit_label(&m.fit),
            m.found,
            m.displaced,
            m.missing
        ));
    }
    out.push('\n');

    for m in &structure.models {
        out.push_str(&format!(
            "#### {}：节拍落点\n\n| 节拍 | 期望区间 | 判定 | 落点 | 偏差 | 依据 |\n| --- | ---: | --- | --- | ---: | --- |\n",
            structure_model_label(&m.model)
        ));
        for b in &m.beats {
            out.push_str(&format!(
                "| {} | {:.0}%–{:.0}% | {} | {} | {} | {} |\n",
                structure_beat_label(&b.beat),
                b.window.0 * 100.0,
                b.window.1 * 100.0,
                structure_status_label(&b.status),
                if b.section.is_empty() {
                    "—".to_string()
                } else {
                    format!("{}（{:.0}%）", esc(&b.section), b.position * 100.0)
                },
                if b.offset > 0.0 {
                    format!("{:.0}%", b.offset * 100.0)
                } else {
                    "—".to_string()
                },
                if b.evidence.is_empty() {
                    "—".to_string()
                } else {
                    esc(&b.evidence)
                }
            ));
        }
        out.push('\n');
    }

    if !structure.issues.is_empty() {
        out.push_str("### 结构提示\n\n");
        for i in structure.issues.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- **{}**（{}）— {}\n",
                structure_issue_label(&i.kind),
                if i.section.is_empty() {
                    "全文".to_string()
                } else {
                    esc(&i.section)
                },
                esc(&i.detail)
            ));
        }
        out.push('\n');
    }

    out.push_str(&structure_limits(structure));
}

fn write_timeline(out: &mut String, timeline: &TimelineStats) {
    out.push_str("## 时间线\n\n");
    let cut = if timeline.chunked {
        "按字数等分，非作者章节"
    } else {
        "按作者章节"
    };
    let dated = timeline
        .markers
        .iter()
        .filter(|m| !m.resolved.is_empty())
        .count();
    out.push_str(&format!(
        "时间标记 {} 处（绝对 {} · 月日 {} · 相对 {} · 模糊 {}）· 可排序日期 {} 个 · 曲线段落 {} 段（{}）· 可见正文 {} 字\n\n",
        timeline.markers.len(),
        timeline.absolute_count,
        timeline.partial_count,
        timeline.relative_count,
        timeline.vague_count,
        dated,
        timeline.segment_count,
        cut,
        timeline.char_count
    ));

    if !timeline.readable {
        out.push_str(&format!("{}本次不做时间排序。\n\n", esc(&timeline.note)));
        out.push_str(&timeline_limits(timeline));
        return;
    }

    out.push_str(&format!(
        "### 标记分布（{} 段）\n\n| 段落 | 标记 | |\n| --- | ---: | --- |\n",
        timeline.curve.len()
    ));
    let peak = timeline
        .curve
        .iter()
        .map(|p| p.markers)
        .max()
        .unwrap_or(0)
        .max(1) as f64;
    for p in timeline.curve.iter().take(MAX_ROWS) {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            esc(&p.label),
            p.markers,
            bar(p.markers as f64 / peak)
        ));
    }
    out.push('\n');

    if !timeline.forms.is_empty() {
        let listed: Vec<String> = timeline
            .forms
            .iter()
            .take(8)
            .map(|(form, n)| format!("{} {} 处", timeline_precision_label(form), n))
            .collect();
        out.push_str(&format!("### 写法分布\n\n{}\n\n", listed.join(" · ")));
    }

    out.push_str(&format!("### 日期（{} 个，按出现顺序）\n\n", dated));
    if dated < 2 {
        out.push_str("可排序的日期不足两个，先后无从判断。\n\n");
    } else {
        out.push_str("| 位置 | 段落 | 日期 | 粒度 |\n| ---: | --- | --- | --- |\n");
        for m in timeline
            .markers
            .iter()
            .filter(|m| !m.resolved.is_empty())
            .take(MAX_ROWS)
        {
            out.push_str(&format!(
                "| {:.0}% | {} | {} | {} |\n",
                m.position * 100.0,
                esc(&m.section),
                esc(&m.resolved),
                timeline_precision_label(&m.precision)
            ));
        }
        out.push_str(&omitted(dated, MAX_ROWS));
        out.push('\n');
    }

    if !timeline.backsteps.is_empty() {
        out.push_str(&format!(
            "### 时间倒回（{} 处）\n\n| 前一处 | 日期 | 后一处 | 日期 | 相差 |\n| --- | --- | --- | --- | ---: |\n",
            timeline.backsteps.len()
        ));
        for s in &timeline.backsteps {
            out.push_str(&format!(
                "| {} | {} | {} | {} | 早 {} 天 |\n",
                esc(&s.from_section),
                esc(&s.from_date),
                esc(&s.to_section),
                esc(&s.to_date),
                -s.days
            ));
        }
        out.push('\n');
        out.push_str("日历上倒退不等于写错：倒叙是合法手法，文本分不出这两种。\n\n");
    }

    if !timeline.jumps.is_empty() {
        out.push_str(&format!(
            "### 大跳跃（{} 处）\n\n| 从 | 日期 | 到 | 日期 | 跨度 |\n| --- | --- | --- | --- | ---: |\n",
            timeline.jumps.len()
        ));
        for s in &timeline.jumps {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {:.1} 年 |\n",
                esc(&s.from_section),
                esc(&s.from_date),
                esc(&s.to_section),
                esc(&s.to_date),
                s.days as f64 / 365.0
            ));
        }
        out.push('\n');
    }

    if !timeline.unmarked_stretches.is_empty() {
        out.push_str("### 没有标记的区间\n\n");
        for (from, to, count) in timeline.unmarked_stretches.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- 「{}」到「{}」：{} 段没有一处时间标记\n",
                esc(from),
                esc(to),
                count
            ));
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "### 标记样本（前 {} 处）\n\n| 位置 | 段落 | 类型 | 粒度 | 原文 |\n| ---: | --- | --- | --- | --- |\n",
        timeline.markers.len().min(MAX_ROWS)
    ));
    for m in timeline.markers.iter().take(MAX_ROWS) {
        out.push_str(&format!(
            "| {:.0}% | {} | {} | {} | {} |\n",
            m.position * 100.0,
            esc(&m.section),
            timeline_kind_label(&m.kind),
            timeline_precision_label(&m.precision),
            esc(&m.text)
        ));
    }
    out.push_str(&omitted(timeline.markers.len(), MAX_ROWS));
    out.push('\n');

    let notes: Vec<&TimelineIssue> = timeline
        .issues
        .iter()
        .filter(|i| i.kind != "backstep" && i.kind != "long_jump")
        .collect();
    if !notes.is_empty() {
        out.push_str("### 时间线提示\n\n");
        for i in notes.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "- **{}**（{}）— {}\n",
                timeline_issue_label(&i.kind),
                if i.section.is_empty() {
                    "全文".to_string()
                } else {
                    esc(&i.section)
                },
                esc(&i.detail)
            ));
        }
        out.push('\n');
    }

    out.push_str(&timeline_limits(timeline));
}

fn timeline_kind_label(kind: &str) -> String {
    match kind {
        "absolute" => "绝对日期",
        "partial" => "缺年的日期",
        "relative" => "相对表达",
        "vague" => "模糊表达",
        other => other,
    }
    .to_string()
}

fn timeline_precision_label(precision: &str) -> String {
    match precision {
        "year" => "只到年",
        "month" => "到月",
        "day" => "到日",
        "month_day" => "月日",
        "offset" => "距离",
        "deictic" => "指示",
        "season" => "季节",
        "time_of_day" => "时段",
        "weekday" => "星期",
        "solar_term" => "节气",
        "festival" => "节日",
        "era_year" => "年号·到年",
        "era_day" => "年号·到日",
        "ganzhi_year" => "干支·到年",
        "ganzhi_day" => "干支·到日",
        "lunar_month" => "农历月",
        "lunar_month_day" => "农历月日",
        "lunar_day" => "农历日",
        other => other,
    }
    .to_string()
}

fn timeline_issue_label(kind: &str) -> String {
    match kind {
        "no_markers" => "读不到时间",
        "too_short" => "文本过短",
        "no_order" => "无法排序",
        "backstep" => "时间倒回",
        "long_jump" => "大跳跃",
        "unmarked_stretch" => "长时间无标记",
        "unanchored_ganzhi" => "干支未定年",
        "ambiguous_era" => "年号重名",
        "calendar_note" => "历法换算",
        "term_collision" => "字面识别",
        other => other,
    }
    .to_string()
}

fn timeline_limits(timeline: &TimelineStats) -> String {
    let mut note = String::from(
        "_本维度不给出评分：不写日期的故事不比写满日期的故事差。识别范围是阿拉伯数字与逐字写出的中文年份，加上 天/周/月/年 的相对表达、季节与时段词、节气与节日、星期，以及年号纪年（乾隆三年）、民国纪年、干支纪年（庚子年）与农历月日（腊月初八）。年号表取自公开的历代年号对照（CC BY-SA，据李崇智《中国历代年号考》），农历与公历的换算来自开源历法引擎 lunar-lite（MIT）。三条边界跟着走：年号只到年，因为改元在年中；干支六十年一轮回，只有文档自己给出的年份能唯一框定时才定年，否则只记下不定年；农历日期若该年该月没有那天就拒绝，不做修补。外文日期仍不在范围内。相对与模糊表达锚在叙述当下而不是日历，只统计不参与排序；只到年份的日期粒度太粗，同样不参与排序。月按 30 天、年按 365 天估算。_\n\n",
    );
    if timeline.entity_date_count > 0 {
        note.push_str(&format!(
            "导入的实体标注里有 {} 处日期，本维度的文本扫描没有读取它们；两处数字相差多少，说明有多少日期是用本文不认的写法写的。\n\n",
            timeline.entity_date_count
        ));
    }
    if timeline.chunked {
        note.push_str(
            "本文的章节少于三段，标记分布落在按字数等分的块上，所以位置是算术结果，不是作者自己的分段。\n\n",
        );
    }
    note
}

/// What the shape match cannot claim. Always printed, because the limits are the
/// difference between a description and a grade.
fn structure_limits(structure: &StructureStats) -> String {
    let mut note = String::from(
        "_本维度不给出评分：契合三幕不是优点，有意拒绝这个形状也不是缺点。地标来自线程变化与张力曲线，两者都是代理信号；「未见」只说明这些信号没有显示这一环，不等于故事缺少它。节拍区间（如 25%/50%/25%）是写作手册里的约定，不是测量。_\n\n",
    );
    if structure.chunked {
        note.push_str(
            "本文的章节少于三段，曲线落在按字数等分的块上，所以位置是算术结果，不是作者自己的分段。\n\n",
        );
    }
    note
}
/// 作者声音 — how the text is written, whatever it is about.
///
/// The section prints no score: 长句不比短句好. Everything here is a rate over the
/// same visible-character count, so two projects can be read side by side.
fn write_voice(out: &mut String, voice: &VoiceStats) {
    out.push_str("## 作者声音\n\n");
    if voice.char_count == 0 {
        out.push_str("没有可统计的文本。\n\n");
        return;
    }

    out.push_str("| 指标 | 数值 |\n| --- | ---: |\n");
    let rows: [(&str, String); 9] = [
        ("字数", voice.char_count.to_string()),
        ("句子数", voice.sentence_count.to_string()),
        ("平均句长（字）", format!("{:.1}", voice.avg_sentence_chars)),
        ("句长标准差", format!("{:.1}", voice.sentence_stdev)),
        ("句长变异系数", format!("{:.2}", voice.sentence_cv)),
        (
            "最短 / 最长句（字）",
            format!("{} / {}", voice.shortest_sentence, voice.longest_sentence),
        ),
        ("每句小句数", format!("{:.2}", voice.clauses_per_sentence)),
        (
            "段落数 / 平均段落长（字）",
            format!(
                "{} / {:.1}",
                voice.paragraph_count, voice.avg_paragraph_chars
            ),
        ),
        (
            "只有一句话的段落",
            format!(
                "{} / {}",
                voice.single_sentence_paragraphs, voice.paragraph_count
            ),
        ),
    ];
    for (label, value) in rows {
        out.push_str(&format!("| {} | {} |\n", label, value));
    }
    out.push('\n');

    out.push_str("标点密度（每千字）：\n\n");
    let present: Vec<&VoiceMark> = voice.marks.iter().filter(|m| m.count > 0).collect();
    if present.is_empty() {
        out.push_str("这段文本没有用到清单里的标点。\n\n");
    } else {
        out.push_str("| 标点 | 次数 | 每千字 |\n| --- | ---: | ---: |\n");
        for m in present {
            out.push_str(&format!(
                "| {}（{}） | {} | {:.2} |\n",
                esc(&m.label),
                esc(&m.mark),
                m.count,
                m.per_1000
            ));
        }
        out.push('\n');
    }

    out.push_str("功能词密度（每千字，按单字计数）：\n\n");
    out.push_str("| 类别 | 字 | 次数 | 每千字 |\n| --- | --- | ---: | ---: |\n");
    for g in &voice.function_words {
        out.push_str(&format!(
            "| {} | {} | {} | {:.2} |\n",
            esc(&g.group),
            esc(&g.words),
            g.count,
            g.per_1000
        ));
    }
    out.push('\n');

    let verdict = match voice.stance.verdict.as_str() {
        "hedging" => "偏犹疑",
        "assertive" => "偏断言",
        _ => "均衡",
    };
    out.push_str("人称与立场：\n\n");
    out.push_str("| 指标 | 数值 |\n| --- | ---: |\n");
    out.push_str(&format!(
        "| 第一人称 | {}（{:.0}%） |\n",
        voice.pronouns.first,
        voice.pronouns.first_share * 100.0
    ));
    out.push_str(&format!(
        "| 第二人称 | {}（{:.0}%） |\n",
        voice.pronouns.second,
        voice.pronouns.second_share * 100.0
    ));
    out.push_str(&format!(
        "| 第三人称 | {}（{:.0}%） |\n",
        voice.pronouns.third,
        voice.pronouns.third_share * 100.0
    ));
    out.push_str(&format!(
        "| 犹疑用词 / 断言用词 | {} / {}（每万字 {:.1} / {:.1}） |\n",
        voice.stance.hedging,
        voice.stance.assertive,
        voice.stance.hedging_per_10k,
        voice.stance.assertive_per_10k
    ));
    out.push_str(&format!("| 立场 | {} |\n\n", verdict));

    out.push_str("句长曲线：\n\n");
    if voice.points.is_empty() {
        out.push_str("没有可切分的段落。\n\n");
    } else {
        out.push_str("| 段落 | 字数 | 句子数 | 平均句长 | 每句小句数 |\n");
        out.push_str("| --- | ---: | ---: | ---: | ---: |\n");
        for p in voice.points.iter().take(MAX_ROWS) {
            out.push_str(&format!(
                "| {} | {} | {} | {:.1} | {:.2} |\n",
                esc(&p.section),
                p.char_count,
                p.sentences,
                p.avg_sentence_chars,
                p.clauses_per_sentence
            ));
        }
        if voice.points.len() > MAX_ROWS {
            out.push_str(&format!(
                "\n另有 {} 段未列出。\n\n",
                voice.points.len() - MAX_ROWS
            ));
        }
        if voice.chunked {
            out.push_str(
                "_本文的章节少于三段，段落是按字数等分的块，是算术结果而非作者自己的分段。_\n\n",
            );
        }
    }

    if !voice.issues.is_empty() {
        out.push_str("读得到的习惯：\n\n");
        for issue in &voice.issues {
            out.push_str(&format!("- {}\n", esc(&issue.detail)));
        }
        out.push('\n');
    }

    out.push_str(&voice_limits(voice));
}

/// What the numbers were measured with, and where they stop.
fn voice_limits(voice: &VoiceStats) -> String {
    let mut note = String::from(
        "_本维度不给出评分：长句不比短句好，留余地的叙述不比下断言的叙述差。功能词与人称按单字计数，\
         所以「了解」记一个「了」、「其他」记一个「他」（人称统计已跳过「其他/别的」里的这一字）；\
         所有密度都按可见字符每千字计算，一段被引用的长歌词会把标点密度抬高。\
         犹疑与断言的词表是固定的，命中只说明这些词出现过，不说明作者是否真的犹豫。_\n\n",
    );
    if voice.char_count < 600 {
        note.push_str("全文不足 600 字，下面的数字只是这一次统计的结果，还不足以说明写法。\n\n");
    }
    note
}

fn write_footer(out: &mut String) {
    out.push_str("---\n\n");
    out.push_str("_本报告由 NarrativeStudio 基于导入的分析结果自动生成。_\n");
}

/// A (name, count) list rendered as a table.
fn write_pair_section(out: &mut String, title: &str, items: &[(String, u32)], limit: usize) {
    out.push_str(&format!("### {}（{} 项）\n\n", title, items.len()));
    if items.is_empty() {
        out.push_str("无数据。\n\n");
        return;
    }
    out.push_str("| 名称 | 次数 |\n| --- | ---: |\n");
    for (name, count) in items.iter().take(limit) {
        out.push_str(&format!("| {} | {} |\n", esc(name), count));
    }
    out.push_str(&omitted(items.len(), limit));
    out.push('\n');
}

/// A `|` would end a table cell and a newline would end the row; analysed text can
/// contain either, so both are neutralised.
fn esc(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace("\r\n", " ")
        .replace('\n', " ")
}

fn omitted(total: usize, shown: usize) -> String {
    if total > shown {
        // The trailing newline keeps a blank line between the note and whatever
        // heading follows, which several Markdown parsers require.
        format!("\n_（另有 {} 项未列出）_\n", total - shown)
    } else {
        String::new()
    }
}

/// Ten-cell intensity bar: the arc stays legible in plain text, not only in the UI
/// chart.
fn bar(value: f64) -> String {
    let filled = (value.clamp(0.0, 1.0) * 10.0).round() as usize;
    format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled))
}

fn severity_label(severity: &str) -> &'static str {
    match severity {
        "high" => "高",
        "medium" => "中",
        "low" => "低",
        _ => "其他",
    }
}

fn arc_shape_label(shape: &str) -> &'static str {
    match shape {
        "mountain" => "山形",
        "rising" => "上升",
        "falling" => "下降",
        "steady" => "平稳",
        _ => "未知",
    }
}

fn plot_shape_label(shape: &str) -> &'static str {
    match shape {
        "sustained" => "贯穿",
        "intermittent" => "断续",
        "burst" => "集中",
        "setup_only" => "仅铺垫",
        "late_arrival" => "迟到",
        "minor" => "次要",
        _ => "未知",
    }
}

fn pacing_verdict_label(verdict: &str) -> &'static str {
    match verdict {
        "flat" => "单调",
        "varied" => "起伏",
        "spiky" => "跳跃",
        _ => "未知",
    }
}

/// Conflict families. `""` is the no-conflict case, which the curve uses for
/// segments with no vocabulary at all.
fn conflict_type_label(kind: &str) -> &'static str {
    match kind {
        "interpersonal" => "人际",
        "internal" => "内心",
        "societal" => "社会",
        "survival" => "生存",
        "" => "无",
        _ => "未知",
    }
}

/// How much conflict vocabulary the document carries. `none` deliberately does not
/// say "no conflict": it says no conflict *vocabulary*.
fn conflict_verdict_label(verdict: &str) -> &'static str {
    match verdict {
        "none" => "未见用词",
        "thin" => "偏少",
        "present" => "适中",
        "heavy" => "密集",
        _ => "未知",
    }
}

/// Which vocabulary the theme pass read. Naming the source matters: a list built
/// from noun signals and one rebuilt from POS tags are not equally trustworthy.
fn theme_source_label(source: &str) -> &'static str {
    match source {
        "noun_signals" => "名词信号",
        "noun_tokens" => "名词词性",
        "none" => "无",
        _ => "未知",
    }
}

/// How far a theme reaches across the document.
fn theme_reach_label(reach: &str) -> &'static str {
    match reach {
        "sustained" => "贯穿",
        "recurring" => "复现",
        "episodic" => "偶发",
        "occasional" => "零星",
        _ => "未知",
    }
}

/// Where a theme's mentions sit.
fn theme_concentration_label(concentration: &str) -> &'static str {
    match concentration {
        "spread" => "均匀",
        "front" => "集中在开头",
        "middle" => "集中在中段",
        "back" => "集中在结尾",
        _ => "未知",
    }
}

/// How much environment vocabulary the document carries. `none` says no setting
/// *vocabulary*, not no setting.
fn setting_verdict_label(verdict: &str) -> &'static str {
    match verdict {
        "none" => "未见用词",
        "thin" => "偏少",
        "present" => "适中",
        "heavy" => "密集",
        _ => "未知",
    }
}

/// What kind of place the importer found.
fn place_kind_label(kind: &str) -> &'static str {
    match kind {
        "LOCATION" => "地点",
        "FACILITY" => "场所",
        _ => "未知",
    }
}

/// What the setting did for the reader, or failed to do.
fn setting_issue_label(kind: &str) -> &'static str {
    match kind {
        "unanchored" => "无地点锚点",
        "crowded" => "地点过密",
        "long_absence" => "长时间离开地点",
        "thin_environment" => "环境描写偏少",
        _ => "未知",
    }
}

/// How much stock phrasing the document carries. `none` says the library found
/// nothing, not that the prose is fresh.
fn cliche_verdict_label(verdict: &str) -> &'static str {
    match verdict {
        "none" => "未见用词",
        "light" => "偏少",
        "moderate" => "中等",
        "heavy" => "密集",
        _ => "未知",
    }
}

/// Which kind of stock phrase was found.
fn cliche_category_label(category: &str) -> &'static str {
    match category {
        "expression" => "表情动作",
        "body" => "身体反应",
        "emotion" => "情绪直陈",
        "simile" => "套路比喻",
        "atmosphere" => "场面套话",
        "speech_tag" => "说话套话",
        "transition" => "时间过渡",
        "look" => "外貌套话",
        _ => "未知",
    }
}

/// What the phrase use does to the prose.
fn cliche_issue_label(kind: &str) -> &'static str {
    match kind {
        "repeated" => "反复使用",
        "dense_segment" => "局部密集",
        "one_note" => "依赖同一句",
        "speech_tags" => "对白标签过多",
        _ => "未知",
    }
}

/// How much of the text is spoken. `none` says no speech was *detected*: a text whose
/// source carries no quotation marks reads the same way as one where nobody speaks.
fn dialogue_verdict_label(verdict: &str) -> &'static str {
    match verdict {
        "none" => "未见对白",
        "sparse" => "对白偏少",
        "balanced" => "对白与叙述混合",
        "heavy" => "以对白为主",
        _ => "未知",
    }
}

/// How one piece of speech was recognised.
fn dialogue_kind_label(kind: &str) -> &'static str {
    match kind {
        "quote" => "引号",
        "dash" => "破折号",
        _ => "未知",
    }
}

/// What the mix does to the reading.
fn dialogue_issue_label(kind: &str) -> &'static str {
    match kind {
        "no_dialogue" => "没有对白",
        "silent_segment" => "长时间无对白",
        "flooded_segment" => "局部对白过密",
        "monologue" => "长独白",
        "telegraphic" => "对白过短",
        "unquoted" => "对白未加引号",
        "uneven" => "各段比例不均",
        _ => "未知",
    }
}

/// How strong the words found are. `none` says the library found nothing, not that the
/// content is gentle — a text can be brutal in words the library does not carry.
fn explicit_advisory_label(advisory: &str) -> &'static str {
    match advisory {
        "none" => "词库未见",
        "mild" => "轻微",
        "moderate" => "中等",
        "strong" => "强烈",
        _ => "未知",
    }
}

/// Which content family a hit belongs to.
fn explicit_category_label(category: &str) -> &'static str {
    match category {
        "profanity" => "粗口辱骂",
        "sexual" => "性描写",
        "sexual_violence" => "性暴力",
        "violence" => "暴力血腥",
        "self_harm" => "自伤",
        "hate" => "贬损歧视",
        "substance" => "毒品",
        _ => "未知",
    }
}

/// How strong one term's wording is.
fn explicit_tier_label(tier: &str) -> &'static str {
    match tier {
        "mild" => "轻",
        "moderate" => "中",
        "strong" => "重",
        _ => "未知",
    }
}

/// Why a family is being surfaced.
fn explicit_issue_label(kind: &str) -> &'static str {
    match kind {
        "self_harm" => "自伤描写",
        "sexual_violence" => "性暴力",
        "sexual_content" => "露骨性描写",
        "gore" => "血腥细节",
        "hate_slur" => "贬损歧视用语",
        "substance" => "毒品描写",
        "dense_segment" => "局部密集",
        _ => "未知",
    }
}

/// The classic shapes tried against the document.
fn structure_model_label(model: &str) -> &'static str {
    match model {
        "three_act" => "三幕结构",
        "kisho_tenketsu" => "起承转合",
        "hero_journey" => "英雄之旅",
        _ => "未知",
    }
}

/// One beat of a model.
fn structure_beat_label(beat: &str) -> &'static str {
    match beat {
        // 三幕结构
        "setup" => "铺垫",
        "first_turn" => "第一转折点",
        "midpoint" => "中点",
        "second_turn" => "第二转折点",
        "climax" => "高潮",
        "resolution" => "收束",
        // 起承转合
        "kisho" => "起",
        "sho" => "承",
        "ten" => "转",
        "ketsu" => "合",
        // 英雄之旅
        "ordinary_world" => "日常世界",
        "call" => "冒险的召唤",
        "threshold" => "跨越门槛",
        "tests" => "试炼与伙伴",
        "inmost_cave" => "最深洞穴",
        "resurrection" => "复活",
        "return" => "携宝归来",
        _ => "未知",
    }
}

/// Where a beat was answered. `missing` says the signals showed nothing there, which
/// is not the same as the story lacking the beat — the closing note says so.
fn structure_status_label(status: &str) -> &'static str {
    match status {
        "found" => "命中",
        "displaced" => "错位",
        "missing" => "未见",
        _ => "未知",
    }
}

/// How many beats landed where the model expects them.
fn structure_fit_label(fit: &str) -> &'static str {
    match fit {
        "matched" => "吻合",
        "partial" => "部分吻合",
        "unclear" => "信号不足",
        _ => "未知",
    }
}

/// What kind of signal made a landmark.
fn structure_landmark_label(kind: &str) -> &'static str {
    match kind {
        "opening" => "开篇",
        "turn" => "线程转折",
        "peak" => "张力高点",
        "trough" => "张力低点",
        "climax" => "张力最高点",
        _ => "未知",
    }
}

/// Which part of the document's own three-way split.
fn structure_act_label(name: &str) -> &'static str {
    match name {
        "act_one" => "第一幕",
        "act_two" => "第二幕",
        "act_three" => "第三幕",
        _ => "未知",
    }
}

/// What the shape does that a writer may want to look at again.
fn structure_issue_label(kind: &str) -> &'static str {
    match kind {
        "no_early_turn" => "开头无变化",
        "missing_midpoint" => "中段空白",
        "climax_early" => "张力最高点偏早",
        "no_descent" => "结尾未回落",
        "act_imbalance" => "幕长失衡",
        "too_few_landmarks" => "地标太少",
        _ => "未知",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_analysis::{
        ArcPoint, ClicheHit, ClicheIssue, ClichePhrase, ClichePoint, ConflictAbsence,
        ConflictPoint, ConflictType, DialogueIssue, DialoguePoint, DialogueSample, DimensionScore,
        DragSection, ExplicitHit, ExplicitIssue, ExplicitTerm, FlatStretch, NarrativeArc,
        PacingPoint, Place, Plotline, PlotlineStage, Readability, Recommendation, RepeatedPhrase,
        Scene, SectionInfo, SettingIssue, StructureAct, StructureBeat, StructureIssue,
        StructureLandmark, StructureModel, Theme, TimelineMarker, TimelinePoint, TimelineStep,
        TurningPoint, VoiceIssue, VoicePoint, VoicePronouns, VoiceStance, VoiceWordGroup,
    };

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-01-01T08:30:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn empty_t0() -> T0Stats {
        T0Stats {
            word_count: 0,
            char_count: 0,
            block_count: 0,
            top_words: vec![],
            pos_distribution: vec![],
            entity_counts: vec![],
            top_entities: vec![],
            top_noun_signals: vec![],
            noun_signal_count: 0,
            avg_block_length: 0.0,
            avg_sentence_length: 0.0,
            sentence_count: 0,
            avg_sentence_chars: 0.0,
            sentence_length_distribution: vec![],
            adverb_count: 0,
            adjective_count: 0,
            top_adverbs: vec![],
            top_adjectives: vec![],
            top_repeated_phrases: vec![],
            readability: Readability {
                score: 0.0,
                level: "适中".into(),
                method: "cjk-sentence-length".into(),
            },
        }
    }

    fn empty_t1() -> T1Stats {
        T1Stats {
            sections: vec![],
            section_count: 0,
            title_blocks: 0,
            paragraph_blocks: 0,
            list_blocks: 0,
            block_type_distribution: vec![],
            longest_section: None,
            shortest_section: None,
            characters: vec![],
            arc: NarrativeArc {
                points: vec![],
                shape: "steady".into(),
            },
            entity_cooccurrences: vec![],
        }
    }

    fn empty_plot() -> PlotStats {
        PlotStats {
            plotlines: vec![],
            thread_count: 0,
            main_plotlines: vec![],
            sub_plotlines: vec![],
            turning_points: vec![],
            unresolved_threads: vec![],
            late_arrivals: vec![],
            coherence_score: 100.0,
            section_count: 0,
        }
    }

    /// What `run_pacing_analysis` returns for an empty document: nothing judged.
    fn empty_pacing() -> PacingStats {
        PacingStats {
            points: vec![],
            segment_count: 0,
            chunked: false,
            verdict: "flat".into(),
            tension_stdev: 0.0,
            sentence_stdev: 0.0,
            min_tension: 0.0,
            max_tension: 0.0,
            mean_section_chars: 0.0,
            longest_flat_run: 0,
            flat_stretches: vec![],
            drag_sections: vec![],
            pace_score: 100.0,
        }
    }

    /// What `run_conflict_analysis` returns for an empty document: no vocabulary,
    /// and therefore no score.
    fn empty_conflict() -> ConflictStats {
        ConflictStats {
            points: vec![],
            segment_count: 0,
            chunked: false,
            total_hits: 0,
            density: 0.0,
            types: vec![],
            dominant_type: String::new(),
            verdict: "none".into(),
            peak_section: String::new(),
            peak_intensity: 0.0,
            peak_position: 0.0,
            tail_intensity: 0.0,
            concentration: 0.0,
            absent_runs: vec![],
            longest_absent_run: 0,
            conflict_score: None,
        }
    }

    /// What `run_theme_analysis` returns for an empty document: nothing usable, and
    /// therefore no score.
    fn empty_theme() -> ThemeStats {
        ThemeStats {
            themes: vec![],
            candidate_count: 0,
            total_mentions: 0,
            source: "none".into(),
            top_share: 0.0,
            top3_share: 0.0,
            singleton_share: 0.0,
            dominant_terms: vec![],
            section_count: 0,
            theme_score: None,
        }
    }

    /// What `run_setting_analysis` returns for an empty document: no scene, no
    /// place, no description, and therefore no score.
    fn empty_setting() -> SettingStats {
        SettingStats {
            points: vec![],
            segment_count: 0,
            chunked: false,
            scenes: vec![],
            scene_count: 0,
            unanchored_scenes: 0,
            unanchored_share: 0.0,
            places: vec![],
            place_count: 0,
            recurring_places: 0,
            total_env_hits: 0,
            env_density: 0.0,
            top_env_terms: vec![],
            verdict: "none".into(),
            dominant_place: String::new(),
            peak_section: String::new(),
            peak_intensity: 0.0,
            issues: vec![],
            longest_absence: 0,
            setting_score: None,
        }
    }

    /// What `run_cliche_analysis` returns for an empty document: no text long enough
    /// to carry a rate, and therefore no score.
    fn empty_cliche() -> ClicheStats {
        ClicheStats {
            points: vec![],
            segment_count: 0,
            chunked: false,
            total_hits: 0,
            distinct_phrases: 0,
            char_count: 0,
            per_10k: 0.0,
            verdict: "none".into(),
            categories: vec![],
            phrases: vec![],
            hits: vec![],
            dialogue_hits: 0,
            repeated_phrases: 0,
            repeat_hits: 0,
            top_share: 0.0,
            top_phrase: String::new(),
            peak_section: String::new(),
            peak_intensity: 0.0,
            issues: vec![],
            lexicon_size: 150,
            cliche_score: None,
        }
    }

    /// No speech detected: the section still reports the split (all of it narrative) and
    /// leaves out everything that only exists once something is spoken.
    fn empty_dialogue() -> DialogueStats {
        DialogueStats {
            points: vec![],
            segment_count: 0,
            chunked: false,
            char_count: 0,
            dialogue_chars: 0,
            narrative_chars: 0,
            dialogue_share: 0.0,
            narrative_share: 1.0,
            unit_count: 0,
            quoted_units: 0,
            dash_units: 0,
            avg_unit_chars: 0.0,
            longest_unit: 0,
            unit_length_distribution: vec![],
            blocks: 0,
            blocks_with_dialogue: 0,
            longest_dialogue_run: 0,
            longest_quiet_run: 0,
            longest_quiet_chars: 0,
            silent_segments: 0,
            flooded_segments: 0,
            verdict: "none".into(),
            spread: 0.0,
            peak_section: String::new(),
            peak_share: 0.0,
            quietest_section: String::new(),
            quietest_share: 0.0,
            samples: vec![],
            issues: vec![],
        }
    }

    fn empty_explicit() -> ExplicitStats {
        ExplicitStats {
            points: vec![],
            segment_count: 0,
            chunked: false,
            char_count: 0,
            total_hits: 0,
            distinct_terms: 0,
            per_10k: 0.0,
            advisory: "none".into(),
            tiers: vec![
                ("mild".into(), 0),
                ("moderate".into(), 0),
                ("strong".into(), 0),
            ],
            categories: vec![],
            advisory_categories: vec![],
            terms: vec![],
            hits: vec![],
            dialogue_hits: 0,
            top_share: 0.0,
            top_term: String::new(),
            peak_section: String::new(),
            peak_intensity: 0.0,
            issues: vec![],
            lexicon_size: 35,
            lexicon_variants: 190,
        }
    }

    /// What `run_structure_analysis` returns for an empty document: nothing read, and
    /// the reason why stated. There is no score to be absent.
    fn empty_structure() -> StructureStats {
        StructureStats {
            landmarks: vec![],
            acts: vec![],
            models: vec![],
            issues: vec![],
            segment_count: 0,
            chunked: false,
            char_count: 0,
            turn_count: 0,
            readable: false,
            note: "文档没有可切分的正文。".into(),
        }
    }

    fn empty_timeline() -> TimelineStats {
        TimelineStats {
            markers: vec![],
            curve: vec![],
            absolute_count: 0,
            partial_count: 0,
            relative_count: 0,
            vague_count: 0,
            forms: vec![],
            entity_date_count: 0,
            first_date: String::new(),
            last_date: String::new(),
            span_days: 0,
            backsteps: vec![],
            jumps: vec![],
            unmarked_stretches: vec![],
            issues: vec![],
            segment_count: 0,
            chunked: false,
            char_count: 0,
            readable: false,
            note: "全文没有读到任何时间表达。".into(),
        }
    }

    fn clean_assessment() -> Assessment {
        Assessment {
            overall: 88.0,
            dimensions: vec![DimensionScore {
                key: "readability".into(),
                label: "可读性".into(),
                score: 88.0,
                detail: "等级「适中」（cjk-sentence-length）".into(),
            }],
            recommendations: vec![],
        }
    }

    #[test]
    fn test_render_is_deterministic() {
        let (t0, t1, plot, pacing, conflict, theme, setting, cliche, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
            clean_assessment(),
        );
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "测试项目",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        assert_eq!(render_markdown(&input), render_markdown(&input));
    }

    #[test]
    fn test_golden_minimal_report() {
        let (t0, t1, plot, pacing, conflict, theme, setting, cliche, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
            clean_assessment(),
        );
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "测试项目",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let expected = "\
# 测试项目 · 叙事分析报告

- 生成时间：2026-01-01 08:30 UTC
- 分析版本：m2-v6

## 总体评估

总体评分：**88.0 / 100**

| 维度 | 评分 | 说明 |
| --- | ---: | --- |
| 可读性 | 88.0 | 等级「适中」（cjk-sentence-length） |

## 关键建议

本次分析未发现需要处理的问题。

## 文本统计

| 指标 | 数值 |
| --- | ---: |
| 字数 | 0 |
| 词数 | 0 |
| 块数 | 0 |
| 句子数 | 0 |
| 平均句长（字） | 0.0 |
| 平均块长（字） | 0.0 |
| 可读性 | 0.0（适中） |
| 副词 / 形容词 | 0 / 0 |

## 文本诊断

### 高频词（0 项）

无数据。

### 高频实体（0 项）

无数据。

### 高频名词信号（0 项）

无数据。

### 副词（0 项）

无数据。

### 形容词（0 项）

无数据。

### 重复短语（0 项）

未发现高频重复短语。\n\n## 对话与叙事

对白占比：**0.0%** · 叙述 100.0% · 对白 0 处（引号 0 · 破折号 0）· 判定：**未见对白**（`none`）

全文 0 字：对白 0 字 · 叙述 0 字 · 平均每处 0.0 字 · 最长一处 0 字 · 含对白的文本块 0 / 0

各段均不足 300 字，不比较段落间的对白差异。

### 对白占比曲线（0 段）

无数据。

_对白按引号与破折号识别：原文不用引号时会漏判，引号本身计入叙述一侧。比例只说明对白与叙述的多少，不说明好坏。_

## 章节结构

共 0 个章节；最长：—；最短：—

### 块类型分布（0 项）

无数据。

### 章节一览

未识别到章节结构。

## 叙事分析

叙事弧线形态：**平稳**（`steady`）

### 实体共现（0 项）

无数据。

### 人物（0 位）

未识别到人物。

## 情节线

情节连贯性：**100.0 / 100**；共 0 条线索（0 个章节）

未识别到情节线：没有实体或名词信号达到 3 次的提及门槛。

## 节奏

节奏评分：**100.0 / 100**；判定：**单调**（`flat`）· 0 段 · 张力标准差 0.0

未识别到节奏曲线：文档没有可切分的段落。

## 冲突

未检测到冲突用词，因此**不给出评分**：文本可能不是叙事文本，也可能是词库尚未覆盖。

### 冲突类型（0 类）

无数据。

未识别到冲突曲线：文档没有可切分的段落。

## 主题

没有可用于主题分析的词汇，或没有任何词反复出现，因此**不给出评分**。

未识别到主题：没有词达到 3 次提及且跨越 2 段以上。

## 场景/设定

没有点名地点，也没有环境用词，因此**不给出评分**。

未识别到场景：文档没有可切分的段落。

## 陈词滥调

全文 0 字，不足 1000 字，按万字密度评分没有意义，因此**不给出评分**。

最常用：—（占命中 0%）· 峰值段落：—（0）· 反复使用的短语 0 个

### 高频套话（0 个）

没有命中词库里的表达。

_命中只说明字面撞上词库里的说法，不等于写得差；是否真的是陈词滥调、要不要改，由人来判断。_

## 显式语言

内容分级：**词库未见**（`none`）· 命中 0 处（0.0 处/万字）· 涉及 0 个词条 · 对白内 0 处

按强度：轻 0 处 · 中 0 处 · 重 0 处

最常用：—（占命中 0%）· 峰值段落：—（0）

### 命中词条（0 个）

没有命中词库里的用词。

_本维度不给出评分：一部战争题材并不比田园题材更差，露骨与否是内容属性而不是质量。命中只说明词面撞上词库，不等于剧情如此；分级只反映用词强度，不反映事件本身，是否需要读者提示由人决定。_

## 故事结构

地标 0 个 · 转折点 0 个 · 曲线段落 0 段（按作者章节）· 可见正文 0 字

文档没有可切分的正文。本次不做模型对照。

_本维度不给出评分：契合三幕不是优点，有意拒绝这个形状也不是缺点。地标来自线程变化与张力曲线，两者都是代理信号；「未见」只说明这些信号没有显示这一环，不等于故事缺少它。节拍区间（如 25%/50%/25%）是写作手册里的约定，不是测量。_

## 时间线

时间标记 0 处（绝对 0 · 月日 0 · 相对 0 · 模糊 0）· 可排序日期 0 个 · 曲线段落 0 段（按作者章节）· 可见正文 0 字

全文没有读到任何时间表达。本次不做时间排序。

_本维度不给出评分：不写日期的故事不比写满日期的故事差。识别范围是阿拉伯数字与逐字写出的中文年份，加上 天/周/月/年 的相对表达、季节与时段词、节气与节日、星期，以及年号纪年（乾隆三年）、民国纪年、干支纪年（庚子年）与农历月日（腊月初八）。年号表取自公开的历代年号对照（CC BY-SA，据李崇智《中国历代年号考》），农历与公历的换算来自开源历法引擎 lunar-lite（MIT）。三条边界跟着走：年号只到年，因为改元在年中；干支六十年一轮回，只有文档自己给出的年份能唯一框定时才定年，否则只记下不定年；农历日期若该年该月没有那天就拒绝，不做修补。外文日期仍不在范围内。相对与模糊表达锚在叙述当下而不是日历，只统计不参与排序；只到年份的日期粒度太粗，同样不参与排序。月按 30 天、年按 365 天估算。_

## 作者声音

没有可统计的文本。

---

_本报告由 NarrativeStudio 基于导入的分析结果自动生成。_
";
        assert_eq!(render_markdown(&input), expected);
    }

    #[test]
    fn test_all_dimensions_and_headings_rendered() {
        let mut a = clean_assessment();
        for i in 0..4 {
            a.dimensions.push(DimensionScore {
                key: format!("d{i}"),
                label: format!("维度{i}"),
                score: 50.0,
                detail: "—".into(),
            });
        }
        let (t0, t1, plot, pacing, conflict, theme, setting, cliche) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
        );
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);
        for heading in [
            "# P · 叙事分析报告",
            "## 总体评估",
            "## 关键建议",
            "## 文本统计",
            "## 文本诊断",
            "## 对话与叙事",
            "## 章节结构",
            "## 叙事分析",
            "## 情节线",
            "## 节奏",
            "## 冲突",
            "## 主题",
            "## 场景/设定",
            "## 陈词滥调",
            "## 显式语言",
            "## 故事结构",
            "## 时间线",
            "## 作者声音",
        ] {
            assert!(md.contains(heading), "missing heading: {heading}");
        }
        for i in 0..4 {
            assert!(
                md.contains(&format!("| 维度{i} | 50.0 |")),
                "missing dimension {i}"
            );
        }
        assert_eq!(
            md.matches("\n## ").count(),
            17,
            "exactly seventeen H2 sections"
        );
    }

    #[test]
    fn test_recommendations_grouped_and_counted() {
        let (t0, t1, plot, pacing, conflict, theme, setting, cliche) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
        );
        let mut a = clean_assessment();
        a.recommendations = vec![
            Recommendation {
                severity: "high".into(),
                dimension: "readability".into(),
                title: "可读性偏低".into(),
                detail: "拆分长句".into(),
            },
            Recommendation {
                severity: "medium".into(),
                dimension: "repetition".into(),
                title: "重复短语".into(),
                detail: "「出版单位」出现 12 次".into(),
            },
        ];
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);
        assert!(md.contains("共 2 项（高 1 · 中 1 · 低 0）"));
        assert!(md.contains("- **高 · 可读性偏低**（readability）— 拆分长句"));
        assert!(md.contains("- **中 · 重复短语**（repetition）"));
    }

    #[test]
    fn test_table_cells_are_escaped() {
        let (mut t0, mut t1, plot, pacing, conflict, theme, setting, cliche, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
            clean_assessment(),
        );
        t0.top_entities = vec![("A|B".into(), 3)];
        t1.sections = vec![SectionInfo {
            path: "第一章\n第二节".into(),
            block_count: 2,
            char_count: 100,
        }];
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);
        assert!(md.contains("| A\\|B | 3 |"), "pipe must be escaped");
        assert!(
            md.contains("| 第一章 第二节 | 2 | 100 |"),
            "newline must be flattened"
        );
        // An analysed `|` must not create an extra column.
        let rows: Vec<&str> = md.lines().filter(|l| l.starts_with("| 第一章")).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].matches('|').count(), 4, "row must stay three cells");
    }

    #[test]
    fn test_long_lists_are_capped_and_announced() {
        let (mut t0, mut t1, plot, pacing, conflict, theme, setting, cliche, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
            clean_assessment(),
        );
        t0.top_words = (0..25).map(|i| (format!("w{i}"), i)).collect();
        t0.top_repeated_phrases = (0..25)
            .map(|i| RepeatedPhrase {
                text: format!("p{i}"),
                count: i,
            })
            .collect();
        t1.sections = (0..25)
            .map(|i| SectionInfo {
                path: format!("s{i}"),
                block_count: 1,
                char_count: 10,
            })
            .collect();
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);
        // Word lists cap at 15, phrase and section lists at 20 — each cap must say
        // how much it left out.
        assert_eq!(md.matches("\n_（另有 10 项未列出）_").count(), 1);
        assert_eq!(md.matches("\n_（另有 5 项未列出）_").count(), 2);
        assert!(md.contains("| w14 | 14 |") && !md.contains("| w15 | 15 |"));
        assert!(md.contains("| s19 | 1 | 10 |") && !md.contains("| s20 | 1 | 10 |"));
    }

    #[test]
    fn test_arc_renders_bar_and_shape() {
        let (t0, mut t1, plot, pacing, conflict, theme, setting, cliche, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
            clean_assessment(),
        );
        t1.arc.shape = "mountain".into();
        t1.arc.points = vec![
            ArcPoint {
                section: "Ch1".into(),
                intensity: 0.0,
                char_count: 100,
            },
            ArcPoint {
                section: "Ch2".into(),
                intensity: 1.0,
                char_count: 200,
            },
            ArcPoint {
                section: "Ch3".into(),
                intensity: 0.5,
                char_count: 150,
            },
        ];
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);
        assert!(md.contains("叙事弧线形态：**山形**（`mountain`）"));
        assert!(md.contains("| Ch1 | ░░░░░░░░░░ 0.00 | 100 |"));
        assert!(md.contains("| Ch2 | ██████████ 1.00 | 200 |"));
        assert!(md.contains("| Ch3 | █████░░░░░ 0.50 | 150 |"));
    }

    #[test]
    fn test_plot_section_renders_threads_turns_and_dangling_threads() {
        let (t0, t1, a) = (empty_t0(), empty_t1(), clean_assessment());
        let mut plot = empty_plot();
        let pacing = empty_pacing();
        let conflict = empty_conflict();
        let theme = empty_theme();
        let setting = empty_setting();
        let cliche = empty_cliche();
        plot.coherence_score = 84.0;
        plot.thread_count = 3;
        plot.section_count = 5;
        plot.plotlines = vec![
            Plotline {
                subject: "Marlowe".into(),
                kind: "PERSON".into(),
                mentions: 12,
                sections_touched: 5,
                coverage: 1.0,
                first_section: "第一章".into(),
                last_section: "第五章".into(),
                stages: vec![PlotlineStage {
                    section: "第一章".into(),
                    mentions: 4,
                    gap_before: 0,
                }],
                longest_gap: 0,
                shape: "sustained".into(),
                is_main: true,
            },
            Plotline {
                subject: "Chase".into(),
                kind: "PERSON".into(),
                mentions: 3,
                sections_touched: 1,
                coverage: 0.2,
                first_section: "第一章".into(),
                last_section: "第一章".into(),
                stages: vec![PlotlineStage {
                    section: "第一章".into(),
                    mentions: 3,
                    gap_before: 0,
                }],
                longest_gap: 0,
                shape: "setup_only".into(),
                is_main: false,
            },
        ];
        plot.turning_points = vec![TurningPoint {
            section: "第三章".into(),
            change: 0.6,
            appeared: vec!["Tobey".into()],
            dropped: vec!["Chase".into()],
        }];
        plot.unresolved_threads = vec!["Chase".into()];
        plot.late_arrivals = vec!["Tobey".into()];

        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);
        assert!(md.contains("## 情节线"));
        assert!(md.contains("情节连贯性：**84.0 / 100**；共 3 条线索（5 个章节）"));
        assert!(md.contains("| Marlowe | PERSON | 是 | 贯穿 | 12 | 100% | 第一章 → 第五章 |"));
        assert!(md.contains("| Chase | PERSON | — | 仅铺垫 | 3 | 20% | 第一章 → 第一章 |"));
        assert!(md.contains("### 转折点（1 处）"));
        assert!(md.contains("| 第三章 | 60% | Tobey | Chase |"));
        assert!(md.contains("铺垫未回收（1）：Chase"));
        assert!(md.contains("结尾登场（1）：Tobey"));
    }

    #[test]
    fn test_pacing_section_renders_curve_stretches_and_drag() {
        let (t0, t1, plot, a) = (empty_t0(), empty_t1(), empty_plot(), clean_assessment());
        let mut pacing = empty_pacing();
        let conflict = empty_conflict();
        let theme = empty_theme();
        let setting = empty_setting();
        let cliche = empty_cliche();
        pacing.pace_score = 62.0;
        pacing.verdict = "varied".into();
        pacing.segment_count = 4;
        pacing.tension_stdev = 18.4;
        pacing.points = vec![
            PacingPoint {
                section: "第一章".into(),
                char_count: 900,
                sentence_count: 40,
                avg_sentence_chars: 22.0,
                sentence_stdev: 9.0,
                dialogue_share: 0.05,
                entity_density: 3.0,
                tension: 12.0,
            },
            PacingPoint {
                section: "第二章".into(),
                char_count: 1200,
                sentence_count: 55,
                avg_sentence_chars: 21.0,
                sentence_stdev: 11.0,
                dialogue_share: 0.42,
                entity_density: 8.0,
                tension: 76.0,
            },
        ];
        pacing.flat_stretches = vec![FlatStretch {
            from_section: "第一章".into(),
            to_section: "第二章".into(),
            sections: 4,
            mean_tension: 14.0,
            char_count: 4200,
        }];
        pacing.drag_sections = vec![DragSection {
            section: "第一章".into(),
            char_count: 2100,
            tension: 12.0,
            length_ratio: 2.4,
        }];

        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);
        assert!(md.contains(
            "节奏评分：**62.0 / 100**；判定：**起伏**（`varied`）· 4 段 · 张力标准差 18.4"
        ));
        // Tension is drawn on the same 0-100 scale the UI charts use.
        assert!(md.contains("| 第一章 | █░░░░░░░░░ 12 | 900 | 40 | 22.0 | 5% |"));
        assert!(md.contains("| 第二章 | ████████░░ 76 | 1200 | 55 | 21.0 | 42% |"));
        assert!(md.contains("### 平淡段落（1 处）"));
        assert!(md.contains("| 第一章 | 第二章 | 4 | 14 | 4200 |"));
        assert!(md.contains("### 拖沓章节（1 处）"));
        assert!(md.contains("| 第一章 | 2100 | 2.4× | 12 |"));
    }

    #[test]
    fn test_conflict_section_renders_types_curve_and_absence() {
        let (t0, t1, plot, a) = (empty_t0(), empty_t1(), empty_plot(), clean_assessment());
        let pacing = empty_pacing();
        let mut conflict = empty_conflict();
        let theme = empty_theme();
        let setting = empty_setting();
        let cliche = empty_cliche();
        conflict.conflict_score = Some(71.0);
        conflict.verdict = "present".into();
        conflict.total_hits = 9;
        conflict.density = 4.2;
        conflict.dominant_type = "interpersonal".into();
        conflict.segment_count = 4;
        conflict.types = vec![
            ConflictType {
                kind: "interpersonal".into(),
                hits: 6,
                share: 0.667,
                density: 2.8,
                top_terms: vec![("战斗".into(), 3), ("威胁".into(), 2)],
            },
            ConflictType {
                kind: "internal".into(),
                hits: 3,
                share: 0.333,
                density: 1.4,
                top_terms: vec![("恐惧".into(), 3)],
            },
        ];
        conflict.points = vec![
            ConflictPoint {
                section: "第一章".into(),
                char_count: 1000,
                hits: 0,
                density: 0.0,
                intensity: 0.0,
                dominant: String::new(),
                by_type: vec![],
            },
            ConflictPoint {
                section: "第二章".into(),
                char_count: 800,
                hits: 6,
                density: 7.5,
                intensity: 93.75,
                dominant: "interpersonal".into(),
                by_type: vec![("interpersonal".into(), 6)],
            },
        ];
        conflict.peak_section = "第二章".into();
        conflict.peak_intensity = 93.75;
        conflict.peak_position = 1.0;
        conflict.tail_intensity = 46.9;
        conflict.longest_absent_run = 3;
        conflict.absent_runs = vec![ConflictAbsence {
            from_section: "第一章".into(),
            to_section: "第一章".into(),
            sections: 3,
            char_count: 3000,
        }];

        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);

        assert!(md.contains(
            "冲突评分：**71.0 / 100** · 判定：**适中**（`present`）· 命中 9 处 · 4.2 处/千字 · 主类型：人际"
        ));
        assert!(md.contains("| 人际 | 6 | 67% | 2.8 | 战斗×3、威胁×2 |"));
        assert!(md.contains("| 内心 | 3 | 33% | 1.4 | 恐惧×3 |"));
        assert!(md.contains(
            "峰值：第二章（94，位于全文 100%）· 结尾三分之一平均 47 · 最长无冲突区间 3 段"
        ));
        // The same 0-100 bar the pacing curve uses, so the two sections read alike.
        assert!(md.contains("| 第一章 | ░░░░░░░░░░ 0 | 0 | 0.0 | 无 |"));
        assert!(md.contains("| 第二章 | █████████░ 94 | 6 | 7.5 | 人际 |"));
        assert!(md.contains("### 无冲突区间（1 处）"));
        assert!(md.contains("| 第一章 | 第一章 | 3 | 3000 |"));
    }

    #[test]
    fn test_conflict_without_vocabulary_says_so_instead_of_scoring() {
        let (t0, t1, plot, pacing, conflict, theme, setting, cliche, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
            clean_assessment(),
        );
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);

        assert!(md.contains("未检测到冲突用词，因此**不给出评分**"));
        assert!(md.contains("### 冲突类型（0 类）"));
        assert!(!md.contains("冲突评分："));
    }

    #[test]
    fn test_theme_section_renders_list_shares_and_folding() {
        let (t0, t1, plot, pacing, conflict, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            clean_assessment(),
        );
        let mut theme = empty_theme();
        let setting = empty_setting();
        let cliche = empty_cliche();
        theme.theme_score = Some(88.0);
        theme.source = "noun_signals".into();
        theme.candidate_count = 2;
        theme.total_mentions = 12;
        theme.top_share = 0.583;
        theme.top3_share = 0.917;
        theme.singleton_share = 0.083;
        theme.section_count = 6;
        theme.themes = vec![
            Theme {
                term: "出版".into(),
                members: vec![("出版".into(), 5), ("出版业".into(), 2)],
                mentions: 7,
                sections_touched: 6,
                coverage: 1.0,
                share: 0.583,
                first_section: "第一章".into(),
                last_section: "第六章".into(),
                reach: "sustained".into(),
                longest_gap: 0,
                concentration: "spread".into(),
                curve: vec![1, 1, 2, 1, 1, 1],
            },
            Theme {
                term: "合同".into(),
                members: vec![("合同".into(), 5)],
                mentions: 5,
                sections_touched: 2,
                coverage: 0.333,
                share: 0.417,
                first_section: "第五章".into(),
                last_section: "第六章".into(),
                reach: "episodic".into(),
                longest_gap: 4,
                concentration: "back".into(),
                curve: vec![0, 0, 0, 0, 3, 2],
            },
        ];

        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);

        assert!(md.contains(
            "主题评分：**88.0 / 100** · 词汇来源：**名词信号**（`noun_signals`）· 2 个主题 · 候选词命中 12 次"
        ));
        assert!(md.contains("### 主题清单（2 个）"));
        assert!(md.contains("| 出版 | 7 | 100% | 58.3% | 第一章 | 第六章 | 贯穿 | 均匀 |"));
        assert!(md.contains("| 合同 | 5 | 33% | 41.7% | 第五章 | 第六章 | 偶发 | 集中在结尾 |"));
        // The fold is printed, so a reader can see that 出版 and 出版业 were counted
        // as one subject rather than two.
        assert!(md.contains("- 出版：出版×5、出版业×2"));
        assert!(md.contains("反复出现不等于母题"));
    }

    #[test]
    fn test_theme_without_vocabulary_says_so_instead_of_scoring() {
        let (t0, t1, plot, pacing, conflict, theme, setting, cliche, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
            clean_assessment(),
        );
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);

        assert!(md.contains("没有可用于主题分析的词汇"));
        assert!(md.contains("未识别到主题：没有词达到 3 次提及且跨越 2 段以上"));
        assert!(!md.contains("主题评分："));
    }

    #[test]
    fn test_setting_section_renders_places_scenes_and_issues() {
        let (t0, t1, plot, pacing, conflict, theme, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            clean_assessment(),
        );
        let mut setting = empty_setting();
        let cliche = empty_cliche();
        setting.setting_score = Some(75.0);
        setting.scene_count = 3;
        setting.scenes = vec![
            Scene {
                index: 0,
                section: "第一章".into(),
                anchor: "码头".into(),
                anchor_kind: "FACILITY".into(),
                blocks: 4,
                char_count: 800,
                env_hits: 6,
                env_density: 7.5,
                dialogue_share: 0.2,
                time_mark: false,
                persons: vec![("老陈".into(), 3)],
            },
            Scene {
                index: 1,
                section: "第二章".into(),
                anchor: String::new(),
                anchor_kind: String::new(),
                blocks: 2,
                char_count: 400,
                env_hits: 0,
                env_density: 0.0,
                dialogue_share: 0.8,
                time_mark: true,
                persons: vec![],
            },
        ];
        setting.unanchored_scenes = 1;
        setting.unanchored_share = 0.5;
        setting.places = vec![Place {
            name: "码头".into(),
            kind: "FACILITY".into(),
            mentions: 5,
            segments_touched: 3,
            coverage: 0.6,
            first_section: "第一章".into(),
            last_section: "第三章".into(),
            longest_absence: 3,
            returns: 1,
            curve: vec![2, 0, 0, 0, 2, 1],
        }];
        setting.place_count = 2;
        setting.recurring_places = 1;
        setting.total_env_hits = 20;
        setting.env_density = 4.2;
        setting.verdict = "present".into();
        setting.dominant_place = "码头".into();
        setting.peak_section = "第一章".into();
        setting.peak_intensity = 62.5;
        setting.longest_absence = 3;
        setting.issues = vec![
            SettingIssue {
                kind: "unanchored".into(),
                section: "第二章".into(),
                detail: "第 2 场没有点名地点，环境用词也只有 0.0 次/千字".into(),
            },
            SettingIssue {
                kind: "long_absence".into(),
                section: "第一章".into(),
                detail: "码头 最长缺席 3 段，之后又出现 1 次".into(),
            },
        ];
        setting.top_env_terms = vec![("房间".into(), 6), ("夜色".into(), 4)];

        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        };
        let md = render_markdown(&input);

        assert!(md.contains(
            "场景评分：**75.0 / 100** · 3 场 · 2 个地点 · 环境用词 20 次（4.2 次/千字，判定：**适中**（`present`））"
        ));
        assert!(
            md.contains("没有地点锚点的场景：1/3（50%）· 主导地点：码头 · 环境峰值：第一章（62）")
        );
        assert!(md.contains("### 地点（1 个）"));
        assert!(md.contains("| 码头 | 场所 | 5 | 60% | 第一章 | 第三章 | 3 |"));
        assert!(md.contains("### 场景清单（2 场）"));
        assert!(md.contains("| 1 | 第一章 | 码头 | 4 | 800 | 7.5 | 20% | — |"));
        // A scene the text never places is printed as such rather than dropped.
        assert!(md.contains("| 2 | 第二章 | （未点名） | 2 | 400 | 0.0 | 80% | 是 |"));
        assert!(md.contains("- **无地点锚点**（第二章）— 第 2 场没有点名地点"));
        assert!(md.contains("- **长时间离开地点**（第一章）— 码头 最长缺席 3 段"));
        assert!(md.contains("环境用词：房间×6、夜色×4"));
        assert!(md.contains("是否真的换了场景需要人来核对"));
    }

    #[test]
    fn test_cliche_section_prints_phrases_hits_and_issues() {
        let (t0, t1, plot, pacing, conflict, theme, setting, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            clean_assessment(),
        );
        let mut cliche = empty_cliche();
        cliche.cliche_score = Some(72.5);
        cliche.total_hits = 12;
        cliche.distinct_phrases = 2;
        cliche.char_count = 8000;
        cliche.per_10k = 15.0;
        cliche.verdict = "moderate".into();
        cliche.categories = vec![("expression".into(), 7), ("body".into(), 5)];
        cliche.top_share = 0.583;
        cliche.top_phrase = "微微一笑".into();
        cliche.peak_section = "第二章".into();
        cliche.peak_intensity = 62.0;
        cliche.repeated_phrases = 1;
        cliche.repeat_hits = 7;
        cliche.dialogue_hits = 2;
        cliche.segment_count = 4;
        cliche.points = vec![ClichePoint {
            section: "第二章".into(),
            char_count: 2000,
            hits: 5,
            per_10k: 25.0,
            intensity: 62.5,
            top_phrase: "微微一笑".into(),
        }];
        cliche.phrases = vec![
            ClichePhrase {
                phrase: "微微一笑".into(),
                category: "expression".into(),
                count: 7,
                share: 0.583,
                variants: vec![("微微一笑".into(), 7)],
                segments_touched: 3,
                coverage: 0.75,
                first_section: "第一章".into(),
                last_section: "第四章".into(),
                note: "万能表情，换成这个人物特有的面部反应".into(),
            },
            ClichePhrase {
                phrase: "(淡淡|浅浅)一笑".into(),
                category: "expression".into(),
                count: 5,
                share: 0.417,
                variants: vec![("淡淡一笑".into(), 3), ("浅浅一笑".into(), 2)],
                segments_touched: 2,
                coverage: 0.5,
                first_section: "第二章".into(),
                last_section: "第三章".into(),
                note: "「一笑」能接在任何一句对白后，改写成说话时的具体动作".into(),
            },
        ];
        cliche.hits = vec![
            ClicheHit {
                section: "第一章".into(),
                phrase: "微微一笑".into(),
                pattern: "微微一笑".into(),
                category: "expression".into(),
                offset: 12,
                excerpt: "她微微一笑，把杯子放下".into(),
                in_dialogue: false,
                note: "万能表情".into(),
            },
            ClicheHit {
                section: "第二章".into(),
                phrase: "淡淡一笑".into(),
                pattern: "(淡淡|浅浅)一笑".into(),
                category: "expression".into(),
                offset: 4,
                excerpt: "“淡淡一笑。”他先开口".into(),
                in_dialogue: true,
                note: "「一笑」能接在任何一句对白后".into(),
            },
        ];
        cliche.issues = vec![
            ClicheIssue {
                kind: "repeated".into(),
                section: "第一章".into(),
                detail: "「微微一笑」出现 7 次（微微一笑×7）".into(),
            },
            ClicheIssue {
                kind: "one_note".into(),
                section: String::new(),
                detail: "最常用的一句占了全部命中 58%".into(),
            },
        ];

        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let md = render_markdown(&ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        });

        assert!(md.contains(
            "用词评分：**72.5 / 100** · 命中 12 处（15.0 处/万字，判定：**中等**（`moderate`））· 涉及 2 个短语 · 对白内 2 处"
        ));
        // The library size rides along only when it found something, so the reader
        // knows how wide the net was.
        assert!(md.contains("词库 150 项"));
        assert!(md.contains(
            "最常用：「微微一笑」（占命中 58%）· 峰值段落：第二章（62）· 反复使用的短语 1 个"
        ));
        assert!(md.contains("类型分布：表情动作×7、身体反应×5"));
        assert!(md.contains("### 高频套话（2 个）"));
        assert!(md.contains("| 微微一笑 | 表情动作 | 7 | 58% | 75% | 第一章 | 第四章 |"));
        // A pattern's `|` is escaped, so it cannot end the table cell.
        assert!(md.contains("| (淡淡\\|浅浅)一笑 | 表情动作 | 5 | 42% | 50% | 第二章 | 第三章 |"));
        assert!(md.contains("### 怎么改"));
        assert!(md.contains("- **微微一笑** — 万能表情，换成这个人物特有的面部反应"));
        assert!(md.contains("### 命中位置（前 2 处）"));
        assert!(md.contains("- **第一章** · 微微一笑 · 她微微一笑，把杯子放下（第 12 字）"));
        assert!(md.contains("- **第二章** · 淡淡一笑 · “淡淡一笑。”他先开口（第 4 字）· 对白内"));
        assert!(md.contains("- **反复使用**（第一章）— 「微微一笑」出现 7 次"));
        assert!(md.contains("- **依赖同一句**（全文）— 最常用的一句占了全部命中 58%"));
        assert!(md.contains("由人来判断"));
    }

    #[test]
    fn test_dialogue_section_renders_split_curve_and_issues() {
        let mut dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        dialogue.char_count = 2000;
        dialogue.dialogue_chars = 600;
        dialogue.narrative_chars = 1400;
        dialogue.dialogue_share = 0.3;
        dialogue.narrative_share = 0.7;
        dialogue.unit_count = 24;
        dialogue.quoted_units = 20;
        dialogue.dash_units = 4;
        dialogue.avg_unit_chars = 25.0;
        dialogue.longest_unit = 320;
        dialogue.unit_length_distribution = vec![
            ("1-9".into(), 6),
            ("10-29".into(), 14),
            ("30-99".into(), 3),
            ("100-299".into(), 0),
            ("300+".into(), 1),
        ];
        dialogue.blocks = 40;
        dialogue.blocks_with_dialogue = 18;
        dialogue.longest_dialogue_run = 5;
        dialogue.longest_quiet_run = 9;
        dialogue.longest_quiet_chars = 1200;
        dialogue.silent_segments = 1;
        dialogue.flooded_segments = 1;
        dialogue.verdict = "balanced".into();
        dialogue.segment_count = 3;
        dialogue.spread = 0.6;
        dialogue.peak_section = "Ch1".into();
        dialogue.peak_share = 0.62;
        dialogue.quietest_section = "Ch3".into();
        dialogue.quietest_share = 0.02;
        dialogue.points = vec![
            DialoguePoint {
                section: "Ch1".into(),
                char_count: 700,
                dialogue_chars: 434,
                share: 0.62,
                intensity: 62.0,
                units: 12,
                longest_unit: 320,
                blocks: 14,
                blocks_with_dialogue: 11,
            },
            DialoguePoint {
                section: "Ch3".into(),
                char_count: 600,
                dialogue_chars: 12,
                share: 0.02,
                intensity: 2.0,
                units: 1,
                longest_unit: 12,
                blocks: 9,
                blocks_with_dialogue: 1,
            },
        ];
        dialogue.samples = vec![DialogueSample {
            section: "Ch1".into(),
            chars: 320,
            kind: "quote".into(),
            excerpt: "他站在门口，说了很久。".into(),
        }];
        dialogue.issues = vec![DialogueIssue {
            kind: "silent_segment".into(),
            section: "Ch3".into(),
            detail: "连续 9 个文本块、1200 字没有对白".into(),
        }];

        let t0 = empty_t0();
        let t1 = empty_t1();
        let plot = empty_plot();
        let pacing = empty_pacing();
        let conflict = empty_conflict();
        let theme = empty_theme();
        let setting = empty_setting();
        let cliche = empty_cliche();
        let assessment = clean_assessment();
        let md = render_markdown(&ReportInput {
            project_name: "测试项目",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &assessment,
        });
        // The all-narrative wording is only for text where no speech was detected.
        assert!(!md.contains("对白占比：**0.0%** · 叙述 100.0%"));
        assert!(md.contains("对白占比：**30.0%** · 叙述 70.0% · 对白 24 处（引号 20 · 破折号 4）"));
        assert!(md.contains("判定：**对白与叙述混合**（`balanced`）"));
        assert!(md.contains("| Ch1 | ██"));
        assert!(md.contains("| 1-9 | 6 |"));
        assert!(md.contains("| 300+ | 1 |"));
        assert!(md.contains("- **320 字** · Ch1 · 引号 · 「他站在门口，说了很久。」"));
        assert!(md.contains("连续无对白最长 9 块 / 1200 字 · 连续对白最长 5 块"));
        assert!(md.contains("- **长时间无对白**（Ch3）— 连续 9 个文本块、1200 字没有对白"));
    }

    #[test]
    fn test_short_text_reports_hits_without_a_score() {
        let (t0, t1, plot, pacing, conflict, theme, setting, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            clean_assessment(),
        );
        let mut cliche = empty_cliche();
        cliche.char_count = 300;
        cliche.total_hits = 2;
        cliche.distinct_phrases = 1;
        cliche.phrases = vec![ClichePhrase {
            phrase: "深吸一口气".into(),
            category: "body".into(),
            count: 2,
            share: 1.0,
            variants: vec![("深吸一口气".into(), 2)],
            segments_touched: 1,
            coverage: 1.0,
            first_section: "1/10".into(),
            last_section: "3/10".into(),
            note: "紧张或平静的通用动作".into(),
        }];

        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        let md = render_markdown(&ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        });

        assert!(md.contains("全文 300 字，不足 1000 字"));
        assert!(!md.contains("用词评分："));
        // The hits survive the missing grade: the reader still sees what was found.
        assert!(md.contains("| 深吸一口气 | 身体反应 | 2 | 100% | 100% | 1/10 | 3/10 |"));
    }

    #[test]
    fn test_explicit_section_renders_advisory_tiers_and_hits() {
        let (t0, t1, plot, pacing, conflict, theme, setting, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            clean_assessment(),
        );
        let cliche = empty_cliche();
        let dialogue = empty_dialogue();

        let mut explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();
        explicit.total_hits = 4;
        explicit.distinct_terms = 2;
        explicit.per_10k = 12.0;
        explicit.advisory = "strong".into();
        explicit.tiers = vec![
            ("mild".into(), 1),
            ("moderate".into(), 0),
            ("strong".into(), 3),
        ];
        explicit.categories = vec![("violence".into(), 3), ("self_harm".into(), 1)];
        explicit.advisory_categories = vec!["violence".into(), "self_harm".into()];
        explicit.terms = vec![
            ExplicitTerm {
                term: "(屠杀|割喉)".into(),
                category: "violence".into(),
                tier: "strong".into(),
                count: 3,
                share: 0.75,
                variants: vec![("割喉".into(), 3)],
                segments_touched: 2,
                coverage: 0.5,
                first_section: "第一章".into(),
                last_section: "第三章".into(),
                note: "血腥细节：读者提示的主要依据".into(),
            },
            ExplicitTerm {
                term: "(自杀|轻生)".into(),
                category: "self_harm".into(),
                tier: "strong".into(),
                count: 1,
                share: 0.25,
                variants: vec![("轻生".into(), 1)],
                segments_touched: 1,
                coverage: 0.25,
                first_section: "第二章".into(),
                last_section: "第二章".into(),
                note: "自伤或自杀描写：多数平台建议读者提示".into(),
            },
        ];
        explicit.hits = vec![
            ExplicitHit {
                section: "第一章".into(),
                term: "割喉".into(),
                pattern: "(屠杀|割喉)".into(),
                category: "violence".into(),
                tier: "strong".into(),
                offset: 12,
                excerpt: "他拔刀割喉。".into(),
                in_dialogue: false,
                note: "血腥细节".into(),
            },
            ExplicitHit {
                section: "第二章".into(),
                term: "轻生".into(),
                pattern: "(自杀|轻生)".into(),
                category: "self_harm".into(),
                tier: "strong".into(),
                offset: 5,
                excerpt: "他低声说：“我想轻生。”".into(),
                in_dialogue: true,
                note: "自伤描写".into(),
            },
        ];
        explicit.dialogue_hits = 1;
        explicit.top_share = 0.75;
        explicit.top_term = "(屠杀|割喉)".into();
        explicit.peak_section = "第一章".into();
        explicit.peak_intensity = 40.0;
        explicit.issues = vec![
            ExplicitIssue {
                kind: "self_harm".into(),
                section: "第二章".into(),
                detail: "1 处自伤或自杀描写，最集中于第二章（1 处）".into(),
            },
            ExplicitIssue {
                kind: "gore".into(),
                section: "第一章".into(),
                detail: "3 处血腥细节描写，最集中于第一章（3 处）".into(),
            },
        ];

        let md = render_markdown(&ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        });

        assert!(md.contains("## 显式语言"));
        assert!(md.contains("内容分级：**强烈**（`strong`）· 命中 4 处"));
        // All three tiers print even though the middle one is empty: a zero has to look
        // like a measurement and not like a missing one.
        assert!(md.contains("按强度：轻 1 处 · 中 0 处 · 重 3 处"));
        assert!(md.contains("需要读者提示的内容：暴力血腥、自伤"));
        assert!(md.contains("类型分布：暴力血腥×3、自伤×1"));
        assert!(md.contains("| (屠杀\\|割喉) | 暴力血腥 | 重 | 3 | 75% | 50% | 第一章 | 第三章 |"));
        assert!(md.contains("- **第一章** · 割喉 · 他拔刀割喉。（第 12 字）重 · 叙述"));
        assert!(md.contains("- **第二章** · 轻生 · 他低声说：“我想轻生。”（第 5 字）重 · 对白内"));
        assert!(md.contains("- **自伤描写**（第二章）— 1 处自伤或自杀描写"));
        assert!(md.contains("- **血腥细节**（第一章）— 3 处血腥细节描写"));
        // The dimension is measured, never graded: no score line belongs to it.
        assert!(md.contains("_本维度不给出评分：一部战争题材并不比田园题材更差"));
    }

    #[test]
    fn test_explicit_silence_says_the_library_found_nothing() {
        let (t0, t1, plot, pacing, conflict, theme, setting, cliche, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            empty_cliche(),
            clean_assessment(),
        );
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let structure = empty_structure();
        let timeline = empty_timeline();

        let md = render_markdown(&ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &structure,
            timeline: &timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        });

        // "词库未见" rather than "无敏感内容": silence is about the library, not the text.
        assert!(md.contains("内容分级：**词库未见**（`none`）"));
        assert!(md.contains("没有命中词库里的用词。"));
        assert!(!md.contains("需要读者提示的内容："));
        // The library size is printed only next to hits, where it means something.
        assert!(!md.contains("词库 35 项"));
    }

    /// A shape the signals can place: two landmarks, the document's own acts, and one
    /// model with a beat in place, one out of place and one not shown.
    fn shaped_structure() -> StructureStats {
        StructureStats {
            landmarks: vec![
                StructureLandmark {
                    kind: "opening".into(),
                    section: "第一章".into(),
                    position: 0.08,
                    strength: 0.0,
                    detail: "开篇于 第一章，张力 0".into(),
                },
                StructureLandmark {
                    kind: "climax".into(),
                    section: "第五章".into(),
                    position: 0.8,
                    strength: 85.0,
                    detail: "全文张力最高点 85（均值 25）".into(),
                },
            ],
            acts: vec![
                StructureAct {
                    name: "act_one".into(),
                    from_section: "第一章".into(),
                    to_section: "第一章".into(),
                    char_count: 600,
                    share: 0.16,
                    mean_tension: 0.0,
                    min_tension: 0.0,
                    max_tension: 0.0,
                },
                StructureAct {
                    name: "act_two".into(),
                    from_section: "第二章".into(),
                    to_section: "第四章".into(),
                    char_count: 2200,
                    share: 0.54,
                    mean_tension: 20.0,
                    min_tension: 0.0,
                    max_tension: 67.0,
                },
                StructureAct {
                    name: "act_three".into(),
                    from_section: "第五章".into(),
                    to_section: "第六章".into(),
                    char_count: 1250,
                    share: 0.3,
                    mean_tension: 42.0,
                    min_tension: 0.0,
                    max_tension: 85.0,
                },
            ],
            models: vec![StructureModel {
                model: "three_act".into(),
                fit: "partial".into(),
                beats: vec![
                    StructureBeat {
                        beat: "climax".into(),
                        window: (0.76, 0.96),
                        status: "found".into(),
                        section: "第五章".into(),
                        position: 0.8,
                        offset: 0.0,
                        evidence: "全文张力最高点 85（均值 25）".into(),
                    },
                    StructureBeat {
                        beat: "midpoint".into(),
                        window: (0.42, 0.62),
                        status: "displaced".into(),
                        section: "第二章".into(),
                        position: 0.21,
                        offset: 0.21,
                        evidence: "线程变化 33%：新出现：沈墨".into(),
                    },
                    StructureBeat {
                        beat: "resolution".into(),
                        window: (0.9, 1.0),
                        status: "missing".into(),
                        section: String::new(),
                        position: 0.0,
                        offset: 0.0,
                        evidence: String::new(),
                    },
                ],
                found: 1,
                displaced: 1,
                missing: 1,
            }],
            issues: vec![StructureIssue {
                kind: "climax_early".into(),
                section: "第一章".into(),
                detail: "张力最高点落在 8%（第一章），其后还有 92% 的正文".into(),
            }],
            segment_count: 6,
            chunked: false,
            char_count: 4050,
            turn_count: 3,
            readable: true,
            note: String::new(),
        }
    }

    /// A report built only to look at one dimension's section.
    fn render_with(structure: &StructureStats) -> String {
        render_sections(structure, &empty_timeline())
    }

    /// The same, for the time-line section.
    fn render_timeline_with(timeline: &TimelineStats) -> String {
        render_sections(&empty_structure(), timeline)
    }

    fn render_sections(structure: &StructureStats, timeline: &TimelineStats) -> String {
        let t0 = empty_t0();
        let t1 = empty_t1();
        let plot = empty_plot();
        let pacing = empty_pacing();
        let conflict = empty_conflict();
        let theme = empty_theme();
        let setting = empty_setting();
        let cliche = empty_cliche();
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let a = clean_assessment();
        render_markdown(&ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure,
            timeline,
            voice: &VoiceStats::default(),
            assessment: &a,
        })
    }

    /// The same, for the author-voice section.
    fn render_voice_with(voice: &VoiceStats) -> String {
        let t0 = empty_t0();
        let t1 = empty_t1();
        let plot = empty_plot();
        let pacing = empty_pacing();
        let conflict = empty_conflict();
        let theme = empty_theme();
        let setting = empty_setting();
        let cliche = empty_cliche();
        let dialogue = empty_dialogue();
        let explicit = empty_explicit();
        let a = clean_assessment();
        render_markdown(&ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            plot: &plot,
            pacing: &pacing,
            conflict: &conflict,
            theme: &theme,
            setting: &setting,
            cliche: &cliche,
            dialogue: &dialogue,
            explicit: &explicit,
            structure: &empty_structure(),
            timeline: &empty_timeline(),
            voice,
            assessment: &a,
        })
    }

    #[test]
    fn test_voice_section_reports_rhythm_and_names_no_score() {
        let voice = VoiceStats {
            char_count: 4000,
            sentence_count: 120,
            avg_sentence_chars: 33.4,
            sentence_stdev: 21.0,
            sentence_cv: 0.63,
            shortest_sentence: 4,
            longest_sentence: 180,
            clause_count: 300,
            avg_clause_chars: 13.4,
            clauses_per_sentence: 2.5,
            paragraph_count: 60,
            avg_paragraph_chars: 66.7,
            paragraph_stdev: 40.0,
            single_sentence_paragraphs: 12,
            marks: vec![VoiceMark {
                mark: "，".into(),
                label: "逗号".into(),
                count: 260,
                per_1000: 65.0,
            }],
            function_words: vec![VoiceWordGroup {
                group: "结构助词".into(),
                words: "的地得".into(),
                count: 180,
                per_1000: 45.0,
            }],
            pronouns: VoicePronouns {
                first: 30,
                second: 5,
                third: 65,
                total: 100,
                first_share: 0.3,
                second_share: 0.05,
                third_share: 0.65,
            },
            stance: VoiceStance {
                hedging: 12,
                assertive: 40,
                hedging_per_10k: 3.0,
                assertive_per_10k: 10.0,
                verdict: "assertive".into(),
            },
            points: vec![VoicePoint {
                section: "第一章".into(),
                char_count: 2000,
                sentences: 60,
                avg_sentence_chars: 33.3,
                clauses_per_sentence: 2.5,
            }],
            issues: vec![VoiceIssue {
                kind: "varied_rhythm".into(),
                detail: "句长差异大（标准差 21.0，变异系数 0.63）：长句和短句交替出现。".into(),
            }],
            segment_count: 1,
            chunked: false,
        };
        let md = render_voice_with(&voice);
        assert!(md.contains("## 作者声音"));
        assert!(md.contains("| 平均句长（字） | 33.4 |"));
        assert!(md.contains("| 句长变异系数 | 0.63 |"));
        assert!(md.contains("| 逗号（，） | 260 | 65.00 |"));
        assert!(md.contains("| 结构助词 | 的地得 | 180 | 45.00 |"));
        assert!(md.contains("| 第三人称 | 65（65%） |"));
        assert!(md.contains("| 立场 | 偏断言 |"));
        assert!(md.contains("| 第一章 | 2000 | 60 | 33.3 | 2.50 |"));
        assert!(md.contains("- 句长差异大"));
        // The section says what the numbers are measured with, and refuses a grade.
        assert!(md.contains("长句不比短句好"));
        assert!(md.contains("按单字计数"));
    }

    #[test]
    fn test_voice_section_says_there_is_nothing_to_measure() {
        let md = render_voice_with(&VoiceStats::default());
        assert!(md.contains("## 作者声音"));
        assert!(md.contains("没有可统计的文本。"));
    }

    #[test]
    fn test_story_structure_section_describes_the_shape() {
        let md = render_with(&shaped_structure());
        assert!(md.contains("## 故事结构"));
        assert!(
            md.contains("地标 2 个 · 转折点 3 个 · 曲线段落 6 段（按作者章节）· 可见正文 4050 字")
        );
        assert!(md.contains("| 80% | 第五章 | 张力最高点 | 全文张力最高点 85（均值 25） |"));
        assert!(md.contains("| 第一幕 | 第一章 | 第一章 | 600 | 16% | 0 / 0 / 0 |"));
        assert!(md.contains("- **三幕结构**：部分吻合（命中 1 · 错位 1 · 未见 1）"));
        assert!(md.contains("#### 三幕结构：节拍落点"));
        assert!(md.contains("| 高潮 | 76%–96% | 命中 | 第五章（80%） | — |"));
        assert!(md.contains("| 中点 | 42%–62% | 错位 | 第二章（21%） | 21% |"));
        // 未见 keeps its row with nothing filled in: the absence is the finding.
        assert!(md.contains("| 收束 | 90%–100% | 未见 | — | — | — |"));
        assert!(md.contains("- **张力最高点偏早**（第一章）— 张力最高点落在 8%"));
    }

    #[test]
    fn test_story_structure_never_grades_the_shape() {
        let md = render_with(&shaped_structure());
        // No score line anywhere, and the note says what 未见 does not mean.
        assert!(!md.contains("结构评分"));
        assert!(md.contains("_本维度不给出评分：契合三幕不是优点"));
        assert!(md.contains("「未见」只说明这些信号没有显示这一环，不等于故事缺少它"));
    }

    #[test]
    fn test_story_structure_says_when_it_could_not_read_the_document() {
        let mut structure = empty_structure();
        structure.char_count = 800;
        structure.note = "全文可见正文 800 字，不足以按位置匹配经典结构（需 1500 字以上）".into();
        let md = render_with(&structure);
        assert!(md.contains(
            "全文可见正文 800 字，不足以按位置匹配经典结构（需 1500 字以上）本次不做模型对照。"
        ));
        assert!(!md.contains("#### 三幕结构：节拍落点"));
        assert!(!md.contains("### 地标（0 个）"));
    }

    #[test]
    fn test_story_structure_flags_arithmetic_positions() {
        let mut structure = shaped_structure();
        structure.chunked = true;
        structure.segment_count = 10;
        let md = render_with(&structure);
        assert!(md.contains("曲线段落 10 段（按字数等分，非作者章节）"));
        assert!(md.contains("位置是算术结果，不是作者自己的分段"));
    }

    /// A document that gives two dates and goes backwards between them.
    fn dated_timeline() -> TimelineStats {
        TimelineStats {
            markers: vec![
                TimelineMarker {
                    text: "2011年5月1日".into(),
                    kind: "absolute".into(),
                    precision: "day".into(),
                    section: "第一章".into(),
                    position: 0.1,
                    resolved: "2011-05-01".into(),
                    offset_days: 0.0,
                    excerpt: String::new(),
                },
                TimelineMarker {
                    text: "三天后".into(),
                    kind: "relative".into(),
                    precision: "offset".into(),
                    section: "第二章".into(),
                    position: 0.4,
                    resolved: String::new(),
                    offset_days: 3.0,
                    excerpt: String::new(),
                },
                TimelineMarker {
                    text: "1998年3月2日".into(),
                    kind: "absolute".into(),
                    precision: "day".into(),
                    section: "第二章".into(),
                    position: 0.6,
                    resolved: "1998-03-02".into(),
                    offset_days: 0.0,
                    excerpt: String::new(),
                },
            ],
            curve: vec![
                TimelinePoint {
                    label: "第一章".into(),
                    markers: 1,
                },
                TimelinePoint {
                    label: "第二章".into(),
                    markers: 2,
                },
            ],
            absolute_count: 2,
            partial_count: 0,
            relative_count: 1,
            forms: vec![
                ("day".into(), 2),
                ("offset".into(), 1),
            ],
            vague_count: 0,
            entity_date_count: 0,
            first_date: "2011-05-01".into(),
            last_date: "1998-03-02".into(),
            span_days: -4809,
            backsteps: vec![TimelineStep {
                from_section: "第一章".into(),
                from_date: "2011-05-01".into(),
                to_section: "第二章".into(),
                to_date: "1998-03-02".into(),
                days: -4809,
            }],
            jumps: vec![],
            unmarked_stretches: vec![],
            issues: vec![TimelineIssue {
                kind: "backstep".into(),
                section: "第二章".into(),
                detail: "「第一章」的 2011-05-01 之后，「第二章」又出现 1998-03-02，早了 4809 天：倒叙还是笔误，文本分不出".into(),
            }],
            segment_count: 2,
            chunked: false,
            char_count: 5000,
            readable: true,
            note: String::new(),
        }
    }

    #[test]
    fn test_timeline_section_lists_dates_and_names_the_backstep() {
        let md = render_timeline_with(&dated_timeline());
        assert!(md.contains("## 时间线"));
        assert!(md.contains("时间标记 3 处（绝对 2 · 月日 0 · 相对 1 · 模糊 0）· 可排序日期 2 个"));
        assert!(md.contains("| 10% | 第一章 | 2011-05-01 | 到日 |"));
        assert!(md.contains("### 时间倒回（1 处）"));
        assert!(md.contains("| 第一章 | 2011-05-01 | 第二章 | 1998-03-02 | 早 4809 天 |"));
        // The report declines to say which kind of backwards it is.
        assert!(md.contains("倒叙是合法手法"));
        assert!(md.contains("| 40% | 第二章 | 相对表达 | 距离 | 三天后 |"));
    }

    #[test]
    fn test_timeline_section_does_not_repeat_a_backstep_as_a_bullet() {
        // The step is already in its own table; a second copy in the notes would only
        // give the same finding two voices with slightly different words.
        let md = render_timeline_with(&dated_timeline());
        assert!(!md.contains("### 时间线提示"));
    }

    #[test]
    fn test_timeline_section_names_dates_it_could_not_read() {
        let mut timeline = empty_timeline();
        timeline.entity_date_count = 19;
        timeline.note = "没有读到可解析的时间表达，而导入数据标了 19 处日期".into();
        let md = render_timeline_with(&timeline);
        assert!(md.contains("本次不做时间排序"));
        assert!(md.contains("导入的实体标注里有 19 处日期"));
    }

    #[test]
    fn test_timeline_section_shows_the_calendar_forms() {
        let mut timeline = dated_timeline();
        timeline.markers.push(TimelineMarker {
            text: "乾隆三年三月初五".into(),
            kind: "absolute".into(),
            precision: "lunar_day".into(),
            section: "第三章".into(),
            position: 0.8,
            resolved: "1738-04-23".into(),
            offset_days: 0.0,
            excerpt: String::new(),
        });
        timeline.absolute_count = 3;
        timeline.forms.push(("lunar_day".into(), 1));
        timeline.issues.push(TimelineIssue {
            kind: "calendar_note".into(),
            section: String::new(),
            detail: "1 处月日按农历换算成公历：年号与干支后面的月日按农历读，换算出自开源历法引擎，改元之年只精确到年".into(),
        });
        let md = render_timeline_with(&timeline);
        assert!(md.contains("### 写法分布"));
        assert!(md.contains("农历日 1 处"));
        assert!(md.contains("| 80% | 第三章 | 1738-04-23 | 农历日 |"));
        assert!(md.contains("- **历法换算**（全文）— 1 处月日按农历换算成公历"));
        // The scope note now names the calendars it reads and where they stop.
        assert!(md.contains("年号纪年（乾隆三年）"));
        assert!(md.contains("lunar-lite（MIT）"));
        assert!(md.contains("干支六十年一轮回"));
    }
}
