//! Author voice: the habits that stay put whatever the story happens to be about.
//!
//! A voice is not a grade. Long sentences are not worse than short ones, a hedging
//! narrator is not worse than an assertive one, so this dimension carries **no
//! score at all** — it describes how the text is written and leaves the reading to
//! the author. What it measures:
//!
//! - 句子节奏: how long the sentences run and how much they vary
//! - 小句节奏: how much a sentence carries before it lands
//! - 段落节奏: whether the text breathes in long blocks or short ones
//! - 标点习惯: which marks the text reaches for, per thousand characters
//! - 功能词: 的地得 / 了着过 / 与而及之 — the words that carry no plot
//! - 人称与视角: how often the text says 我 / 你 / 他
//! - 犹疑与断言: 或许、好像 against 一定、显然
//!
//! Two limits are stated where they bite rather than hidden: function words and
//! pronouns are counted **by character**, so 了解 contributes a 了 and 其他 a 他
//! (a 他 following 其/别/单 is skipped); and every rate is per thousand visible
//! characters, so a page carrying a long quoted song lyric reads as more
//! punctuated than the prose is. Both are habits measured loosely on purpose — a
//! fingerprint, not a parse.
//!
//! Deterministic: lists are scanned in fixed order, the curve uses
//! [`plan_segments`] so it lines up with the pacing, conflict, theme, setting and
//! dialogue curves, and nothing here reads a HashMap.

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;

use crate::segment::plan_segments;
use crate::text::{split_sentences, visible_char_count};
use crate::Result;

/// Below this many visible characters the habits say nothing yet: one page can
/// hold any style, and the numbers have not had room to settle.
const MIN_CHARS_FOR_FINDINGS: u64 = 600;
/// How many findings are reported, in the order they are checked.
const MAX_ISSUES: usize = 10;

/// Punctuation marks counted, in this fixed order. Counted by character, so one
/// …… counts twice and one —— counts twice — the density of ink on the page.
const MARKS: &[(char, &str)] = &[
    ('，', "逗号"),
    ('。', "句号"),
    ('、', "顿号"),
    ('；', "分号"),
    ('：', "冒号"),
    ('？', "问号"),
    ('！', "叹号"),
    ('…', "省略号"),
    ('—', "破折号"),
    ('“', "前引号"),
    ('”', "后引号"),
    ('「', "日式引号"),
    ('（', "括号"),
    ('《', "书名号"),
];

/// Function words grouped, in this fixed order. Characters, not tokens.
const FUNCTION_GROUPS: &[(&str, &[char])] = &[
    ("结构助词", &['的', '地', '得']),
    ("动态助词", &['了', '着', '过']),
    ("并列与转折", &['和', '与', '及', '而', '但', '却']),
    (
        "副词",
        &[
            '很', '太', '更', '最', '就', '都', '也', '还', '又', '才', '只',
        ],
    ),
    ("把与被", &['把', '被']),
    ("文言虚词", &['之', '其', '者', '乃', '则', '亦', '乎']),
];

/// Pronouns by person. 我们 is counted through its 我.
const FIRST_PERSON: &[char] = &['我', '咱', '俺'];
const SECOND_PERSON: &[char] = &['你', '您'];
const THIRD_PERSON: &[char] = &['他', '她', '它'];
/// A 他/她/它 following one of these is part of a word (其他, 别的), not a person.
const PRONOUN_BLOCK_BEFORE: &[char] = &['其', '别', '单'];

/// Words that hold a statement back, and words that land it.
const HEDGING: &[&str] = &[
    "似乎", "好像", "仿佛", "或许", "也许", "大概", "可能", "想来", "据说", "听说", "怕是", "有点",
];
const ASSERTIVE: &[&str] = &[
    "一定", "必然", "肯定", "显然", "确实", "无疑", "正是", "根本", "完全", "绝对", "向来", "从来",
];

