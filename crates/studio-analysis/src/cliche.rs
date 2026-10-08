//! Cliché detection (陈词滥调查找): the stock phrases the text leans on.
//!
//! The design doc specifies a phrase library plus fuzzy matching, and that is what
//! this module does — nothing more. It carries a curated list of stock expressions
//! (表情 / 身体反应 / 情绪 / 比喻 / 场面 / 说话 / 过渡 / 外貌), finds where the text uses
//! them, and reports each one with the sentence around it so a person can decide.
//! It does not claim that a phrase is bad writing: 深吸一口气 in isolation is a
//! sentence like any other. What it measures is *reliance* — how often the text
//! reaches for a ready-made phrase instead of finding the one that fits.
//!
//! Matching is exact on characters, with one controlled allowance: a pattern may
//! carry alternation groups (`(微微|淡淡)一笑`), which stand for the same cliché in
//! different dress and are counted together. Patterns never overlap: at each
//! position the longest pattern wins, so 「空气仿佛凝固」 is one hit, not three.
//!
//! Two limits are stated in the output rather than hidden. Only phrases in the
//! library are found, so silence means the library found nothing — not that the
//! prose is fresh. And a phrase a character *says* is counted too: whether a
//! cliché inside dialogue is a flaw or a voice choice is a reader's call, so the
//! hit is flagged as dialogue instead of dropped.
//!
//! Deterministic: fixed lexicon, segments in reading order, no clock, no
//! randomness; the same document always yields the same hits and the same score.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use studio_core::document::DocumentData;

use crate::segment::plan_segments;
use crate::text::visible_char_count;
use crate::Result;

/// Clichés per 10 000 visible characters at which a segment saturates the curve.
const CLICHE_FULL: f64 = 40.0;
/// Below this overall density the document is called light on stock phrases.
const LIGHT_PER_10K: f64 = 10.0;
/// Above this, heavy.
const HEAVY_PER_10K: f64 = 25.0;
/// Up to this many hits per 10 000 characters the document is not charged at all.
const CLEAN_PER_10K: f64 = 4.0;
/// A segment at or above this density is reported as a pile-up.
const DENSE_SEGMENT_PER_10K: f64 = 30.0;
/// Times one phrase has to appear before repeating it is itself the problem.
const REPEAT_THRESHOLD: u32 = 3;
/// Below this many visible characters a per-10 000 rate is extrapolated from too
/// little text to mean anything, so nothing is scored.
const MIN_CHARS_FOR_SCORE: u64 = 1000;
/// How many phrases are reported, most used first.
const MAX_PHRASES: usize = 20;
/// How many individual hits are reported, document order.
const MAX_HITS: usize = 60;
/// How many issues are reported, first found first.
const MAX_ISSUES: usize = 12;
/// Characters of context kept on each side of a hit.
const EXCERPT_RADIUS: usize = 24;
/// Variants a single pattern may expand into; guards against a careless pattern.
const MAX_VARIANTS: usize = 16;

