//! Rule-based assessment: dimension scores and recommendations.
//!
//! Aggregates T0/T1 statistics into 0-100 dimension scores and a prioritized
//! list of actionable recommendations. Purely deterministic — no LLM.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::t0::T0Stats;
use crate::t1::{SectionInfo, T1Stats};

/// Score for a single assessment dimension (0-100, higher = better).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub key: String,
    pub label: String,
    pub score: f64,
    pub detail: String,
}

/// An actionable recommendation produced by the rule engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// "high" | "medium" | "low"
    pub severity: String,
    /// Dimension key the recommendation belongs to.
    pub dimension: String,
    pub title: String,
    pub detail: String,
}

/// Overall assessment for a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    /// Equal-weight mean of dimension scores, 0-100.
    pub overall: f64,
    pub dimensions: Vec<DimensionScore>,
    pub recommendations: Vec<Recommendation>,
}

/// Run the rule-based assessment over T0/T1 statistics.
pub fn run_assessment(t0: &T0Stats, t1: &T1Stats) -> Assessment {
    let mut dimensions = Vec::new();
    let mut recommendations = Vec::new();

    // 1. Readability
    let readability = &t0.readability;
    dimensions.push(DimensionScore {
        key: "readability".into(),
        label: "可读性".into(),
        score: readability.score,
        detail: format!("等级「{}」（{}）", readability.level, readability.method),
    });
    if readability.score < 60.0 {
        recommendations.push(rec(
            "high",
            "readability",
            "可读性偏低",
            "平均句长偏长，建议拆分长句、减少从句嵌套以提升可读性",
        ));
    }

    // 2. Sentence rhythm
    let total_sentences = t0
        .sentence_length_distribution
        .iter()
        .map(|(_, c)| *c)
        .sum::<u32>();
    let long_sentences = t0
        .sentence_length_distribution
        .iter()
        .find(|(label, _)| label == ">100")
        .map(|(_, c)| *c)
        .unwrap_or(0);
    let long_ratio = if total_sentences > 0 {
        long_sentences as f64 / total_sentences as f64
    } else {
        0.0
    };
    let rhythm_score = (100.0 - long_ratio * 400.0).clamp(0.0, 100.0);
    dimensions.push(DimensionScore {
        key: "sentence_rhythm".into(),
        label: "句子节奏".into(),
        score: rhythm_score,
        detail: format!("超长句(>100字)占比 {:.1}%", long_ratio * 100.0),
    });
    if long_ratio > 0.15 {
        recommendations.push(rec(
            "high",
            "sentence_rhythm",
            "超长句过多",
            format!("{long_sentences} 个句子超过 100 字，建议拆分为多个短句"),
        ));
    } else if long_ratio > 0.05 {
        recommendations.push(rec(
            "medium",
            "sentence_rhythm",
            "长句偏多",
            format!("{long_sentences} 个句子超过 100 字，可适当拆分"),
        ));
    }

    // 3. Word choice (adverb usage)
    let adverb_density = if t0.word_count > 0 {
        t0.adverb_count as f64 / t0.word_count as f64
    } else {
        0.0
    };
    let top_adverb = t0.top_adverbs.first();
    let overuse_penalty = top_adverb
        .map(|(_, c)| (*c as f64 / 20.0).min(20.0))
        .unwrap_or(0.0);
    let density_penalty = (adverb_density - 0.04).max(0.0) * 500.0;
    let word_score = (100.0 - density_penalty - overuse_penalty).clamp(0.0, 100.0);
    dimensions.push(DimensionScore {
        key: "word_choice".into(),
        label: "用词".into(),
        score: word_score,
        detail: format!("副词密度 {:.1}%", adverb_density * 100.0),
    });
    if let Some((word, count)) = top_adverb {
        if *count > 100 {
            recommendations.push(rec(
                "medium",
                "word_choice",
                format!("副词「{word}」高频使用"),
                format!("出现 {count} 次，建议用更具体的动词或细节描写替代"),
            ));
        }
    }
    if adverb_density > 0.06 {
        recommendations.push(rec(
            "low",
            "word_choice",
            "副词使用偏多",
            format!(
                "副词占词数 {:.1}%，建议精简修饰、优先使用精确动词",
                adverb_density * 100.0
            ),
        ));
    }

    // 4. Repetition
    let repeated_total: u32 = t0.top_repeated_phrases.iter().map(|p| p.count).sum();
    let repeat_density = if t0.word_count > 0 {
        repeated_total as f64 / t0.word_count as f64
    } else {
        0.0
    };
    let repeat_score = (100.0 - (repeat_density - 0.02).max(0.0) * 1000.0).clamp(0.0, 100.0);
    dimensions.push(DimensionScore {
        key: "repetition".into(),
        label: "重复".into(),
        score: repeat_score,
        detail: format!(
            "高频重复短语 {} 个，共出现 {} 次",
            t0.top_repeated_phrases.len(),
            repeated_total
        ),
    });
    if let Some(phrase) = t0.top_repeated_phrases.first() {
        if phrase.count > 30 {
            recommendations.push(rec(
                "medium",
                "repetition",
                format!("短语「{}」重复 {} 次", phrase.text, phrase.count),
                "建议合并重复表述或改用指代",
            ));
        }
    }

    // 5. Structure balance
    let (balance_score, balance_detail, top_section) = section_balance(&t1.sections);
    dimensions.push(DimensionScore {
        key: "structure_balance".into(),
        label: "结构均衡".into(),
        score: balance_score,
        detail: balance_detail,
    });
    if let Some((name, ratio)) = top_section {
        if ratio > 3.0 {
            recommendations.push(rec(
                "medium",
                "structure_balance",
                format!("章节「{name}」体量显著偏大"),
                format!("为中位章节字数的 {ratio:.1} 倍，建议拆分或补充其他章节"),
            ));
        }
    }

    let overall = dimensions.iter().map(|d| d.score).sum::<f64>() / dimensions.len().max(1) as f64;

    recommendations.sort_by_key(|a| severity_rank(&a.severity));

    Assessment {
        overall,
        dimensions,
        recommendations,
    }
}