/// Sentence-length thresholds that make a rhythm worth naming.
const LONG_SENTENCE_AVG: f64 = 45.0;
const SHORT_SENTENCE_AVG: f64 = 14.0;
/// Coefficient of variation either side of which the rhythm has a name.
const VARIED_CV: f64 = 0.55;
const EVEN_CV: f64 = 0.30;
/// Clauses per sentence above which the sentences are running on commas.
const COMMA_RUN: f64 = 3.5;
/// Average paragraph length under which the text is cut fine.
const SHORT_PARAGRAPH_AVG: f64 = 90.0;
/// Question plus exclamation marks per thousand characters under which the text
/// never raises its voice.
const FLAT_VOICE_RATE: f64 = 0.4;
/// Sentence count under which the rhythm has no shape to describe yet.
const MIN_SENTENCES_FOR_RHYTHM: u32 = 20;
/// Share of paragraphs that are one sentence before it reads as a habit.
const SNIPPET_PARAGRAPH_SHARE: f64 = 0.4;
/// Pronoun occurrences under which the text is said to avoid them.
const MIN_PRONOUNS: u32 = 10;
/// First-person share that makes the telling first person.
const FIRST_PERSON_SHARE: f64 = 0.6;
/// Per ten thousand characters at which hedging or assertion is a habit.
const STANCE_RATE: f64 = 6.0;
/// Per thousand characters at which the classical particles are a style.
const CLASSICAL_RATE: f64 = 3.0;
/// One segment of the rhythm curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicePoint {
    pub section: String,
    pub char_count: u64,
    pub sentences: u32,
    pub avg_sentence_chars: f64,
    pub clauses_per_sentence: f64,
}

/// How often one mark appears.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceMark {
    pub mark: String,
    pub label: String,
    pub count: u32,
    pub per_1000: f64,
}

/// One group of function words.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceWordGroup {
    pub group: String,
    pub words: String,
    pub count: u32,
    pub per_1000: f64,
}

/// How often the text names who is talking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoicePronouns {
    pub first: u32,
    pub second: u32,
    pub third: u32,
    pub total: u32,
    pub first_share: f64,
    pub second_share: f64,
    pub third_share: f64,
}

/// How firmly the text stands behind what it says.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoiceStance {
    pub hedging: u32,
    pub assertive: u32,
    pub hedging_per_10k: f64,
    pub assertive_per_10k: f64,
    /// `hedging` | `balanced` | `assertive`.
    pub verdict: String,
}

/// A habit worth naming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceIssue {
    /// `long_sentences` | `short_sentences` | `varied_rhythm` | `even_rhythm` |
    /// `comma_run` | `short_paragraphs` | `snippet_paragraphs` | `no_pronouns` |
    /// `first_person` | `hedging` | `assertive` | `classical` | `flat_marks` |
    /// `too_short`.
    pub kind: String,
    pub detail: String,
}

/// Author-voice analysis output for one document. Carries no score by design.
/// The `Default` is an empty document's voice — nothing measured, nothing found —
/// which is what a report renders when a project was analysed before this
/// dimension existed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoiceStats {
    pub points: Vec<VoicePoint>,
    pub segment_count: u32,
    /// True when the segments are block chunks rather than sections.
    pub chunked: bool,
    pub char_count: u64,
    pub sentence_count: u32,
    pub avg_sentence_chars: f64,
    pub sentence_stdev: f64,
    /// `sentence_stdev / avg_sentence_chars` — the rhythm's spread, scale-free.
    pub sentence_cv: f64,
    pub shortest_sentence: u32,
    pub longest_sentence: u32,
    pub clause_count: u32,
    pub avg_clause_chars: f64,
    pub clauses_per_sentence: f64,
    pub paragraph_count: u32,
    pub avg_paragraph_chars: f64,
    pub paragraph_stdev: f64,
    pub single_sentence_paragraphs: u32,
    pub marks: Vec<VoiceMark>,
    pub function_words: Vec<VoiceWordGroup>,
    pub pronouns: VoicePronouns,
    pub stance: VoiceStance,
    pub issues: Vec<VoiceIssue>,
}