/// The phrase library: `pattern, category, note`.
///
/// Every pattern is at least two characters — Chinese has no word boundaries, and a
/// lone 心 or 笑 would register on ordinary prose. The note says what shape the fix
/// should take; it is not a thesaurus, because the engine cannot know what this
/// character would actually do.
const CLICHE_LIBRARY: &[(&str, &str, &str)] = &[
    // 表情动作
    (
        "微微一笑",
        "expression",
        "万能表情，换成这个人物特有的面部反应",
    ),
    (
        "(淡淡|浅浅)一笑",
        "expression",
        "「一笑」能接在任何一句对白后，改写成说话时的具体动作",
    ),
    (
        "嘴角勾起",
        "expression",
        "套路化的笑法，写笑意从哪里来、到哪儿止",
    ),
    (
        "嘴角(微微)?(上扬|翘起)",
        "expression",
        "同上：笑的位置和形状要具体",
    ),
    (
        "一抹弧度",
        "expression",
        "「一抹」是凑字数的量词，删掉后重写",
    ),
    (
        "挑了挑眉",
        "expression",
        "通用的示意动作，换成这个人会怎么做",
    ),
    (
        "眉头(微|紧|轻)?(皱|锁|蹙)",
        "expression",
        "皱眉用得太滥，写他真正在抗拒什么",
    ),
    (
        "眯起(了)?眼睛",
        "expression",
        "通用反应，配合具体动机才有意义",
    ),
    (
        "面无表情",
        "expression",
        "「没有表情」不是描写，写脸上哪里绷着",
    ),
    ("神色如常", "expression", "用结论代替了描写"),
    (
        "似笑非笑",
        "expression",
        "含义模糊的套话，读者不知道那是什么表情",
    ),
    ("若有所思", "expression", "直接宣布心理状态，不写他在想什么"),
    ("意味深长", "expression", "替读者下结论：哪里深长要写出来"),
    (
        "眼中闪过一丝(惊讶|喜色|寒意|狠色|黯然)",
        "expression",
        "「闪过一丝」是套话骨架，换掉整句",
    ),
    ("眼底闪过", "expression", "同上"),
    ("瞳孔(骤|猛|微)?(缩|收缩)", "expression", "震惊的套路写法"),
    (
        "脸色(微变|一变|铁青|煞白|沉了下来)",
        "expression",
        "写这张脸具体怎么了",
    ),
    ("面沉如水", "expression", "套话比喻"),
    ("玩味(地|的笑容|的眼神)?", "expression", "含糊的情绪标签"),
    ("邪魅一笑", "expression", "网文高频套话，几乎不指具体表情"),
    ("扯了扯嘴角", "expression", "万能过渡动作"),
    (
        "苦笑(一声|着)?",
        "expression",
        "「苦笑」概括了表情，没有写出来",
    ),
    ("笑而不语", "expression", "用成语代替描写"),
    // 身体反应
    (
        "深吸一口气",
        "body",
        "紧张或平静的通用动作，写呼吸的具体感官",
    ),
    (
        "呼吸(一滞|急促|乱了)",
        "body",
        "常见的即时反应，换一个只属于此刻的反应",
    ),
    (
        "心跳(漏了半拍|漏了一拍|加速|骤然加快|如鼓)",
        "body",
        "心跳是万能情绪指标，换掉",
    ),
    (
        "(心|心头|心中)(一紧|一沉|一凛|一颤|咯噔一下)",
        "body",
        "「心头一紧」几乎万能，删或重写",
    ),
    (
        "手心(出汗|冒汗|沁出冷汗)",
        "body",
        "紧张套路，写他怎么处理这双手",
    ),
    ("指尖(发白|微颤|冰凉)", "body", "细节本身不差，但用得太频繁"),
    ("指节(发白|泛白)", "body", "同上"),
    ("握紧(了)?拳头", "body", "愤怒的默认动作"),
    (
        "指甲(掐进|嵌入|陷进)(了)?掌心",
        "body",
        "在「握紧拳头」之上再加一层套路",
    ),
    ("后背(发凉|一阵发凉|沁出冷汗)", "body", "恐惧的套路落点"),
    ("脊背发凉", "body", "同上"),
    ("冷汗(直流|直冒|沁出|渗出)", "body", "同上"),
    ("如坠冰窟", "body", "寒意成语，写身体具体怎么了"),
    ("一股寒意从脚底", "body", "寒意套路骨架"),
    ("毛骨悚然", "body", "成语代替了具体的恐惧反应"),
    ("头皮发麻", "body", "通用不适反应"),
    ("双腿(发软|一软)", "body", "惊惧的默认反应"),
    ("浑身(一震|僵硬|紧绷)", "body", "全身概括，写具体部位"),
    ("喉咙(发紧|一紧|干涩)", "body", "情绪落到喉咙的套路"),
    ("鼻子一酸", "body", "眼泪之前的固定动作"),
    ("眼眶(湿润|发红|一热)", "body", "同上"),
    (
        "眼泪(不受控制地|不由自主地|夺眶而出)",
        "body",
        "写这滴眼泪为什么来，而不是它怎么来",
    ),
    ("不由自主地", "body", "免除动机的副词，删掉后句子更清楚"),
    ("情不自禁", "body", "同上"),
    ("下意识(地)?", "body", "同上"),
    ("鬼使神差", "body", "用成语代替人物做决定的过程"),
    // 情绪直陈
    (
        "五味杂陈",
        "emotion",
        "四种味道不等于四种情绪，写清是哪几种",
    ),
    ("百感交集", "emotion", "同上：概括代替了具体"),
    ("复杂的情绪", "emotion", "「复杂」是承认没写清楚"),
    ("说不出的滋味", "emotion", "说不出来就不写，读者也无从感受"),
    (
        "莫名的(烦躁|恐慌|委屈|难过)",
        "emotion",
        "「莫名」放弃了原因",
    ),
    ("怒火中烧", "emotion", "成语代替愤怒的写法"),
    ("一股无名火", "emotion", "同上"),
    ("气得浑身发抖", "emotion", "愤怒的套路终点"),
    ("心中暗道", "emotion", "把内心独白挂在一个固定引导语上"),
    ("暗自思忖", "emotion", "文言套话，直接写想法"),
    (
        "一阵(酸涩|刺痛|悸动|失落)",
        "emotion",
        "「一阵」是万能量词，写清对象",
    ),
    (
        "胸口(一阵剧痛|发闷|像被压住)",
        "emotion",
        "情绪落进胸口的套路",
    ),
    ("心如刀绞", "emotion", "成语代替痛苦的具体形状"),
    // 套路比喻
    (
        "仿佛整个世界都(安静|静止|旋转)了",
        "simile",
        "世界配合主角的比喻，收窄到这个人听见了什么",
    ),
    (
        "空气(仿佛|似乎|好像)?(在这一刻)?凝固",
        "simile",
        "最常用的停顿句，换一个只属于这个房间的静止",
    ),
    ("时间(仿佛|好像|似乎)?(静止|停滞|凝固)了", "simile", "同上"),
    ("整个世界都安静了", "simile", "同上"),
    (
        "仿佛过了(一个世纪|很久|漫长的瞬间)",
        "simile",
        "时间膨胀套话",
    ),
    (
        "死一般的寂静",
        "simile",
        "「死一般」是空量词，写静到什么程度",
    ),
    (
        "犹如(晴天霹雳|雷击|噩梦)",
        "simile",
        "成语比喻，写消息具体怎样落地",
    ),
    ("像被(雷劈|电击|击中)(了)?(一样)?", "simile", "同上"),
    ("如遭雷击", "simile", "同上"),
    ("仿佛被施了定身法", "simile", "套路化的呆住"),
    ("像(被|被人)掐住了(脖子|喉咙)", "simile", "压迫感的常用比喻"),
    ("仿佛被抽空了(力气|所有)", "simile", "脱力的常用说法"),
    // 场面套话
    ("鸦雀无声", "atmosphere", "成语代替了具体的静"),
    ("落针可闻", "atmosphere", "同上"),
    (
        "一片哗然",
        "atmosphere",
        "群体反应的概括，写谁先出声、说了什么",
    ),
    ("全场哗然", "atmosphere", "同上"),
    ("倒吸(了)?一口凉气", "atmosphere", "群体反应的默认动作"),
    ("议论纷纷", "atmosphere", "概括代替了具体的话"),
    ("炸开了锅", "atmosphere", "口语套话"),
    (
        "空气(突然|顿时|一下子)安静(下来|了)",
        "atmosphere",
        "场面转折的固定写法",
    ),
    // 说话套话
    (
        "淡淡(开口|说|道|回答)",
        "speech_tag",
        "「淡淡地」几乎能修饰任何一句对白",
    ),
    ("缓缓(开口|说|道)", "speech_tag", "「缓缓」是凑节奏的副词"),
    ("一字一顿", "speech_tag", "强调的固定写法，写他为什么咬字"),
    ("沉声(开口|说|道|问)", "speech_tag", "对白标签套话"),
    ("冷声(说|道|开口)", "speech_tag", "同上"),
    (
        "声音不大(，却)?",
        "speech_tag",
        "「声音不大却传遍全场」的固定开头",
    ),
    ("平静无波", "speech_tag", "语气套话"),
    (
        "语气(平淡|平静|冷淡|生硬)",
        "speech_tag",
        "写语气不如写说话方式",
    ),
    (
        "顿了顿",
        "speech_tag",
        "最常用的停顿填充，删掉通常不影响句子",
    ),
    ("沉吟(片刻|半晌)?", "speech_tag", "文言化的停顿填充"),
    ("沉默(了)?(片刻|半晌|一会儿)", "speech_tag", "停顿填充"),
    ("半晌(才)?", "speech_tag", "同上"),
    ("良久", "speech_tag", "同上"),
    // 时间过渡
    ("片刻之后", "transition", "过渡套话，直接写下一件事"),
    ("没过多久", "transition", "同上"),
    ("不一会儿", "transition", "同上"),
    ("就在这时", "transition", "制造突兀的固定接缝"),
    ("下一秒", "transition", "同上"),
    ("说时迟那时快", "transition", "评书腔的固定套语"),
    ("电光火石(之间|之际)", "transition", "同上"),
    ("千钧一发", "transition", "用成语宣布危险，不写危险本身"),
    ("生死一线", "transition", "同上"),
    ("次日清晨", "transition", "转场固定句，写醒来时看见什么"),
    ("第二天一早", "transition", "同上"),
    // 外貌套话
    (
        "剑眉(星目|斜飞|入鬓)",
        "look",
        "外貌模板词，写这张脸具体特别在哪",
    ),
    ("(深邃|狭长|桃花|丹凤)(的)?眼(眸|神)", "look", "眼睛模板"),
    ("薄唇轻启", "look", "外貌套话"),
    ("棱角分明的(脸|脸庞|下颌)", "look", "脸模板"),
    ("刀削般的(脸|脸庞|侧脸|轮廓)", "look", "同上"),
    ("肤若凝脂", "look", "成语外貌"),
    ("明眸皓齿", "look", "同上"),
    ("清冷(的)?气质", "look", "气质标签，不是描写"),
    ("绝美的容颜", "look", "形容词加名词，没有形象"),
    ("美得不似(凡人|真人|人间)", "look", "夸张套话"),
    ("目光如炬", "look", "成语代替眼神"),
    (
        "鹰隼(般|一样|似的)(的)?(目光|眼神|眼睛)",
        "look",
        "比喻模板",
    ),
    ("钢铁般的(意志|眼神)", "look", "比喻模板"),
    ("坚定(的)?(眼神|目光)", "look", "眼神模板"),
    ("黑眸(深邃|如墨|幽深)", "look", "网文外貌模板"),
];

