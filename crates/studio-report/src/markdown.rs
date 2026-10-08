//! Markdown report rendering.
//!
//! A pure function over the analysis output: the same `T0Stats`/`T1Stats`/
//! `Assessment` plus the same timestamp produce byte-identical Markdown. That is
//! what makes the report testable against a golden string, and it is why the
//! timestamp is an input rather than read from the clock inside.

use chrono::{DateTime, Utc};

use studio_analysis::{Assessment, T0Stats, T1Stats, ANALYSIS_VERSION};

/// Everything a report is rendered from. All borrowed: the renderer never owns an
/// analysis, so a report can be produced straight from cached rows.
pub struct ReportInput<'a> {
    pub project_name: &'a str,
    pub generated_at: DateTime<Utc>,
    pub t0: &'a T0Stats,
    pub t1: &'a T1Stats,
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

#[cfg(test)]
mod tests {
    use super::*;
    use studio_analysis::{
        ArcPoint, DimensionScore, NarrativeArc, Readability, Recommendation, RepeatedPhrase,
        SectionInfo,
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
        let (t0, t1, a) = (empty_t0(), empty_t1(), clean_assessment());
        let input = ReportInput {
            project_name: "测试项目",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            assessment: &a,
        };
        assert_eq!(render_markdown(&input), render_markdown(&input));
    }

    #[test]
    fn test_golden_minimal_report() {
        let (t0, t1, a) = (empty_t0(), empty_t1(), clean_assessment());
        let input = ReportInput {
            project_name: "测试项目",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
            assessment: &a,
        };
        let expected = "\
# 测试项目 · 叙事分析报告

- 生成时间：2026-01-01 08:30 UTC
- 分析版本：m2-v1

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
        let (t0, t1) = (empty_t0(), empty_t1());
        let input = ReportInput {
            project_name: "P",
            generated_at: at(),
            t0: &t0,
            t1: &t1,
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
        ] {
            assert!(md.contains(heading), "missing heading: {heading}");
        }
        for i in 0..4 {
            assert!(
                md.contains(&format!("| 维度{i} | 50.0 |")),
                "missing dimension {i}"
            );
        }
        assert_eq!(md.matches("\n## ").count(), 6, "exactly six H2 sections");
    }

    #[test]
    fn test_recommendations_grouped_and_counted() {
        let (t0, t1) = (empty_t0(), empty_t1());
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
            assessment: &a,
        };
        let md = render_markdown(&input);
        assert!(md.contains("共 2 项（高 1 · 中 1 · 低 0）"));
        assert!(md.contains("- **高 · 可读性偏低**（readability）— 拆分长句"));
        assert!(md.contains("- **中 · 重复短语**（repetition）"));
    }

    #[test]
    fn test_table_cells_are_escaped() {
        let (mut t0, mut t1, a) = (empty_t0(), empty_t1(), clean_assessment());
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
        let (mut t0, mut t1, a) = (empty_t0(), empty_t1(), clean_assessment());
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
        let (t0, mut t1, a) = (empty_t0(), empty_t1(), clean_assessment());
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
            assessment: &a,
        };
        let md = render_markdown(&input);
        assert!(md.contains("叙事弧线形态：**山形**（`mountain`）"));
        assert!(md.contains("| Ch1 | ░░░░░░░░░░ 0.00 | 100 |"));
        assert!(md.contains("| Ch2 | ██████████ 1.00 | 200 |"));
        assert!(md.contains("| Ch3 | █████░░░░░ 0.50 | 150 |"));
    }
}