/// Characters that cut a sentence into clauses.
const CLAUSE_BREAKS: &[char] = &['，', '、', '；', '：', ','];

/// The sentences of one run of text, each with its length and clause count.
fn sentences_of(text: &str) -> Vec<Sentence> {
    split_sentences(text)
        .iter()
        .map(|s| Sentence {
            chars: visible_char_count(s) as u32,
            clauses: s.chars().filter(|c| CLAUSE_BREAKS.contains(c)).count() as u32 + 1,
        })
        .filter(|s| s.chars > 0)
        .collect()
}

/// Population standard deviation; one sentence has no spread to speak of.
fn stdev(values: &[u32], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let n = values.len() as f64;
    let variance = values
        .iter()
        .map(|v| (*v as f64 - mean).powi(2))
        .sum::<f64>()
        / n;
    variance.sqrt()
}

/// A count expressed per `per` characters, which is how every rate here is said.
fn rate(count: u32, chars: u64, per: f64) -> f64 {
    if chars == 0 {
        0.0
    } else {
        count as f64 / chars as f64 * per
    }
}

fn count_marks(text: &str, chars: u64) -> Vec<VoiceMark> {
    MARKS
        .iter()
        .map(|(mark, label)| {
            let count = text.chars().filter(|c| c == mark).count() as u32;
            VoiceMark {
                mark: mark.to_string(),
                label: (*label).to_string(),
                count,
                per_1000: round(rate(count, chars, 1000.0), 2),
            }
        })
        .collect()
}

/// Round to fixed decimals, so a stored rate reads the same tomorrow.
fn round(value: f64, digits: u32) -> f64 {
    let factor = 10f64.powi(digits as i32);
    (value * factor).round() / factor
}

fn count_function_words(text: &str, chars: u64) -> Vec<VoiceWordGroup> {
    FUNCTION_GROUPS
        .iter()
        .map(|(group, words)| {
            let count = text.chars().filter(|c| words.contains(c)).count() as u32;
            VoiceWordGroup {
                group: (*group).to_string(),
                words: words.iter().collect(),
                count,
                per_1000: round(rate(count, chars, 1000.0), 2),
            }
        })
        .collect()
}

/// Count pronouns by character, skipping the 他 in 其他 and the 它 in 其它.
fn count_pronouns(chars: &[char]) -> VoicePronouns {
    let mut counts = [0u32; 3];
    for (idx, c) in chars.iter().enumerate() {
        let blocked = idx
            .checked_sub(1)
            .is_some_and(|p| PRONOUN_BLOCK_BEFORE.contains(&chars[p]));
        if blocked {
            continue;
        }
        for (person, set) in [FIRST_PERSON, SECOND_PERSON, THIRD_PERSON]
            .iter()
            .enumerate()
        {
            if set.contains(c) {
                counts[person] += 1;
                break;
            }
        }
    }
    let total = counts.iter().sum::<u32>();
    let share = |n: u32| {
        if total == 0 {
            0.0
        } else {
            n as f64 / total as f64
        }
    };
    VoicePronouns {
        first: counts[0],
        second: counts[1],
        third: counts[2],
        total,
        first_share: round(share(counts[0]), 3),
        second_share: round(share(counts[1]), 3),
        third_share: round(share(counts[2]), 3),
    }
}

fn count_words(text: &str, list: &[&str]) -> u32 {
    list.iter()
        .map(|word| text.matches(*word).count() as u32)
        .sum()
}