/// One segment of the cliché curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClichePoint {
    pub section: String,
    pub char_count: u64,
    pub hits: u32,
    /// Hits per 10 000 visible characters.
    pub per_10k: f64,
    /// 0-100 on the absolute scale, so two projects' curves can be compared.
    pub intensity: f64,
    /// The phrase used most here; empty when the segment has none.
    pub top_phrase: String,
}

/// One library phrase and how hard the document leans on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClichePhrase {
    /// The library pattern, e.g. `(淡淡|浅浅)一笑`.
    pub phrase: String,
    /// `expression` | `body` | `emotion` | `simile` | `atmosphere` | `speech_tag` |
    /// `transition` | `look`.
    pub category: String,
    pub count: u32,
    /// Share of all hits this phrase accounts for (0-1).
    pub share: f64,
    /// The forms actually found, busiest first — a pattern with alternations shows
    /// here which dress the cliché wore.
    pub variants: Vec<(String, u32)>,
    /// Curve segments that contain at least one hit.
    pub segments_touched: u32,
    /// Share of the curve's segments this phrase reaches (0-1).
    pub coverage: f64,
    pub first_section: String,
    pub last_section: String,
    /// What shape the replacement should take.
    pub note: String,
}

/// One place in the text where a library phrase was found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClicheHit {
    /// The curve segment the hit sits in.
    pub section: String,
    /// The text actually matched.
    pub phrase: String,
    /// The library pattern it came from.
    pub pattern: String,
    pub category: String,
    /// Character offset inside the block, so the UI can point at it.
    pub offset: usize,
    /// The surrounding characters, so the phrase can be judged in context.
    pub excerpt: String,
    /// The hit is inside quotation marks: someone says it rather than narrates it.
    pub in_dialogue: bool,
    pub note: String,
}

/// A pattern after its alternation groups have been expanded.
#[derive(Debug, Clone)]
struct Compiled {
    pattern: &'static str,
    category: &'static str,
    note: &'static str,
    variants: Vec<Vec<char>>,
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
    phrases: HashMap<usize, u32>,
}

/// Per-phrase counters.
#[derive(Debug, Default)]
struct PhraseTally {
    count: u32,
    variants: HashMap<String, u32>,
    /// Variant texts in first-appearance order, so the list is not HashMap order.
    variant_order: Vec<String>,
    segments: HashMap<usize, u32>,
    first_segment: usize,
    last_segment: usize,
}

/// Cliché analysis output for one document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClicheStats {
    pub points: Vec<ClichePoint>,
    pub segment_count: u32,
    /// True when the segments are block chunks rather than sections.
    pub chunked: bool,
    pub total_hits: u32,
    /// Library patterns found at least once.
    pub distinct_phrases: u32,
    /// Visible characters the rate is measured over.
    pub char_count: u64,
    /// Hits per 10 000 visible characters across the document.
    pub per_10k: f64,
    /// `none` | `light` | `moderate` | `heavy` — library coverage, not quality.
    pub verdict: String,
    /// Categories present, most hits first.
    pub categories: Vec<(String, u32)>,
    /// Reported phrases, most used first.
    pub phrases: Vec<ClichePhrase>,
    /// Reported hits, document order.
    pub hits: Vec<ClicheHit>,
    /// Hits inside quotation marks.
    pub dialogue_hits: u32,
    /// Phrases used [`REPEAT_THRESHOLD`] times or more.
    pub repeated_phrases: u32,
    /// Hits belonging to those repeated phrases.
    pub repeat_hits: u32,
    /// Share of all hits held by the single most used phrase (0-1).
    pub top_share: f64,
    pub top_phrase: String,
    pub peak_section: String,
    pub peak_intensity: f64,
    pub issues: Vec<ClicheIssue>,
    /// How many patterns the library carries — coverage the reader can weigh.
    pub lexicon_size: usize,
    /// 0-100, or `None` when the text is too short for a per-10 000 rate to mean
    /// anything.
    pub cliche_score: Option<f64>,
}