/// Section size balance: penalize a max/median ratio well above 1.5.
///
/// Sections are aggregated to their top-level component first, so one large
/// sub-section is not compared against a median of many small ones.
/// Returns (score, detail, Option<(largest section name, ratio)>).
fn section_balance(sections: &[SectionInfo]) -> (f64, String, Option<(String, f64)>) {
    let mut totals: HashMap<String, u64> = HashMap::new();
    for s in sections {
        let key = s.path.split(" / ").next().unwrap_or(s.path.as_str());
        *totals.entry(key.to_string()).or_insert(0) += s.char_count;
    }
    if totals.len() < 2 {
        return (100.0, "章节数不足，跳过均衡性评估".into(), None);
    }
    let sizes: Vec<(String, u64)> = totals.into_iter().collect();
    let mut sorted: Vec<u64> = sizes.iter().map(|(_, c)| *c).collect();
    sorted.sort_unstable();
    // Lower-middle element: a stable "typical section" size for any length.
    let median = sorted[(sorted.len() - 1) / 2].max(1) as f64;
    let (name, max) = sizes.iter().max_by_key(|(_, c)| *c).expect("non-empty");
    let ratio = *max as f64 / median;
    let score = (100.0 - (ratio - 1.5).max(0.0) * 20.0).clamp(0.0, 100.0);
    (
        score,
        format!("最大章节为中位数的 {ratio:.1} 倍"),
        Some((name.clone(), ratio)),
    )
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "high" => 0,
        "medium" => 1,
        _ => 2,
    }
}