fn stance_of(hedging: u32, assertive: u32, chars: u64) -> VoiceStance {
    let hedging_per_10k = round(rate(hedging, chars, 10_000.0), 1);
    let assertive_per_10k = round(rate(assertive, chars, 10_000.0), 1);
    let verdict = if hedging_per_10k >= STANCE_RATE && hedging_per_10k > assertive_per_10k * 1.5 {
        "hedging"
    } else if assertive_per_10k >= STANCE_RATE && assertive_per_10k > hedging_per_10k * 1.5 {
        "assertive"
    } else {
        "balanced"
    };
    VoiceStance {
        hedging,
        assertive,
        hedging_per_10k,
        assertive_per_10k,
        verdict: verdict.to_string(),
    }
}

/// What one curve segment accumulated.
#[derive(Default)]
struct Bucket {
    label: String,
    chars: u64,
    sentences: u32,
    sentence_chars: u32,
    clauses: u32,
}

/// Measure how the document is written.
pub fn run_voice_analysis(doc: &DocumentData) -> Result<VoiceStats> {
    let plan = plan_segments(doc);

    // The paragraph is the block: TraceView cut the text there, and a paragraph
    // habit is a habit about those cuts rather than about the newline.
    let per_block: Vec<Vec<Sentence>> = doc
        .blocks
        .iter()
        .map(|b| sentences_of(&b.content))
        .collect();
    let paragraph_lens: Vec<u32> = doc
        .blocks
        .iter()
        .map(|b| visible_char_count(&b.content) as u32)
        .collect();

    let text: String = doc
        .blocks
        .iter()
        .map(|b| b.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let chars: Vec<char> = text.chars().collect();
    let char_count = visible_char_count(&text) as u64;

    let mut buckets: Vec<Bucket> = plan
        .labels
        .iter()
        .map(|label| Bucket {
            label: label.clone(),
            ..Default::default()
        })
        .collect();
    for (idx, sentences) in per_block.iter().enumerate() {
        let bucket = &mut buckets[plan.segment_of[idx]];
        bucket.chars += visible_char_count(&doc.blocks[idx].content) as u64;
        for s in sentences {
            bucket.sentences += 1;
            bucket.sentence_chars += s.chars;
            bucket.clauses += s.clauses;
        }
    }
    let points: Vec<VoicePoint> = buckets
        .iter()
        .map(|b| VoicePoint {
            section: b.label.clone(),
            char_count: b.chars,
            sentences: b.sentences,
            avg_sentence_chars: share(b.sentence_chars, b.sentences, 1),
            clauses_per_sentence: share(b.clauses, b.sentences, 2),
        })
        .collect();

    let lengths: Vec<u32> = per_block.iter().flatten().map(|s| s.chars).collect();
    let clause_counts: Vec<u32> = per_block.iter().flatten().map(|s| s.clauses).collect();
    let sentence_count = lengths.len() as u32;
    let sentence_chars: u32 = lengths.iter().sum();
    let clause_count: u32 = clause_counts.iter().sum();
    let avg_sentence_chars = share(sentence_chars, sentence_count, 1);
    let sentence_stdev = round(stdev(&lengths, avg_sentence_chars), 1);
    let sentence_cv = if avg_sentence_chars == 0.0 {
        0.0
    } else {
        round(sentence_stdev / avg_sentence_chars, 2)
    };
    let avg_clause_chars = share(sentence_chars, clause_count, 1);
    let clauses_per_sentence = share(clause_count, sentence_count, 2);

    let paragraph_count = paragraph_lens.len() as u32;
    let avg_paragraph_chars = share(paragraph_lens.iter().sum(), paragraph_count, 1);
    let paragraph_stdev = round(stdev(&paragraph_lens, avg_paragraph_chars), 1);
    let single_sentence_paragraphs = per_block.iter().filter(|s| s.len() == 1).count() as u32;

    let hedging = count_words(&text, HEDGING);
    let assertive = count_words(&text, ASSERTIVE);

    let mut stats = VoiceStats {
        points,
        segment_count: plan.labels.len() as u32,
        chunked: plan.chunked,
        char_count,
        sentence_count,
        avg_sentence_chars,
        sentence_stdev,
        sentence_cv,
        shortest_sentence: lengths.iter().min().copied().unwrap_or(0),
        longest_sentence: lengths.iter().max().copied().unwrap_or(0),
        clause_count,
        avg_clause_chars,
        clauses_per_sentence,
        paragraph_count,
        avg_paragraph_chars,
        paragraph_stdev,
        single_sentence_paragraphs,
        marks: count_marks(&text, char_count),
        function_words: count_function_words(&text, char_count),
        pronouns: count_pronouns(&chars),
        stance: stance_of(hedging, assertive, char_count),
        issues: Vec::new(),
    };
    stats.issues = findings(&stats);
    Ok(stats)
}

/// A sum over a count, said to `digits` decimals; nothing over nothing is 0.
fn share(sum: u32, count: u32, digits: u32) -> f64 {
    if count == 0 {
        0.0
    } else {
        round(sum as f64 / count as f64, digits)
    }
}

/// How often a mark comes per thousand characters, 0 when it is not in the list.
fn mark_per_1000(stats: &VoiceStats, label: &str) -> f64 {
    stats
        .marks
        .iter()
        .find(|m| m.label == label)
        .map(|m| m.per_1000)
        .unwrap_or(0.0)
}

/// How often one function-word group comes per thousand characters.
fn group_per_1000(stats: &VoiceStats, group: &str) -> f64 {
    stats
        .function_words
        .iter()
        .find(|g| g.group == group)
        .map(|g| g.per_1000)
        .unwrap_or(0.0)
}

fn push(issues: &mut Vec<VoiceIssue>, kind: &str, detail: String) {
    if issues.len() < MAX_ISSUES {
        issues.push(VoiceIssue {
            kind: kind.to_string(),
            detail,
        });
    }
}

/// Name the habits a reader would notice, in the order they are checked. None of
/// these is a fault: they say what the writing tends to do.
fn findings(stats: &VoiceStats) -> Vec<VoiceIssue> {
    let mut issues: Vec<VoiceIssue> = Vec::new();

    if stats.char_count < MIN_CHARS_FOR_FINDINGS {
        push(
            &mut issues,
            "too_short",
            format!(
                "全文 {} 字，不足 {} 字：下面的数字是这一次统计的结果，还不足以说明写法。",
                stats.char_count, MIN_CHARS_FOR_FINDINGS
            ),
        );
        return issues;
    }

    if stats.sentence_count >= MIN_SENTENCES_FOR_RHYTHM {
        if stats.avg_sentence_chars >= LONG_SENTENCE_AVG {
            push(
                &mut issues,
                "long_sentences",
                format!(
                    "平均一句 {} 字，句子偏长（最短 {} 字，最长 {} 字）。",
                    stats.avg_sentence_chars, stats.shortest_sentence, stats.longest_sentence
                ),
            );
        } else if stats.avg_sentence_chars <= SHORT_SENTENCE_AVG {
            push(
                &mut issues,
                "short_sentences",
                format!(
                    "平均一句 {} 字，句子偏短（最短 {} 字，最长 {} 字）。",
                    stats.avg_sentence_chars, stats.shortest_sentence, stats.longest_sentence
                ),
            );
        }
        if stats.sentence_cv >= VARIED_CV {
            push(
                &mut issues,
                "varied_rhythm",
                format!(
                    "句长差异大（标准差 {}，变异系数 {}）：长句和短句交替出现。",
                    stats.sentence_stdev, stats.sentence_cv
                ),
            );
        } else if stats.sentence_cv <= EVEN_CV {
            push(
                &mut issues,
                "even_rhythm",
                format!(
                    "句长整齐（标准差 {}，变异系数 {}）：句子普遍差不多长。",
                    stats.sentence_stdev, stats.sentence_cv
                ),
            );
        }
    }

    if stats.clauses_per_sentence >= COMMA_RUN {
        push(
            &mut issues,
            "comma_run",
            format!(
                "平均每句 {} 个小句：句子靠逗号一段一段往下推。",
                stats.clauses_per_sentence
            ),
        );
    }

    if stats.paragraph_count >= 5 {
        if stats.avg_paragraph_chars <= SHORT_PARAGRAPH_AVG {
            push(
                &mut issues,
                "short_paragraphs",
                format!(
                    "平均一段 {} 字（共 {} 段）：段落切得短。",
                    stats.avg_paragraph_chars, stats.paragraph_count
                ),
            );
        }
        let snippets = stats.single_sentence_paragraphs;
        let share = if stats.paragraph_count == 0 {
            0.0
        } else {
            round(snippets as f64 / stats.paragraph_count as f64, 2)
        };
        if share >= SNIPPET_PARAGRAPH_SHARE {
            push(
                &mut issues,
                "snippet_paragraphs",
                format!(
                    "{} / {} 段只有一句话（{}）：成段常常就是一句。",
                    snippets,
                    stats.paragraph_count,
                    (share * 100.0).round()
                ),
            );
        }
    }

    if stats.pronouns.total < MIN_PRONOUNS {
        push(
            &mut issues,
            "no_pronouns",
            format!(
                "全文只出现 {} 次人称代词：叙述很少直接说谁在看、谁在说。",
                stats.pronouns.total
            ),
        );
    } else if stats.pronouns.first_share >= FIRST_PERSON_SHARE {
        push(
            &mut issues,
            "first_person",
            format!(
                "人称代词里 {:.0}% 是第一人称（{} 次）：叙述多从「我」的位置看。",
                stats.pronouns.first_share * 100.0,
                stats.pronouns.first
            ),
        );
    }

    match stats.stance.verdict.as_str() {
        "hedging" => push(
            &mut issues,
            "hedging",
            format!(
                "犹疑用词 {}/万字（{} 处），断言用词 {}/万字：说法多半留着余地。",
                stats.stance.hedging_per_10k, stats.stance.hedging, stats.stance.assertive_per_10k
            ),
        ),
        "assertive" => push(
            &mut issues,
            "assertive",
            format!(
                "断言用词 {}/万字（{} 处），犹疑用词 {}/万字：说法多半落得很实。",
                stats.stance.assertive_per_10k,
                stats.stance.assertive,
                stats.stance.hedging_per_10k
            ),
        ),
        _ => {}
    }

    let classical = group_per_1000(stats, "文言虚词");
    if classical >= CLASSICAL_RATE {
        push(
            &mut issues,
            "classical",
            format!(
                "之、其、者、乃、则、亦、乎出现 {}/千字：句子带着文言的骨架。",
                classical
            ),
        );
    }

    let raised = mark_per_1000(stats, "问号") + mark_per_1000(stats, "叹号");
    if raised < FLAT_VOICE_RATE {
        push(
            &mut issues,
            "flat_marks",
            format!("问号与叹号合计 {}/千字：文本几乎不抬高声音。", raised),
        );
    }

    issues
}

/// What one sentence contributes: its length and how many clauses it was cut into.
struct Sentence {
    chars: u32,
    clauses: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

    fn block(content: &str) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: content.into(),
            section_path: "第一章".into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
        }
    }

    fn doc_of(blocks: &[&str]) -> DocumentData {
        DocumentData {
            doc_id: uuid::Uuid::nil(),
            title: "Test".into(),
            blocks: blocks.iter().map(|c| block(c)).collect(),
            total_word_count: 0,
            total_char_count: 0,
        }
    }

    /// One sentence of `len` characters, cut by a comma every seventh.
    fn sentence(len: usize, seed: char) -> String {
        let body: String = (0..len)
            .map(|i| if i % 7 == 6 { '，' } else { seed })
            .collect();
        format!("{body}。")
    }

    fn paragraphs(count: usize, len: usize, seed: char) -> Vec<String> {
        (0..count).map(|_| sentence(len, seed)).collect()
    }

    fn owned(parts: &[String]) -> Vec<&str> {
        parts.iter().map(|s| s.as_str()).collect()
    }

    fn kinds(stats: &VoiceStats) -> Vec<&str> {
        stats.issues.iter().map(|i| i.kind.as_str()).collect()
    }

    #[test]
    fn test_a_voice_carries_no_score() {
        let stats = run_voice_analysis(&doc_of(&["他走了。她留下了。"])).unwrap();
        let json = serde_json::to_string(&stats).unwrap();
        assert!(
            !json.contains("score"),
            "a voice is a fingerprint, not a grade: {json}"
        );
    }

    #[test]
    fn test_alternating_lengths_read_as_a_varied_rhythm() {
        let long = sentence(70, '长');
        let short = sentence(8, '短');
        let parts: Vec<String> = (0..24)
            .map(|i| {
                if i % 2 == 0 {
                    long.clone()
                } else {
                    short.clone()
                }
            })
            .collect();
        let stats = run_voice_analysis(&doc_of(&owned(&parts))).unwrap();
        assert_eq!(stats.sentence_count, 24);
        assert!(
            stats.sentence_cv >= VARIED_CV,
            "70 and 8 alternating should vary: {}",
            stats.sentence_cv
        );
        assert!(kinds(&stats).contains(&"varied_rhythm"));
    }

    #[test]
    fn test_alike_sentences_read_as_an_even_rhythm() {
        let parts = paragraphs(24, 30, '平');
        let stats = run_voice_analysis(&doc_of(&owned(&parts))).unwrap();
        assert_eq!(stats.sentence_stdev, 0.0);
        assert!(stats.sentence_cv <= EVEN_CV);
        assert!(kinds(&stats).contains(&"even_rhythm"));
    }

    #[test]
    fn test_clauses_are_counted_between_the_commas() {
        let stats = run_voice_analysis(&doc_of(&["他走了，她留下，门开着，灯亮着。"])).unwrap();
        assert_eq!(stats.sentence_count, 1);
        assert_eq!(stats.clause_count, 4);
        assert_eq!(stats.clauses_per_sentence, 4.0);
        // Fifteen visible characters over four clauses.
        assert_eq!(stats.avg_clause_chars, 3.8);
    }

    #[test]
    fn test_a_pronoun_inside_a_word_is_not_a_person() {
        let parts = paragraphs(12, 30, '字');
        let mut lines = owned(&parts);
        lines.push("其他人都走了。他也来了。其它的它不算，其他也不算。");
        let stats = run_voice_analysis(&doc_of(&lines)).unwrap();
        // One 他 and one 它; the two after 其 and 别 belong to a word.
        assert_eq!(stats.pronouns.third, 2);
        assert_eq!(stats.pronouns.first, 0);
    }

    #[test]
    fn test_first_person_telling_is_named() {
        let mut parts = paragraphs(16, 40, '我');
        parts.push(sentence(30, '他'));
        parts.push(sentence(30, '他'));
        let stats = run_voice_analysis(&doc_of(&owned(&parts))).unwrap();
        assert!(stats.pronouns.first_share >= FIRST_PERSON_SHARE);
        assert!(kinds(&stats).contains(&"first_person"));
    }

    #[test]
    fn test_every_mark_is_reported_even_when_absent() {
        let parts = paragraphs(6, 60, '字');
        let stats = run_voice_analysis(&doc_of(&owned(&parts))).unwrap();
        assert_eq!(stats.marks.len(), MARKS.len());
        let comma = stats.marks.iter().find(|m| m.label == "逗号").unwrap();
        assert!(comma.count > 0 && comma.per_1000 > 0.0);
        assert!(stats
            .marks
            .iter()
            .any(|m| m.label == "书名号" && m.count == 0));
    }

    #[test]
    fn test_function_words_come_grouped_in_order() {
        let parts = paragraphs(6, 60, '的');
        let stats = run_voice_analysis(&doc_of(&owned(&parts))).unwrap();
        assert_eq!(stats.function_words.len(), FUNCTION_GROUPS.len());
        assert_eq!(stats.function_words[0].group, "结构助词");
        assert_eq!(stats.function_words[0].words, "的地得");
        assert!(stats.function_words[0].count > 0);
    }

    #[test]
    fn test_hedging_is_a_stance_and_not_a_fault() {
        let mut parts = paragraphs(12, 60, '字');
        parts.push("或许是他来过。好像门没有关。".to_string());
        parts.push("大概他记得。也许全忘了。".to_string());
        let stats = run_voice_analysis(&doc_of(&owned(&parts))).unwrap();
        assert_eq!(stats.stance.verdict, "hedging");
        assert_eq!(stats.stance.hedging, 4);
        assert!(kinds(&stats).contains(&"hedging"));
    }

    #[test]
    fn test_a_balanced_stance_names_no_finding() {
        let parts = paragraphs(8, 60, '字');
        let stats = run_voice_analysis(&doc_of(&owned(&parts))).unwrap();
        assert_eq!(stats.stance.verdict, "balanced");
        assert!(!kinds(&stats).contains(&"hedging"));
        assert!(!kinds(&stats).contains(&"assertive"));
    }

    #[test]
    fn test_classical_particles_are_named_as_a_style() {
        let mut parts = paragraphs(12, 60, '字');
        parts.push("来之则安之，其言也善，行者常至。".to_string());
        let stats = run_voice_analysis(&doc_of(&owned(&parts))).unwrap();
        assert!(kinds(&stats).contains(&"classical"));
    }

    #[test]
    fn test_short_text_says_only_that_it_is_short() {
        let stats = run_voice_analysis(&doc_of(&["他走了。她留下了。"])).unwrap();
        assert_eq!(kinds(&stats), vec!["too_short"]);
        // The curve is still cut into chunks and still counts what it saw; only
        // the findings hold back, because one page cannot show a habit.
        assert_eq!(stats.points.iter().map(|p| p.sentences).sum::<u32>(), 2);
    }

    #[test]
    fn test_an_empty_document_gives_zeros_not_a_panic() {
        let stats = run_voice_analysis(&doc_of(&[])).unwrap();
        assert_eq!(stats.char_count, 0);
        assert_eq!(stats.sentence_count, 0);
        assert_eq!(stats.avg_sentence_chars, 0.0);
        assert_eq!(stats.pronouns.total, 0);
        assert_eq!(stats.stance.verdict, "balanced");
        assert_eq!(kinds(&stats), vec!["too_short"]);
    }

    #[test]
    fn test_the_curve_shares_the_segments_other_curves_use() {
        let parts = paragraphs(10, 60, '字');
        let stats = run_voice_analysis(&doc_of(&owned(&parts))).unwrap();
        assert_eq!(stats.points.len() as u32, stats.segment_count);
        // One section only, so the cut falls back to block chunks — the same cut
        // the pacing, conflict, theme, setting and dialogue curves are drawn on.
        assert!(stats.chunked);
        assert_eq!(stats.points[0].section, "1/10");
        let chars: u64 = stats.points.iter().map(|p| p.char_count).sum();
        assert_eq!(chars, stats.char_count);
    }

    #[test]
    fn test_the_same_text_gives_the_same_voice() {
        let parts = paragraphs(8, 45, '字');
        let doc = doc_of(&owned(&parts));
        let a = serde_json::to_string(&run_voice_analysis(&doc).unwrap()).unwrap();
        let b = serde_json::to_string(&run_voice_analysis(&doc).unwrap()).unwrap();
        assert_eq!(a, b);
    }
}