/// Something the phrase use does to the prose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClicheIssue {
    /// `repeated` | `dense_segment` | `one_note` | `speech_tags`.
    pub kind: String,
    pub section: String,
    pub detail: String,
}

/// Expand a pattern's alternation groups into the variants it stands for.
///
/// `(淡淡|浅浅)一笑` yields `淡淡一笑` and `浅浅一笑`; a `?` after a group means the
/// group may also be absent. Expansion is capped at [`MAX_VARIANTS`] so a careless
/// pattern cannot blow up the index.
fn expand(pattern: &str) -> Vec<Vec<char>> {
    let mut current: Vec<Vec<char>> = vec![Vec::new()];
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '(' {
            for variant in &mut current {
                variant.push(c);
            }
            continue;
        }

        let mut body = String::new();
        while let Some(&c) = chars.peek() {
            if c == ')' {
                chars.next();
                break;
            }
            body.push(c);
            chars.next();
        }

        let mut options: Vec<Vec<char>> = body
            .split('|')
            .map(|part| part.chars().collect::<Vec<char>>())
            .collect();
        if chars.peek() == Some(&'?') {
            chars.next();
            options.push(Vec::new());
        }

        let mut next: Vec<Vec<char>> = Vec::new();
        for prefix in &current {
            for option in &options {
                let mut merged = prefix.clone();
                merged.extend(option.iter().copied());
                next.push(merged);
            }
        }
        next.truncate(MAX_VARIANTS);
        current = next;
    }

    current.retain(|v| !v.is_empty());
    // Longest first: the scanner relies on this to take the longest match at a
    // position, which is what stops overlapping patterns from double counting.
    current.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    current
}

/// The library, compiled once per analysis.
fn compile_library() -> Vec<Compiled> {
    CLICHE_LIBRARY
        .iter()
        .map(|(pattern, category, note)| Compiled {
            pattern,
            category,
            note,
            variants: expand(pattern),
        })
        .collect()
}

/// Map first character → `(pattern index, variant index)`, longest variant first.
///
/// Indexing on the first character keeps a full pass over a novel linear: at each
/// position only the handful of patterns starting with that character are tried.
fn build_index(lexicon: &[Compiled]) -> HashMap<char, Vec<(usize, usize)>> {
    let mut index: HashMap<char, Vec<(usize, usize)>> = HashMap::new();
    for (p, entry) in lexicon.iter().enumerate() {
        for (v, variant) in entry.variants.iter().enumerate() {
            if let Some(&first) = variant.first() {
                index.entry(first).or_default().push((p, v));
            }
        }
    }
    for candidates in index.values_mut() {
        candidates.sort_by(|a, b| {
            let la = lexicon[a.0].variants[a.1].len();
            let lb = lexicon[b.0].variants[b.1].len();
            lb.cmp(&la).then_with(|| a.cmp(b))
        });
    }
    index
}

/// Whether `variant` fits at `at`.
fn fits(chars: &[char], at: usize, variant: &[char]) -> bool {
    at + variant.len() <= chars.len() && chars[at..at + variant.len()] == *variant
}

/// Opening and closing quotation marks, as the dialogue measures elsewhere treat them.
fn is_open_quote(c: char) -> bool {
    matches!(c, '“' | '「' | '『' | '"')
}

fn is_close_quote(c: char) -> bool {
    matches!(c, '”' | '」' | '』')
}

fn apply_quote(c: char, in_quote: &mut bool) {
    if is_open_quote(c) {
        *in_quote = true;
    } else if is_close_quote(c) {
        *in_quote = false;
    }
}

/// A window of context around a hit, with ellipses where characters were cut.
fn excerpt(chars: &[char], at: usize, len: usize) -> String {
    let start = at.saturating_sub(EXCERPT_RADIUS);
    let end = (at + len + EXCERPT_RADIUS).min(chars.len());
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(chars[start..end].iter());
    if end < chars.len() {
        out.push('…');
    }
    out
}

/// Walk one block and collect its hits.
///
/// The walk advances one character at a time except past a match, so hits never
/// overlap. Quotation state is tracked in the same pass: a phrase inside 「」 is
/// someone's line rather than the narration's, and that difference is reported
/// instead of resolved.
fn scan_block(
    content: &str,
    index: &HashMap<char, Vec<(usize, usize)>>,
    lexicon: &[Compiled],
) -> Vec<RawHit> {
    let chars: Vec<char> = content.chars().collect();
    let mut hits = Vec::new();
    let mut in_quote = false;
    let mut i = 0usize;

    while i < chars.len() {
        apply_quote(chars[i], &mut in_quote);
        let mut step = 1usize;

        if let Some(candidates) = index.get(&chars[i]) {
            for &(p, v) in candidates {
                let variant = &lexicon[p].variants[v];
                if fits(&chars, i, variant) {
                    hits.push(RawHit {
                        pattern_idx: p,
                        matched: variant.iter().collect(),
                        char_offset: i,
                        excerpt: excerpt(&chars, i, variant.len()),
                        in_dialogue: in_quote,
                        segment: 0,
                    });
                    step = variant.len();
                    break;
                }
            }
        }

        // Quotation state has to stay correct across the characters the match skipped.
        for c in &chars[i + 1..i + step] {
            apply_quote(*c, &mut in_quote);
        }
        i += step;
    }

    hits
}