fn rec(
    severity: &str,
    dimension: &str,
    title: impl Into<String>,
    detail: impl Into<String>,
) -> Recommendation {
    Recommendation {
        severity: severity.into(),
        dimension: dimension.into(),
        title: title.into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t0::{Readability, RepeatedPhrase};
    use crate::t1::{ArcPoint, NarrativeArc};

    fn base_t0() -> T0Stats {
        T0Stats {
            word_count: 1000,
            char_count: 900,
            block_count: 10,
            top_words: vec![],
            pos_distribution: vec![],
            entity_counts: vec![],
            top_entities: vec![],
            top_noun_signals: vec![],
            noun_signal_count: 0,
            avg_block_length: 90.0,
            avg_sentence_length: 10.0,
            sentence_count: 50,
            avg_sentence_chars: 18.0,
            sentence_length_distribution: vec![
                ("≤20".into(), 40),
                ("21-50".into(), 8),
                ("51-100".into(), 2),
                (">100".into(), 0),
            ],
            adverb_count: 20,
            adjective_count: 30,
            top_adverbs: vec![("非常".into(), 10)],
            top_adjectives: vec![],
            top_repeated_phrases: vec![],
            readability: Readability {
                score: 90.0,
                level: "易读".into(),
                method: "cjk-sentence-length".into(),
            },
        }
    }

    fn base_t1() -> T1Stats {
        T1Stats {
            sections: vec![
                SectionInfo {
                    path: "Ch1".into(),
                    block_count: 5,
                    char_count: 400,
                },
                SectionInfo {
                    path: "Ch2".into(),
                    block_count: 5,
                    char_count: 500,
                },
            ],
            section_count: 2,
            title_blocks: 2,
            paragraph_blocks: 8,
            list_blocks: 0,
            block_type_distribution: vec![],
            longest_section: Some("Ch2".into()),
            shortest_section: Some("Ch1".into()),
            characters: vec![],
            arc: NarrativeArc {
                points: vec![ArcPoint {
                    section: "Ch1".into(),
                    intensity: 0.5,
                    char_count: 400,
                }],
                shape: "steady".into(),
            },
            entity_cooccurrences: vec![],
        }
    }

    #[test]
    fn test_assessment_clean_doc() {
        let a = run_assessment(&base_t0(), &base_t1());
        assert_eq!(a.dimensions.len(), 5);
        assert!((0.0..=100.0).contains(&a.overall));
        // No rule should fire on this clean document.
        assert!(a.recommendations.is_empty());
    }

    #[test]
    fn test_assessment_flags_long_sentences() {
        let mut t0 = base_t0();
        t0.sentence_length_distribution = vec![
            ("≤20".into(), 10),
            ("21-50".into(), 10),
            ("51-100".into(), 10),
            (">100".into(), 20),
        ];
        let a = run_assessment(&t0, &base_t1());
        assert!(a
            .recommendations
            .iter()
            .any(|r| r.severity == "high" && r.dimension == "sentence_rhythm"));
    }

    #[test]
    fn test_assessment_flags_adverb_overuse() {
        let mut t0 = base_t0();
        t0.top_adverbs = vec![("非常".into(), 150)];
        let a = run_assessment(&t0, &base_t1());
        assert!(a
            .recommendations
            .iter()
            .any(|r| r.dimension == "word_choice" && r.title.contains("非常")));
    }

    #[test]
    fn test_assessment_flags_repetition() {
        let mut t0 = base_t0();
        t0.top_repeated_phrases = vec![RepeatedPhrase {
            text: "出版单位".into(),
            count: 50,
        }];
        let a = run_assessment(&t0, &base_t1());
        assert!(a
            .recommendations
            .iter()
            .any(|r| r.dimension == "repetition" && r.title.contains("出版单位")));
    }

    #[test]
    fn test_assessment_flags_unbalanced_sections() {
        let mut t1 = base_t1();
        t1.sections[0].char_count = 2000; // 4x the median of 500
        let a = run_assessment(&base_t0(), &t1);
        assert!(a
            .recommendations
            .iter()
            .any(|r| r.dimension == "structure_balance"));
    }

    #[test]
    fn test_assessment_recommendation_order() {
        let mut t0 = base_t0();
        t0.readability = Readability {
            score: 30.0,
            level: "困难".into(),
            method: "cjk-sentence-length".into(),
        };
        t0.sentence_length_distribution = vec![
            ("≤20".into(), 1),
            ("21-50".into(), 1),
            ("51-100".into(), 1),
            (">100".into(), 10),
        ];
        t0.top_adverbs = vec![("很".into(), 200)];
        let a = run_assessment(&t0, &base_t1());
        assert!(!a.recommendations.is_empty());
        // high-severity items must come before medium/low.
        let ranks: Vec<u8> = a
            .recommendations
            .iter()
            .map(|r| severity_rank(&r.severity))
            .collect();
        let mut sorted = ranks.clone();
        sorted.sort_unstable();
        assert_eq!(ranks, sorted);
    }
}
