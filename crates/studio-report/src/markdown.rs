//! Markdown report rendering.
//!
//! A pure function over the analysis output: the same `T0Stats`/`T1Stats`/
//! `PlotStats`/`Assessment` plus the same timestamp produce byte-identical Markdown.
//! That is what makes the report testable against a golden string, and it is why the
//! timestamp is an input rather than read from the clock inside.

use chrono::{DateTime, Utc};

use studio_analysis::{
    Assessment, ConflictStats, PacingStats, PlotStats, SettingStats, T0Stats, T1Stats, ThemeStats,
    ANALYSIS_VERSION,
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
    write_structure(&mut out, input.t1);
    write_narrative(&mut out, input.t1);
    write_plot(&mut out, input.plot);
    write_pacing(&mut out, input.pacing);
    write_conflict(&mut out, input.conflict);
    write_theme(&mut out, input.theme);
    write_setting(&mut out, input.setting);
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

#[cfg(test)]
mod tests {
    use super::*;
    use studio_analysis::{
        ArcPoint, ConflictAbsence, ConflictPoint, ConflictType, DimensionScore, DragSection,
        FlatStretch, NarrativeArc, PacingPoint, Place, Plotline, PlotlineStage, Readability,
        Recommendation, RepeatedPhrase, Scene, SectionInfo, SettingIssue, Theme, TurningPoint,
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
            assessment: &a,
        };
        assert_eq!(render_markdown(&input), render_markdown(&input));
    }

    #[test]
    fn test_golden_minimal_report() {
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
            assessment: &a,
        };
        let expected = "\
# 测试项目 · 叙事分析报告

- 生成时间：2026-01-01 08:30 UTC
- 分析版本：m2-v3

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

未发现高频重复短语。

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
        let (t0, t1, plot, pacing, conflict, theme, setting) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
        );
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
            assessment: &a,
        };
        let md = render_markdown(&input);
        for heading in [
            "# P · 叙事分析报告",
            "## 总体评估",
            "## 关键建议",
            "## 文本统计",
            "## 文本诊断",
            "## 章节结构",
            "## 叙事分析",
            "## 情节线",
            "## 节奏",
            "## 冲突",
            "## 主题",
            "## 场景/设定",
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
            11,
            "exactly eleven H2 sections"
        );
    }

    #[test]
    fn test_recommendations_grouped_and_counted() {
        let (t0, t1, plot, pacing, conflict, theme, setting) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
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
            assessment: &a,
        };
        let md = render_markdown(&input);
        assert!(md.contains("共 2 项（高 1 · 中 1 · 低 0）"));
        assert!(md.contains("- **高 · 可读性偏低**（readability）— 拆分长句"));
        assert!(md.contains("- **中 · 重复短语**（repetition）"));
    }

    #[test]
    fn test_table_cells_are_escaped() {
        let (mut t0, mut t1, plot, pacing, conflict, theme, setting, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
            clean_assessment(),
        );
        t0.top_entities = vec![("A|B".into(), 3)];
        t1.sections = vec![SectionInfo {
            path: "第一章\n第二节".into(),
            block_count: 2,
            char_count: 100,
        }];
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
        let (mut t0, mut t1, plot, pacing, conflict, theme, setting, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
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
        let (t0, mut t1, plot, pacing, conflict, theme, setting, a) = (
            empty_t0(),
            empty_t1(),
            empty_plot(),
            empty_pacing(),
            empty_conflict(),
            empty_theme(),
            empty_setting(),
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
}