/// Scan every block, tagging each hit with the curve segment it falls in.
fn collect_hits(
    doc: &DocumentData,
    index: &HashMap<char, Vec<(usize, usize)>>,
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

        let hits = scan_block(&block.content, index, lexicon);
        for mut hit in hits {
            hit.segment = segment;
            let seg = &mut segments[segment];
            seg.hits += 1;
            *seg.phrases.entry(hit.pattern_idx).or_insert(0) += 1;
            all_hits.push(hit);
        }
    }

    (segments, all_hits, plan.chunked)
}

/// The busiest library pattern inside one segment; ties go to the pattern that sorts
/// first, so the label never depends on HashMap order.
fn top_pattern(segment: &Segment, lexicon: &[Compiled]) -> String {
    let mut best: Option<(usize, u32)> = None;
    for (idx, count) in &segment.phrases {
        if best.map(|(_, c)| *count > c).unwrap_or(true) {
            best = Some((*idx, *count));
        }
    }
    best.map(|(i, _)| lexicon[i].pattern.to_string())
        .unwrap_or_default()
}

/// One library phrase as the report shows it.
fn phrase_view(
    idx: usize,
    tally: &PhraseTally,
    lexicon: &[Compiled],
    segments: &[Segment],
    total_hits: u32,
    segment_count: u32,
) -> ClichePhrase {
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

    ClichePhrase {
        phrase: lexicon[idx].pattern.to_string(),
        category: lexicon[idx].category.to_string(),
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

/// How much stock phrasing the document carries. `none` deliberately does not say
/// "the prose is fresh": it says the library found nothing.
fn verdict(per_10k: f64, total_hits: u32) -> &'static str {
    if total_hits == 0 {
        "none"
    } else if per_10k < LIGHT_PER_10K {
        "light"
    } else if per_10k > HEAVY_PER_10K {
        "heavy"
    } else {
        "moderate"
    }
}

/// What the phrase use costs the prose, read from the stats themselves.
///
/// 100 minus three charges:
/// - rate: every hit per 10 000 characters above [`CLEAN_PER_10K`] costs 2.5, up to
///   −45. A document at the design doc's benchmark of 15 per 10 000 lands at 72.5.
/// - repetition: one point per hit of a phrase used [`REPEAT_THRESHOLD`] times or
///   more, up to −20 — reusing one phrase is the cheapest way to sound canned.
/// - one note: −10 when a single phrase carries a quarter of all hits.
///
/// Text shorter than [`MIN_CHARS_FOR_SCORE`] is left unscored: a rate per 10 000
/// characters extrapolated from a page is a guess, and a guess is not a grade.
fn cliche_score(stats: &ClicheStats) -> Option<f64> {
    if stats.char_count < MIN_CHARS_FOR_SCORE {
        return None;
    }
    let mut score = 100.0;
    if stats.per_10k > CLEAN_PER_10K {
        score -= ((stats.per_10k - CLEAN_PER_10K) * 2.5).min(45.0);
    }
    score -= stats.repeat_hits.min(20) as f64;
    if stats.total_hits >= 8 && stats.top_share >= 0.25 {
        score -= 10.0;
    }
    Some(score.max(0.0))
}

/// The phrase habits worth fixing, in the order they were found.
fn find_issues(
    segments: &[Segment],
    tallies: &[PhraseTally],
    lexicon: &[Compiled],
    total_hits: u32,
) -> Vec<ClicheIssue> {
    let mut issues: Vec<ClicheIssue> = Vec::new();

    // Phrases the document keeps returning to, most repeated first.
    let mut repeated: Vec<(usize, u32)> = tallies
        .iter()
        .enumerate()
        .filter(|(_, t)| t.count >= REPEAT_THRESHOLD)
        .map(|(i, t)| (i, t.count))
        .collect();
    repeated.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| lexicon[a.0].pattern.cmp(lexicon[b.0].pattern))
    });
    for (idx, count) in repeated.into_iter().take(6) {
        let tally = &tallies[idx];
        let forms: Vec<String> = tally
            .variant_order
            .iter()
            .take(3)
            .map(|v| format!("{v}×{}", tally.variants[v]))
            .collect();
        issues.push(ClicheIssue {
            kind: "repeated".to_string(),
            section: segments[tally.first_segment].label.clone(),
            detail: format!(
                "「{}」出现 {} 次（{}）",
                lexicon[idx].pattern,
                count,
                forms.join("、")
            ),
        });
    }

    // Segments where the phrases pile up, in reading order.
    for segment in segments {
        let chars = segment.chars.max(1) as f64;
        let per_10k = segment.hits as f64 / chars * 10000.0;
        if segment.hits > 0 && per_10k >= DENSE_SEGMENT_PER_10K {
            issues.push(ClicheIssue {
                kind: "dense_segment".to_string(),
                section: segment.label.clone(),
                detail: format!("每万字 {:.1} 处老套表达（{} 处）", per_10k, segment.hits),
            });
        }
    }

    if total_hits >= 8 {
        // One phrase doing a quarter of the work.
        let one_note = tallies
            .iter()
            .map(|t| t.count)
            .max()
            .map(|c| c as f64 / total_hits as f64)
            .unwrap_or(0.0);
        if one_note >= 0.25 {
            issues.push(ClicheIssue {
                kind: "one_note".to_string(),
                section: String::new(),
                detail: format!(
                    "最常用的一句占了全部命中 {:.0}%：同一个动作反复出现，读者会开始看见措辞而不是场景",
                    one_note * 100.0
                ),
            });
        }

        // Dialogue tags doing a quarter of the work: the talking, not the talk.
        let speech: u32 = tallies
            .iter()
            .enumerate()
            .filter(|(i, _)| lexicon[*i].category == "speech_tag")
            .map(|(_, t)| t.count)
            .sum();
        if speech as f64 / total_hits as f64 >= 0.25 {
            issues.push(ClicheIssue {
                kind: "speech_tags".to_string(),
                section: String::new(),
                detail: format!(
                    "对白标签类套话 {speech} 处，占 {:.0}%：说话的方式可以写出来，不必都挂在副词上",
                    speech as f64 / total_hits as f64 * 100.0
                ),
            });
        }
    }

    issues.truncate(MAX_ISSUES);
    issues
}

/// Count what the library finds, segment by segment.
pub fn run_cliche_analysis(doc: &DocumentData) -> Result<ClicheStats> {
    let lexicon = compile_library();
    let index = build_index(&lexicon);
    let (segments, all_hits, chunked) = collect_hits(doc, &index, &lexicon);

    let total_chars: u64 = segments.iter().map(|s| s.chars).sum();
    let total_hits = all_hits.len() as u32;
    let per_10k = if total_chars == 0 {
        0.0
    } else {
        total_hits as f64 / total_chars as f64 * 10000.0
    };
    let segment_count = segments.len() as u32;

    // Per-phrase tallies, indexed by position in the library, so the order is the
    // library's own rather than a HashMap's.
    let mut tallies: Vec<PhraseTally> =
        (0..lexicon.len()).map(|_| PhraseTally::default()).collect();
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

    // Phrases ranked by use, then by pattern text: two documents with the same
    // counts list the same phrases in the same order.
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

    let phrases: Vec<ClichePhrase> = ranked
        .iter()
        .take(MAX_PHRASES)
        .map(|&(idx, _)| {
            phrase_view(
                idx,
                &tallies[idx],
                &lexicon,
                &segments,
                total_hits,
                segment_count,
            )
        })
        .collect();

    let mut category_counts: HashMap<&str, u32> = HashMap::new();
    for (idx, count) in &ranked {
        *category_counts.entry(lexicon[*idx].category).or_insert(0) += count;
    }
    let mut categories: Vec<(String, u32)> = category_counts
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    categories.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let points: Vec<ClichePoint> = segments
        .iter()
        .map(|s| {
            let chars = s.chars.max(1) as f64;
            let seg_per_10k = s.hits as f64 / chars * 10000.0;
            ClichePoint {
                section: s.label.clone(),
                char_count: s.chars,
                hits: s.hits,
                per_10k: seg_per_10k,
                intensity: (seg_per_10k / CLICHE_FULL).min(1.0) * 100.0,
                top_phrase: top_pattern(s, &lexicon),
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

    let hits: Vec<ClicheHit> = all_hits
        .iter()
        .take(MAX_HITS)
        .map(|h| ClicheHit {
            section: segments[h.segment].label.clone(),
            phrase: h.matched.clone(),
            pattern: lexicon[h.pattern_idx].pattern.to_string(),
            category: lexicon[h.pattern_idx].category.to_string(),
            offset: h.char_offset,
            excerpt: h.excerpt.clone(),
            in_dialogue: h.in_dialogue,
            note: lexicon[h.pattern_idx].note.to_string(),
        })
        .collect();

    let issues = find_issues(&segments, &tallies, &lexicon, total_hits);

    let mut stats = ClicheStats {
        points,
        segment_count,
        chunked,
        total_hits,
        distinct_phrases: tallies.iter().filter(|t| t.count > 0).count() as u32,
        char_count: total_chars,
        per_10k,
        verdict: verdict(per_10k, total_hits).to_string(),
        categories,
        phrases,
        hits,
        dialogue_hits: all_hits.iter().filter(|h| h.in_dialogue).count() as u32,
        repeated_phrases: tallies
            .iter()
            .filter(|t| t.count >= REPEAT_THRESHOLD)
            .count() as u32,
        repeat_hits: tallies
            .iter()
            .filter(|t| t.count >= REPEAT_THRESHOLD)
            .map(|t| t.count)
            .sum(),
        top_share: ranked
            .first()
            .map(|(_, c)| {
                if total_hits == 0 {
                    0.0
                } else {
                    *c as f64 / total_hits as f64
                }
            })
            .unwrap_or(0.0),
        top_phrase: ranked
            .first()
            .map(|(i, _)| lexicon[*i].pattern.to_string())
            .unwrap_or_default(),
        peak_section,
        peak_intensity,
        issues,
        lexicon_size: CLICHE_LIBRARY.len(),
        cliche_score: None,
    };
    // The score reads the stats rather than a parameter list: every number it charges
    // for is one the caller can see in the output.
    stats.cliche_score = cliche_score(&stats);
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

    /// A sentence with no library phrase in it, used to reach a scorable length.
    const FILLER: &str = "他们把话说完，事情就算定了下来。";

    /// Distinct library phrases, for the tests that need many hits without tripping
    /// the repetition charge.
    const DISTINCT: [&str; 24] = [
        "微微一笑",
        "深吸一口气",
        "五味杂陈",
        "鸦雀无声",
        "就在这时",
        "目光如炬",
        "顿了顿",
        "千钧一发",
        "一片哗然",
        "如坠冰窟",
        "薄唇轻启",
        "次日清晨",
        "毛骨悚然",
        "头皮发麻",
        "怒火中烧",
        "若有所思",
        "剑眉星目",
        "明眸皓齿",
        "一字一顿",
        "半晌",
        "良久",
        "没过多久",
        "不一会儿",
        "炸开了锅",
    ];

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

    /// One block: some phrases plus enough filler to be long enough to score.
    fn doc_with(phrases: &[&str], filler_repeats: usize) -> DocumentData {
        let mut content = String::new();
        for p in phrases {
            content.push_str(p);
            content.push('，');
        }
        content.push_str(&FILLER.repeat(filler_repeats));
        doc_of(vec![para("第一章", &content)])
    }

    #[test]
    fn test_hit_reports_phrase_offset_and_context() {
        let stats = run_cliche_analysis(&doc_of(vec![para(
            "第一章",
            "他站在门外，深吸一口气，才推门进去。",
        )]))
        .unwrap();

        assert_eq!(stats.total_hits, 1);
        assert_eq!(stats.distinct_phrases, 1);
        let hit = &stats.hits[0];
        assert_eq!(hit.phrase, "深吸一口气");
        assert_eq!(hit.pattern, "深吸一口气");
        assert_eq!(hit.category, "body");
        // Character offset, not byte offset: 他站在门外， is 6 characters.
        assert_eq!(hit.offset, 6);
        assert!(hit.excerpt.contains("深吸一口气"));
        assert!(!hit.in_dialogue);
        assert!(!hit.note.is_empty());
    }

    #[test]
    fn test_alternation_variants_roll_up_under_one_pattern() {
        let stats =
            run_cliche_analysis(&doc_of(vec![para("第一章", "她淡淡一笑。他又浅浅一笑。")]))
                .unwrap();

        assert_eq!(stats.total_hits, 2);
        // One pattern, not two: the alternation stands for one cliché in two dresses.
        assert_eq!(stats.distinct_phrases, 1);
        let phrase = &stats.phrases[0];
        assert_eq!(phrase.phrase, "(淡淡|浅浅)一笑");
        assert_eq!(phrase.count, 2);
        assert_eq!(
            phrase.variants,
            [("淡淡一笑".to_string(), 1), ("浅浅一笑".to_string(), 1)]
        );
    }

    #[test]
    fn test_longest_pattern_wins_and_hits_never_overlap() {
        let stats = run_cliche_analysis(&doc_of(vec![para(
            "第一章",
            "屋里的空气仿佛凝固了，谁也没有先开口。",
        )]))
        .unwrap();

        // 「空气仿佛凝固」 is one hit, not one for the long pattern plus one each for
        // the shorter patterns that live inside it.
        assert_eq!(stats.total_hits, 1);
        assert_eq!(
            stats.hits[0].pattern,
            "空气(仿佛|似乎|好像)?(在这一刻)?凝固"
        );
        assert_eq!(stats.hits[0].phrase, "空气仿佛凝固");
    }

    #[test]
    fn test_optional_group_matches_both_forms() {
        let stats = run_cliche_analysis(&doc_of(vec![para(
            "第一章",
            "台下倒吸一口凉气。台上倒吸了一口凉气。",
        )]))
        .unwrap();

        assert_eq!(stats.total_hits, 2);
        assert_eq!(stats.phrases[0].phrase, "倒吸(了)?一口凉气");
        assert_eq!(stats.phrases[0].count, 2);
    }

    #[test]
    fn test_density_and_verdict() {
        let clean = run_cliche_analysis(&doc_with(&[], 40)).unwrap();
        assert_eq!(clean.total_hits, 0);
        assert_eq!(clean.verdict, "none");

        let light = run_cliche_analysis(&doc_with(&DISTINCT[..2], 150)).unwrap();
        assert_eq!(light.total_hits, 2);
        assert!(light.per_10k < LIGHT_PER_10K, "per_10k {}", light.per_10k);
        assert_eq!(light.verdict, "light");

        let heavy = run_cliche_analysis(&doc_with(&DISTINCT, 6)).unwrap();
        assert_eq!(heavy.total_hits, DISTINCT.len() as u32);
        assert!(heavy.per_10k > HEAVY_PER_10K, "per_10k {}", heavy.per_10k);
        assert_eq!(heavy.verdict, "heavy");
    }

    #[test]
    fn test_short_text_is_not_scored() {
        // A page of text cannot carry a rate per 10 000 characters.
        let stats =
            run_cliche_analysis(&doc_of(vec![para("第一章", "他深吸一口气，走了。")])).unwrap();
        assert_eq!(stats.total_hits, 1);
        assert!(stats.char_count < MIN_CHARS_FOR_SCORE);
        assert!(stats.cliche_score.is_none());
    }

    #[test]
    fn test_empty_document_has_no_hits_and_no_score() {
        let stats = run_cliche_analysis(&doc_of(vec![])).unwrap();
        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.segment_count, 0);
        assert!(stats.points.is_empty());
        assert!(stats.phrases.is_empty());
        assert_eq!(stats.verdict, "none");
        assert!(stats.cliche_score.is_none());
        assert!(stats.lexicon_size > 100);
    }

    #[test]
    fn test_score_drops_as_the_rate_rises() {
        // Same length, more stock phrases: the rate is what moves the score.
        let few = run_cliche_analysis(&doc_with(&DISTINCT[..2], 300)).unwrap();
        let some = run_cliche_analysis(&doc_with(&DISTINCT[..6], 300)).unwrap();
        let many = run_cliche_analysis(&doc_with(&DISTINCT[..12], 300)).unwrap();

        let (few, some, many) = (
            few.cliche_score.unwrap(),
            some.cliche_score.unwrap(),
            many.cliche_score.unwrap(),
        );
        assert!(few > 95.0, "few {few}");
        assert!(few > some, "{few} vs {some}");
        assert!(some > many, "{some} vs {many}");
    }

    #[test]
    fn test_repeating_one_phrase_is_an_issue() {
        let stats = run_cliche_analysis(&doc_with(&["微微一笑"; 4], 150)).unwrap();

        assert_eq!(stats.total_hits, 4);
        assert_eq!(stats.repeated_phrases, 1);
        assert_eq!(stats.repeat_hits, 4);
        let issue = stats
            .issues
            .iter()
            .find(|i| i.kind == "repeated")
            .expect("repeated issue");
        // One section only, so the curve is cut into chunks: the issue points at the
        // segment where the phrase first appears.
        assert_eq!(issue.section, stats.points[0].section);
        assert!(issue.detail.contains("微微一笑"));
        assert!(issue.detail.contains("4 次"));
    }

    #[test]
    fn test_a_piled_up_segment_is_an_issue() {
        let stats = run_cliche_analysis(&doc_with(&DISTINCT[..10], 2)).unwrap();

        let dense = stats
            .issues
            .iter()
            .find(|i| i.kind == "dense_segment")
            .expect("dense segment issue");
        assert_eq!(dense.section, stats.points[0].section);
        assert!(stats.points[0].intensity > 99.0);
    }

    #[test]
    fn test_one_note_phrase_is_an_issue() {
        let mut phrases = vec!["顿了顿"; 8];
        phrases.push("微微一笑");
        let stats = run_cliche_analysis(&doc_with(&phrases, 150)).unwrap();

        assert_eq!(stats.total_hits, 9);
        assert!(stats.top_share > 0.8);
        assert_eq!(stats.top_phrase, "顿了顿");
        assert!(stats.issues.iter().any(|i| i.kind == "one_note"));
    }

    #[test]
    fn test_dialogue_tags_doing_the_work_are_an_issue() {
        let stats = run_cliche_analysis(&doc_with(
            &[
                "顿了顿",
                "良久",
                "半晌",
                "沉默片刻",
                "淡淡说",
                "微微一笑",
                "五味杂陈",
                "就在这时",
            ],
            150,
        ))
        .unwrap();

        assert_eq!(stats.total_hits, 8);
        assert_eq!(stats.categories[0], ("speech_tag".to_string(), 5));
        assert!(stats.issues.iter().any(|i| i.kind == "speech_tags"));
    }

    #[test]
    fn test_a_phrase_inside_quotation_marks_is_flagged_as_dialogue() {
        let stats = run_cliche_analysis(&doc_of(vec![para(
            "第一章",
            "他说：“微微一笑罢了。”然后转身离开。",
        )]))
        .unwrap();

        assert_eq!(stats.total_hits, 1);
        assert!(stats.hits[0].in_dialogue);
        assert_eq!(stats.dialogue_hits, 1);
    }

    #[test]
    fn test_categories_are_ranked_by_hits_then_name() {
        let stats = run_cliche_analysis(&doc_with(
            &[
                "深吸一口气",
                "头皮发麻",
                "毛骨悚然",
                "微微一笑",
                "若有所思",
                "五味杂陈",
            ],
            150,
        ))
        .unwrap();

        assert_eq!(
            stats.categories,
            [
                ("body".to_string(), 3),
                ("expression".to_string(), 2),
                ("emotion".to_string(), 1),
            ]
        );
    }

    #[test]
    fn test_phrases_are_ranked_by_count_then_pattern() {
        let stats = run_cliche_analysis(&doc_with(
            &["微微一笑", "顿了顿", "微微一笑", "深吸一口气", "顿了顿"],
            150,
        ))
        .unwrap();

        let names: Vec<(String, u32)> = stats
            .phrases
            .iter()
            .map(|p| (p.phrase.clone(), p.count))
            .collect();
        assert_eq!(
            names,
            [
                ("微微一笑".to_string(), 2),
                ("顿了顿".to_string(), 2),
                ("深吸一口气".to_string(), 1),
            ]
        );
    }

    #[test]
    fn test_curve_uses_the_shared_segment_axis() {
        let doc = doc_of(vec![
            para("第一章", "他深吸一口气。"),
            para("第二章", "屋里鸦雀无声。"),
            para("第三章", "这一段没有套话。"),
            para("第四章 / 节", "就在这时，灯灭了。"),
        ]);
        let stats = run_cliche_analysis(&doc).unwrap();

        assert!(!stats.chunked);
        assert_eq!(stats.segment_count, 4);
        assert_eq!(stats.points[0].section, "第一章");
        assert_eq!(stats.points[2].section, "第三章");
        assert_eq!(stats.points[0].hits, 1);
        assert_eq!(stats.points[2].hits, 0);
        assert_eq!(stats.points[3].top_phrase, "就在这时");
    }

    #[test]
    fn test_documents_without_sections_are_cut_into_chunks() {
        let blocks: Vec<SemanticBlock> = (0..25)
            .map(|_| para("同一个章节", "他们把话说完，事情就算定了下来。"))
            .collect();
        let stats = run_cliche_analysis(&doc_of(blocks)).unwrap();

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
            para("第一章", "他深吸一口气。她微微一笑。"),
            para("第二章", "就在这时，灯灭了。"),
            para("第三章", "两个人都没有再说话。"),
        ]);
        let stats = run_cliche_analysis(&doc).unwrap();

        assert_eq!(stats.hits.len(), 3);
        assert_eq!(stats.hits[0].phrase, "深吸一口气");
        assert_eq!(stats.hits[0].offset, 1);
        assert_eq!(stats.hits[1].phrase, "微微一笑");
        assert_eq!(stats.hits[1].offset, 8);
        assert_eq!(stats.hits[2].section, "第二章");
    }

    #[test]
    fn test_a_phrase_used_across_segments_reaches_both_ends() {
        let doc = doc_of(vec![
            para("第一章", "她微微一笑。"),
            para("第二章", "事情定了下来。"),
            para("第三章", "她也只是微微一笑。"),
        ]);
        let stats = run_cliche_analysis(&doc).unwrap();

        let phrase = &stats.phrases[0];
        assert_eq!(phrase.count, 2);
        assert_eq!(phrase.segments_touched, 2);
        assert_eq!(phrase.first_section, "第一章");
        assert_eq!(phrase.last_section, "第三章");
        assert!((phrase.coverage - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_same_document_yields_the_same_result() {
        let doc = doc_with(&["微微一笑", "深吸一口气", "微微一笑", "就在这时"], 60);
        let a = serde_json::to_string(&run_cliche_analysis(&doc).unwrap()).unwrap();
        let b = serde_json::to_string(&run_cliche_analysis(&doc).unwrap()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn test_pattern_expansion_is_longest_first() {
        let text = |pattern: &str| -> Vec<String> {
            expand(pattern)
                .iter()
                .map(|v| v.iter().collect::<String>())
                .collect()
        };

        assert_eq!(text("(淡淡|浅浅)一笑"), ["浅浅一笑", "淡淡一笑"]);
        // The longer form comes first: that is what makes the scanner take it.
        assert_eq!(
            text("倒吸(了)?一口凉气"),
            ["倒吸了一口凉气", "倒吸一口凉气"]
        );
        assert_eq!(text("顿了顿"), ["顿了顿"]);
    }

    #[test]
    fn test_expansion_is_capped() {
        let variants = expand("(a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t)");
        assert!(variants.len() <= MAX_VARIANTS);
    }
}
