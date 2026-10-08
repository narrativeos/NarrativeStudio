//! 时间线: where the text says when, and whether the dates it gives agree.
//!
//! The dimension scans the running text for time expressions rather than reading the
//! `DATE` entities the import carries. Entities were the obvious source and are the
//! wrong one: an NER pass marks `2003年3月12日` and nothing else, so the whole family of
//! 三天后 / 次年春天 / 那天傍晚 — which is where most fiction actually moves time — is
//! invisible to it, and every expression it does find would be counted twice (once by
//! the entity, once by the scan of the same characters).
//!
//! What it can honestly do is separate four things a reader weighs differently:
//!
//! - **absolute** — a year is present, so the expression resolves to a calendar date and
//!   can be placed on a timeline and compared with the others.
//! - **partial** — a month and day without a year (`3月12日`). Real, but it cannot be
//!   ordered against a date in another year.
//! - **relative** — 三天后, 次日, 去年. It moves the story along and carries an estimated
//!   distance, but it is anchored to the narrating moment or to the sentence before it,
//!   not to any calendar date, so it never takes part in the ordering check.
//! - **vague** — 春天, 傍晚, 那天. A flavour of when, with no position at all.
//!
//! The ordering check reports 时间倒回: a resolvable date earlier in the document lying
//! later in the calendar than the one after it. That is offered as something to look at,
//! never as a fault — a flashback is a legitimate device and the analysis cannot tell the
//! two apart. There is no score: a story with no dated events is not worse than one with a
//! calendar on every page.
//!
//! Deterministic: the scan walks each block once, longest match first, never producing
//! overlapping matches, and every list is in reading order.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use studio_core::document::DocumentData;
use studio_core::entity::EntityCategory;

use crate::segment::plan_segments;
use crate::text::visible_char_count;
use crate::Result;

/// How many characters of context are kept on either side of a marker.
const EXCERPT_RADIUS: usize = 24;
/// A document with fewer visible characters than this is too short for the ordering
/// check to mean anything: two dates in three paragraphs prove nothing.
const MIN_CHARS: u64 = 600;
/// Consecutive segments with no marker at all, at which it becomes worth saying.
const MIN_UNMARKED_SEGMENTS: usize = 3;
/// A forward jump of this many days is called a 大跳跃 rather than just a gap.
const LONG_JUMP_DAYS: i64 = 366;
/// How many of each finding the output keeps.
const MAX_MARKERS: usize = 200;
const MAX_BACKSTEPS: usize = 10;
const MAX_JUMPS: usize = 10;

/// One time expression found in the text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMarker {
    /// The characters found, e.g. `2003年3月12日`.
    pub text: String,
    /// `absolute` | `partial` | `relative` | `vague`.
    pub kind: String,
    /// What the expression pins down: `year` | `month` | `day` | `month_day` | `offset` |
    /// `deictic` | `season` | `time_of_day`.
    pub precision: String,
    /// The segment the marker sits in, on the shared curve axis.
    pub section: String,
    /// Document fraction at the marker (0-1).
    pub position: f64,
    /// The calendar date as `2003-03-12` / `2003-03` / `2003`; empty unless absolute.
    pub resolved: String,
    /// Estimated signed distance in days for a relative marker (月≈30天, 年≈365天);
    /// 0 for anything that is not a distance.
    pub offset_days: f64,
    /// Surrounding characters, with ellipses where they were cut.
    pub excerpt: String,
}

/// One step between two resolvable dates, in reading order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineStep {
    /// Where the earlier mention sits and what date it gave.
    pub from_section: String,
    pub from_date: String,
    /// Where the later mention sits and what date it gave.
    pub to_section: String,
    pub to_date: String,
    /// Days between them: positive forward, negative when the story goes back.
    pub days: i64,
}

/// Something about the document's timekeeping a writer would want to know about.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineIssue {
    /// `no_markers` | `too_short` | `no_order` | `backstep` | `long_jump` |
    /// `unmarked_stretch`.
    pub kind: String,
    pub section: String,
    pub detail: String,
}

/// One segment of the shared curve axis and how many markers it carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePoint {
    pub label: String,
    pub markers: u32,
}

/// Time-line analysis output for one document. There is no score field: the dimension
/// describes how a document keeps time rather than grading how well it does.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineStats {
    /// The expressions found, in reading order.
    pub markers: Vec<TimelineMarker>,
    /// Marker counts per segment, on the same axis as the pacing and conflict curves.
    pub curve: Vec<TimelinePoint>,
    /// How many markers fall in each kind.
    pub absolute_count: u32,
    pub partial_count: u32,
    pub relative_count: u32,
    pub vague_count: u32,
    /// How many `DATE` entities the import carries, which the scan does not read. It is
    /// kept as a cross-check: entities the NER pass marked and the scan could not parse
    /// mean the dates are written in a form outside the recognised range (a reign year,
    /// a lunar date, another language), not that the story has no dates.
    pub entity_date_count: u32,
    /// The resolvable dates, first and last in reading order.
    pub first_date: String,
    pub last_date: String,
    /// Days between the first and last resolvable date; 0 when fewer than two.
    pub span_days: i64,
    /// Steps between consecutive resolvable dates that go backwards in the calendar.
    pub backsteps: Vec<TimelineStep>,
    /// The largest forward steps.
    pub jumps: Vec<TimelineStep>,
    /// Runs of segments carrying no marker at all: (first label, last label, how many).
    pub unmarked_stretches: Vec<(String, String, u32)>,
    pub issues: Vec<TimelineIssue>,
    pub segment_count: u32,
    /// True when the segments are block chunks rather than sections.
    pub chunked: bool,
    pub char_count: u64,
    /// False when the text says nothing about when, or is too short for the comparison
    /// to mean anything.
    pub readable: bool,
    /// Why `readable` is false, in words; empty when it is true.
    pub note: String,
}

/// The value of a single digit glyph, ASCII or CJK.
fn digit_value(c: char) -> Option<u32> {
    match c {
        '0'..='9' => Some(c as u32 - '0' as u32),
        '〇' | '零' => Some(0),
        '一' => Some(1),
        '二' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

/// True for the glyphs that stand for one digit and nothing else.
fn is_digit_glyph(c: char) -> bool {
    matches!(
        c,
        '〇' | '零' | '一' | '二' | '三' | '四' | '五' | '六' | '七' | '八' | '九'
    )
}

/// The value of a count glyph, which additionally allows 两 (`两天`, `两个人`) that a
/// written-out year never uses.
fn count_value(c: char) -> Option<u32> {
    if c == '两' {
        Some(2)
    } else {
        digit_value(c)
    }
}

/// A year: three or four ASCII digits, or three or more CJK digit glyphs written out one
/// by one (`一九九八`, `二〇〇三`).
///
/// Two digits are refused: `98年` is as likely to mean ninety-eight years as 1998, and
/// guessing wrong here puts a story sixty years from where it is. One CJK glyph is
/// refused for the same reason in the other direction — `三年` is a duration, not a year.
fn parse_year(chars: &[char], at: usize) -> Option<(i32, usize)> {
    // Never start in the middle of a number, and never on a leading zero: without these,
    // the `003年` inside a rejected `2003年` would come back as year 3.
    if chars[at] == '0' || (at > 0 && chars[at - 1].is_ascii_digit()) {
        return None;
    }
    let mut ascii = String::new();
    let mut i = at;
    while i < chars.len() && chars[i].is_ascii_digit() {
        ascii.push(chars[i]);
        i += 1;
    }
    if (3..=4).contains(&ascii.len()) {
        return ascii.parse::<i32>().ok().map(|y| (y, i));
    }

    let mut value: i32 = 0;
    let mut digits = 0usize;
    let mut i = at;
    while i < chars.len() && is_digit_glyph(chars[i]) {
        value = value * 10 + digit_value(chars[i]).unwrap_or(0) as i32;
        digits += 1;
        i += 1;
    }
    if digits >= 3 {
        return Some((value, i));
    }
    None
}

/// A small count: ASCII digits, or a CJK numeral up to 99 (`三`, `十`, `十二`, `二十五`).
fn parse_count(chars: &[char], at: usize) -> Option<(u32, usize)> {
    if chars.get(at).is_some_and(|c| c.is_ascii_digit()) {
        let mut number = String::new();
        let mut i = at;
        while i < chars.len() && chars[i].is_ascii_digit() {
            number.push(chars[i]);
            i += 1;
        }
        return number.parse::<u32>().ok().map(|n| (n, i));
    }

    // 十 / 十X / X十 / X十X
    if chars.get(at) == Some(&'十') {
        let tail = match chars.get(at + 1) {
            Some(c) if count_value(*c).is_some() && *c != '十' => count_value(*c)?,
            _ => 0,
        };
        return Some((10 + tail, at + 1 + usize::from(tail > 0)));
    }
    let leading = count_value(*chars.get(at)?)?;
    if chars.get(at + 1) == Some(&'十') {
        let tail = match chars.get(at + 2) {
            Some(c) if count_value(*c).is_some() && *c != '十' => count_value(*c)?,
            _ => 0,
        };
        return Some((leading * 10 + tail, at + 2 + usize::from(tail > 0)));
    }
    Some((leading, at + 1))
}

/// Relative distance units, longest first, with the days one of them stands for.
///
/// 月 and 年 are approximations — 30 and 365 days — because the text rarely says which
/// calendar months it means. The estimate is reported as an estimate, and a relative
/// marker never enters the ordering check, so the rounding cannot corrupt a date.
const UNITS: &[(&str, f64)] = &[
    ("分钟", 1.0 / 1440.0),
    ("小时", 1.0 / 24.0),
    ("秒钟", 1.0 / 86400.0),
    ("星期", 7.0),
    ("礼拜", 7.0),
    ("个月", 30.0),
    ("月", 30.0),
    ("天", 1.0),
    ("日", 1.0),
    ("周", 7.0),
    ("年", 365.0),
];

/// Words that place an expression after or before the moment it is anchored to.
const FORWARD: &[&str] = &["之后", "以后", "后"];
const BACKWARD: &[&str] = &["之前", "以前", "前"];

/// Fixed relative words: the distance is implied by the word itself. Their anchor is the
/// narrating moment or the sentence before, never a calendar date, so they are listed and
/// counted but never ordered.
const DEICTIC: &[(&str, f64)] = &[
    ("第二天", 1.0),
    ("前一天", -1.0),
    ("上个月", -30.0),
    ("半个月", 15.0),
    ("今天", 0.0),
    ("明天", 1.0),
    ("次日", 1.0),
    ("翌日", 1.0),
    ("隔天", 1.0),
    ("后天", 2.0),
    ("昨天", -1.0),
    ("昨日", -1.0),
    ("前天", -2.0),
    ("今年", 0.0),
    ("明年", 365.0),
    ("次年", 365.0),
    ("去年", -365.0),
    ("上月", -30.0),
    ("本月", 0.0),
    ("下周", 7.0),
    ("上周", -7.0),
];

/// Vague time words: they say roughly when, and nothing about where.
const VAGUE: &[(&str, &str)] = &[
    ("春天", "season"),
    ("春季", "season"),
    ("初春", "season"),
    ("早春", "season"),
    ("夏天", "season"),
    ("夏季", "season"),
    ("盛夏", "season"),
    ("秋天", "season"),
    ("秋季", "season"),
    ("深秋", "season"),
    ("冬天", "season"),
    ("冬季", "season"),
    ("初冬", "season"),
    ("隆冬", "season"),
    ("凌晨", "time_of_day"),
    ("清晨", "time_of_day"),
    ("早晨", "time_of_day"),
    ("早上", "time_of_day"),
    ("上午", "time_of_day"),
    ("中午", "time_of_day"),
    ("正午", "time_of_day"),
    ("午后", "time_of_day"),
    ("下午", "time_of_day"),
    ("傍晚", "time_of_day"),
    ("黄昏", "time_of_day"),
    ("入夜", "time_of_day"),
    ("夜里", "time_of_day"),
    ("夜晚", "time_of_day"),
    ("深夜", "time_of_day"),
    ("午夜", "time_of_day"),
    ("半夜", "time_of_day"),
    ("多年以后", "deictic"),
    ("很久以后", "deictic"),
    ("许多年后", "deictic"),
    ("若干年后", "deictic"),
    ("片刻之后", "deictic"),
    ("与此同时", "deictic"),
    ("同一时间", "deictic"),
    ("那天", "deictic"),
    ("当天", "deictic"),
    ("那年", "deictic"),
    ("那晚", "deictic"),
    ("此刻", "deictic"),
    ("此时", "deictic"),
    ("当时", "deictic"),
];

/// One expression found inside a block, before it is placed in the document.
struct Found {
    start: usize,
    end: usize,
    kind: &'static str,
    precision: &'static str,
    /// The date as written; empty unless the expression carries a year.
    resolved: String,
    /// Set only when the date is resolved to at least a month. A bare year is left out
    /// on purpose: `2003年` followed by `2003年6月` would read as going backwards, and
    /// nothing in the text says which month that bare year meant.
    order_key: Option<NaiveDate>,
    offset_days: f64,
}

/// A word table bucketed by first character, longest candidate first, so a pass over a
/// novel stays linear instead of trying every word at every position.
type Buckets = HashMap<char, Vec<(usize, Vec<char>)>>;

fn build_buckets<T>(table: &[(&'static str, T)]) -> Buckets {
    let mut buckets: Buckets = HashMap::new();
    for (idx, (word, _)) in table.iter().enumerate() {
        let chars: Vec<char> = word.chars().collect();
        if let Some(first) = chars.first() {
            buckets.entry(*first).or_default().push((idx, chars));
        }
    }
    for entries in buckets.values_mut() {
        entries.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    }
    buckets
}

/// True when `word` sits at `at`.
fn starts_with(chars: &[char], at: usize, word: &[char]) -> bool {
    word.iter()
        .enumerate()
        .all(|(i, c)| chars.get(at + i) == Some(c))
}

/// The longest bucket entry sitting at `at`, as (table index, end offset).
fn match_buckets(buckets: &Buckets, chars: &[char], at: usize) -> Option<(usize, usize)> {
    let entries = buckets.get(&chars[at])?;
    for (idx, word) in entries {
        if starts_with(chars, at, word) {
            return Some((*idx, at + word.len()));
        }
    }
    None
}

/// The direction word at `at`, if any: (sign, end offset). Longest wins.
fn match_direction(chars: &[char], at: usize) -> Option<(f64, usize)> {
    let mut best: Option<(f64, usize)> = None;
    let words = FORWARD
        .iter()
        .map(|w| (*w, 1.0f64))
        .chain(BACKWARD.iter().map(|w| (*w, -1.0f64)));
    for (word, sign) in words {
        let wc: Vec<char> = word.chars().collect();
        if starts_with(chars, at, &wc) {
            let end = at + wc.len();
            if best.is_none_or(|(_, prev)| end > prev) {
                best = Some((sign, end));
            }
        }
    }
    best
}

/// A dated expression: `2003年3月12日`, `2003年3月`, `2003年`, `二〇〇三年三月`,
/// `2003-03-12`.
fn match_absolute(chars: &[char], at: usize) -> Option<Found> {
    let (year, after_year) = parse_year(chars, at)?;

    // The separator form is only read in full: `2003-03` on its own is as likely to be
    // a range or a version number as a month.
    if matches!(chars.get(after_year), Some('-') | Some('/')) {
        let sep = chars[after_year];
        let (month, after_month_sep) = parse_count(chars, after_year + 1)?;
        if chars.get(after_month_sep) != Some(&sep) {
            return None;
        }
        let (day, end) = parse_count(chars, after_month_sep + 1)?;
        let date = NaiveDate::from_ymd_opt(year, month, day)?;
        return Some(Found {
            start: at,
            end,
            kind: "absolute",
            precision: "day",
            resolved: format!("{year:04}-{month:02}-{day:02}"),
            order_key: Some(date),
            offset_days: 0.0,
        });
    }

    if chars.get(after_year) != Some(&'年') {
        return None;
    }
    let mut end = after_year + 1;
    let mut month: Option<u32> = None;
    let mut day: Option<u32> = None;
    if let Some((m, at_month)) = parse_count(chars, end) {
        if chars.get(at_month) == Some(&'月') && (1..=12).contains(&m) {
            month = Some(m);
            end = at_month + 1;
            if let Some((d, at_day)) = parse_count(chars, end) {
                if matches!(chars.get(at_day), Some('日') | Some('号')) && (1..=31).contains(&d) {
                    day = Some(d);
                    end = at_day + 1;
                }
            }
        }
    }

    // A date written down to the day still has to be a real one: 2月30日 is refused
    // outright rather than rounded, because a wrong date in the report is worse than a
    // missed one.
    NaiveDate::from_ymd_opt(year, month.unwrap_or(1), day.unwrap_or(1))?;
    let (precision, resolved, order_key) = match (month, day) {
        (Some(m), Some(d)) => (
            "day",
            format!("{year:04}-{m:02}-{d:02}"),
            NaiveDate::from_ymd_opt(year, m, d),
        ),
        (Some(m), None) => (
            "month",
            format!("{year:04}-{m:02}"),
            NaiveDate::from_ymd_opt(year, m, 1),
        ),
        _ => ("year", format!("{year:04}"), None),
    };
    Some(Found {
        start: at,
        end,
        kind: "absolute",
        precision,
        resolved,
        order_key,
        offset_days: 0.0,
    })
}

/// `3月12日` — a spot inside a year with no year attached to it.
fn match_month_day(chars: &[char], at: usize) -> Option<Found> {
    let (month, at_month) = parse_count(chars, at)?;
    if !(1..=12).contains(&month) || chars.get(at_month) != Some(&'月') {
        return None;
    }
    let (day, at_day) = parse_count(chars, at_month + 1)?;
    if !(1..=31).contains(&day) || !matches!(chars.get(at_day), Some('日') | Some('号')) {
        return None;
    }
    // A day no calendar can hold is refused even without a year: 2月30日 is a slip in the
    // text, and repeating it in the report would only dress it up as a finding. The
    // reference year is a leap year, so 2月29日 stays legal.
    NaiveDate::from_ymd_opt(2000, month, day)?;
    Some(Found {
        start: at,
        end: at_day + 1,
        kind: "partial",
        precision: "month_day",
        resolved: String::new(),
        order_key: None,
        offset_days: 0.0,
    })
}

/// `三天后`, `两个月前`, `半小时以后`.
fn match_relative(chars: &[char], at: usize, units: &Buckets) -> Option<Found> {
    let (count, after_count) = if chars.get(at) == Some(&'半') {
        (0.5f64, at + 1)
    } else {
        let (n, end) = parse_count(chars, at)?;
        (n as f64, end)
    };
    let (unit_idx, after_unit) = match_buckets(units, chars, after_count)?;
    let (sign, end) = match_direction(chars, after_unit)?;
    Some(Found {
        start: at,
        end,
        kind: "relative",
        precision: "offset",
        resolved: String::new(),
        order_key: None,
        offset_days: sign * count * UNITS[unit_idx].1,
    })
}

/// `3月` on its own. `三个月` is a duration rather than a month and cannot reach here:
/// the 个 after the count means there is no 月 in the place this matcher looks for one.
fn match_month_only(chars: &[char], at: usize) -> Option<Found> {
    let (month, at_month) = parse_count(chars, at)?;
    if !(1..=12).contains(&month) || chars.get(at_month) != Some(&'月') {
        return None;
    }
    // `N月` followed by `D日` is one month-and-day expression, and that matcher already
    // had its chance: reporting the month alone would resurrect half of a rejected date.
    if let Some((_, at_day)) = parse_count(chars, at_month + 1) {
        if matches!(chars.get(at_day), Some('日') | Some('号')) {
            return None;
        }
    }
    Some(Found {
        start: at,
        end: at_month + 1,
        kind: "partial",
        precision: "month",
        resolved: String::new(),
        order_key: None,
        offset_days: 0.0,
    })
}

/// `12日` — a day of a month the reader already knows. ASCII digits only: `六日` is just
/// as likely to be six days as the sixth.
fn match_day_only(chars: &[char], at: usize) -> Option<Found> {
    // A number right after a month marker is the day of that date. If the date was
    // rejected — 2月30日 is — the leftover `30日` must not come back as a finding.
    if at > 0 && chars[at - 1] == '月' {
        return None;
    }
    let (day, at_day) = parse_count(chars, at)?;
    if !(1..=31).contains(&day)
        || !chars[at..at_day].iter().all(char::is_ascii_digit)
        || !matches!(chars.get(at_day), Some('日') | Some('号'))
    {
        return None;
    }
    Some(Found {
        start: at,
        end: at_day + 1,
        kind: "partial",
        precision: "day",
        resolved: String::new(),
        order_key: None,
        offset_days: 0.0,
    })
}

/// The three word tables, bucketed once per document.
struct Lexicons {
    units: Buckets,
    deictic: Buckets,
    vague: Buckets,
}

/// Surrounding characters for a match, with ellipses where they were cut.
fn excerpt(chars: &[char], start: usize, end: usize) -> String {
    let from = start.saturating_sub(EXCERPT_RADIUS);
    let to = (end + EXCERPT_RADIUS).min(chars.len());
    let mut out = String::new();
    if from > 0 {
        out.push('…');
    }
    out.extend(chars[from..to].iter());
    if to < chars.len() {
        out.push('…');
    }
    out
}

/// Every time expression in one block, in reading order, never overlapping.
///
/// The order the alternatives are tried in is the order they are more specific: a full
/// date before a month-and-day before a bare month, and a relative expression before the
/// bare unit it is built from, so `三个月后` reads as one distance rather than as a month
/// plus a stray 后.
fn scan_block(chars: &[char], lex: &Lexicons) -> Vec<Found> {
    let mut found: Vec<Found> = Vec::new();
    let mut pos = 0;
    while pos < chars.len() {
        let numeric = match_absolute(chars, pos)
            .or_else(|| match_month_day(chars, pos))
            .or_else(|| match_relative(chars, pos, &lex.units))
            .or_else(|| match_month_only(chars, pos))
            .or_else(|| match_day_only(chars, pos));
        if let Some(f) = numeric {
            pos = f.end.max(pos + 1);
            found.push(f);
            continue;
        }
        if let Some((idx, end)) = match_buckets(&lex.deictic, chars, pos) {
            found.push(Found {
                start: pos,
                end,
                kind: "relative",
                precision: "deictic",
                resolved: String::new(),
                order_key: None,
                offset_days: DEICTIC[idx].1,
            });
            pos = end;
            continue;
        }
        if let Some((idx, end)) = match_buckets(&lex.vague, chars, pos) {
            found.push(Found {
                start: pos,
                end,
                kind: "vague",
                precision: VAGUE[idx].1,
                resolved: String::new(),
                order_key: None,
                offset_days: 0.0,
            });
            pos = end;
            continue;
        }
        pos += 1;
    }
    found
}

/// Record one run of empty segments, if it is long enough to be worth saying.
fn record_stretch(
    out: &mut Vec<(String, String, u32)>,
    curve: &[TimelinePoint],
    start: usize,
    end: usize,
) {
    if end + 1 - start >= MIN_UNMARKED_SEGMENTS && start < curve.len() && end < curve.len() {
        out.push((
            curve[start].label.clone(),
            curve[end].label.clone(),
            (end - start + 1) as u32,
        ));
    }
}

/// Analyse how a document keeps time.
pub fn run_timeline_analysis(doc: &DocumentData) -> Result<TimelineStats> {
    let plan = plan_segments(doc);
    let char_count: u64 = doc
        .blocks
        .iter()
        .map(|b| visible_char_count(&b.content) as u64)
        .sum();
    // What the import's NER pass thought were dates. The scan does not read these, but
    // the gap between the two is itself worth reporting.
    let entity_date_count = doc
        .blocks
        .iter()
        .flat_map(|b| b.entities.iter())
        .filter(|e| e.category == EntityCategory::Date)
        .count() as u32;
    let lex = Lexicons {
        units: build_buckets(UNITS),
        deictic: build_buckets(DEICTIC),
        vague: build_buckets(VAGUE),
    };

    let mut markers: Vec<TimelineMarker> = Vec::new();
    // The dates that can be placed on a calendar: (segment label, as written, key).
    let mut dated: Vec<(String, String, NaiveDate)> = Vec::new();
    let mut per_segment = vec![0u32; plan.labels.len()];
    let mut cursor: u64 = 0;

    for (block_idx, block) in doc.blocks.iter().enumerate() {
        let chars: Vec<char> = block.content.chars().collect();
        let block_chars = visible_char_count(&block.content) as u64;
        let segment = plan.segment_of.get(block_idx).copied();
        let label = segment.map_or_else(String::new, |s| plan.labels[s].clone());
        for f in scan_block(&chars, &lex) {
            if markers.len() >= MAX_MARKERS {
                break;
            }
            if let Some(s) = segment {
                per_segment[s] += 1;
            }
            if let Some(key) = f.order_key {
                dated.push((label.clone(), f.resolved.clone(), key));
            }
            markers.push(TimelineMarker {
                text: chars[f.start..f.end].iter().collect(),
                kind: f.kind.to_string(),
                precision: f.precision.to_string(),
                section: label.clone(),
                position: if char_count > 0 {
                    (cursor + f.start as u64) as f64 / char_count as f64
                } else {
                    0.0
                },
                resolved: f.resolved,
                offset_days: f.offset_days,
                excerpt: excerpt(&chars, f.start, f.end),
            });
        }
        cursor += block_chars;
    }

    let curve: Vec<TimelinePoint> = plan
        .labels
        .iter()
        .zip(per_segment.iter())
        .map(|(label, n)| TimelinePoint {
            label: label.clone(),
            markers: *n,
        })
        .collect();

    let count_kind = |kind: &str| markers.iter().filter(|m| m.kind == kind).count() as u32;
    let mut unmarked_stretches: Vec<(String, String, u32)> = Vec::new();
    let mut run_start: Option<usize> = None;
    for (i, point) in curve.iter().enumerate() {
        if point.markers == 0 {
            run_start.get_or_insert(i);
        } else if let Some(start) = run_start.take() {
            record_stretch(&mut unmarked_stretches, &curve, start, i - 1);
        }
    }
    if let Some(start) = run_start {
        let last = curve.len().saturating_sub(1);
        record_stretch(&mut unmarked_stretches, &curve, start, last);
    }

    let readable = !markers.is_empty() && char_count >= MIN_CHARS;
    let note = if markers.is_empty() && entity_date_count > 0 {
        format!("没有读到可解析的时间表达，而导入数据标了 {entity_date_count} 处日期")
    } else if markers.is_empty() {
        "全文没有读到任何时间表达".to_string()
    } else if char_count < MIN_CHARS {
        format!("可见文字不足 {MIN_CHARS} 字：日期太少，排序不下结论")
    } else {
        String::new()
    };

    let mut stats = TimelineStats {
        absolute_count: count_kind("absolute"),
        partial_count: count_kind("partial"),
        relative_count: count_kind("relative"),
        vague_count: count_kind("vague"),
        entity_date_count,
        markers,
        curve,
        unmarked_stretches: unmarked_stretches.clone(),
        first_date: String::new(),
        last_date: String::new(),
        span_days: 0,
        backsteps: Vec::new(),
        jumps: Vec::new(),
        issues: Vec::new(),
        segment_count: plan.labels.len() as u32,
        chunked: plan.chunked,
        char_count,
        readable,
        note,
    };
    build_findings(&mut stats, &dated, &unmarked_stretches);
    Ok(stats)
}

/// Turn the scan into findings: what the calendar says about the reading order, and —
/// said just as plainly — what cannot be said at all.
fn build_findings(
    stats: &mut TimelineStats,
    dated: &[(String, String, NaiveDate)],
    stretches: &[(String, String, u32)],
) {
    let mut issues: Vec<TimelineIssue> = Vec::new();
    if stats.markers.is_empty() {
        let detail = if stats.entity_date_count > 0 {
            format!(
                "扫描没有读到可解析的时间表达，可导入的实体标注里有 {} 处日期：写法不在识别范围内（年号、农历、外文日期都可能）",
                stats.entity_date_count
            )
        } else {
            "全文没有读到任何时间表达：这份文本不靠日期推进".into()
        };
        issues.push(TimelineIssue {
            kind: "no_markers".into(),
            section: String::new(),
            detail,
        });
        // With nothing found, every segment is empty by definition; saying so segment by
        // segment would only restate the line above.
        stats.unmarked_stretches.clear();
        stats.issues = issues;
        return;
    }
    if !stats.readable {
        issues.push(TimelineIssue {
            kind: "too_short".into(),
            section: String::new(),
            detail: stats.note.clone(),
        });
        stats.issues = issues;
        return;
    }

    let mut backsteps: Vec<TimelineStep> = Vec::new();
    let mut jumps: Vec<TimelineStep> = Vec::new();
    if dated.len() >= 2 {
        let (_, from_date, from_key) = dated.first().expect("checked for length");
        let (_, to_date, to_key) = dated.last().expect("checked for length");
        stats.first_date = from_date.clone();
        stats.last_date = to_date.clone();
        stats.span_days = to_key.signed_duration_since(*from_key).num_days();

        for pair in dated.windows(2) {
            let days = pair[1].2.signed_duration_since(pair[0].2).num_days();
            if days == 0 {
                continue;
            }
            let step = TimelineStep {
                from_section: pair[0].0.clone(),
                from_date: pair[0].1.clone(),
                to_section: pair[1].0.clone(),
                to_date: pair[1].1.clone(),
                days,
            };
            if days < 0 {
                backsteps.push(step);
            } else if days >= LONG_JUMP_DAYS {
                jumps.push(step);
            }
        }
        backsteps.truncate(MAX_BACKSTEPS);
        jumps.sort_by(|a, b| {
            b.days.cmp(&a.days).then_with(|| {
                a.from_date
                    .cmp(&b.from_date)
                    .then_with(|| a.to_date.cmp(&b.to_date))
            })
        });
        jumps.truncate(MAX_JUMPS);
    }

    for step in &backsteps {
        issues.push(TimelineIssue {
            kind: "backstep".into(),
            section: step.to_section.clone(),
            detail: format!(
                "「{}」的 {} 之后，「{}」又出现 {}，早了 {} 天：倒叙还是笔误，文本分不出",
                step.from_section, step.from_date, step.to_section, step.to_date, -step.days
            ),
        });
    }
    for step in &jumps {
        issues.push(TimelineIssue {
            kind: "long_jump".into(),
            section: step.to_section.clone(),
            detail: format!(
                "{} → {}：跳过 {:.1} 年，中间没有任何时间标记接手",
                step.from_date,
                step.to_date,
                step.days as f64 / 365.0
            ),
        });
    }
    for (from, to, count) in stretches {
        issues.push(TimelineIssue {
            kind: "unmarked_stretch".into(),
            section: from.clone(),
            detail: format!(
                "「{from}」到「{to}」共 {count} 段没有一处时间标记；没有标记不等于时间没有过去"
            ),
        });
    }
    if dated.len() < 2 {
        let detail = if stats.absolute_count == 0 {
            let base = format!(
                "{} 处时间表达没有一处给出年份，先后无法判断",
                stats.markers.len()
            );
            if stats.entity_date_count > 0 {
                format!(
                    "{base}；另有 {} 处日期实体写法不在识别范围内",
                    stats.entity_date_count
                )
            } else {
                base
            }
        } else {
            format!(
                "{} 处日期没有一处写到月份，粒度太粗，不参与排序",
                stats.absolute_count
            )
        };
        issues.push(TimelineIssue {
            kind: "no_order".into(),
            section: String::new(),
            detail,
        });
    }

    stats.issues = issues;
    stats.backsteps = backsteps;
    stats.jumps = jumps;
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

    fn para(path: &str, body: &str) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: body.into(),
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

    /// Scan one sentence on its own: the parser questions are about the parser, and
    /// wrapping them in a document only adds noise.
    fn scan(text: &str) -> Vec<Found> {
        let lex = Lexicons {
            units: build_buckets(UNITS),
            deictic: build_buckets(DEICTIC),
            vague: build_buckets(VAGUE),
        };
        scan_block(&text.chars().collect::<Vec<char>>(), &lex)
    }

    /// The filler carries no time word of its own, so a test that needs length does not
    /// accidentally add markers.
    const FILLER: &str = "他把笔记合上，重新读了一遍那行字，又抬头看窗外。";

    /// Enough text to clear the length floor, with `lead` at the front.
    fn long_body(lead: &str) -> String {
        let mut body = String::from(lead);
        while visible_char_count(&body) < 700 {
            body.push_str(FILLER);
        }
        body
    }

    #[test]
    fn test_a_full_date_is_read_to_the_day() {
        let found = scan("2003年3月12日清晨，他到了北岸。");
        assert_eq!(found.len(), 2, "the date and the time of day");
        assert_eq!(found[0].kind, "absolute");
        assert_eq!(found[0].precision, "day");
        assert_eq!(found[0].resolved, "2003-03-12");
        assert_eq!(found[1].precision, "time_of_day");
    }

    #[test]
    fn test_a_written_out_year_is_read() {
        let found = scan("故事开始于一九九八年三月。");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].resolved, "1998-03");
        assert_eq!(found[0].precision, "month");
    }

    #[test]
    fn test_a_two_digit_year_is_refused() {
        // `98年` is as likely to be ninety-eight years as 1998. Guessing puts the whole
        // timeline sixty years out.
        assert!(scan("那是98年的事").is_empty());
    }

    #[test]
    fn test_a_bare_duration_is_not_a_time_marker() {
        assert!(scan("他等了三年，又等了三个月。").is_empty());
    }

    #[test]
    fn test_an_impossible_date_is_refused_rather_than_repaired() {
        // Neither the whole expression nor a fragment of it survives.
        assert!(scan("落款是2003年2月30日").is_empty());
    }

    #[test]
    fn test_a_month_and_day_without_a_year_is_partial() {
        let found = scan("3月12日那天下了雪。");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].kind, "partial");
        assert_eq!(found[0].precision, "month_day");
        assert!(found[0].resolved.is_empty());
        assert!(found[0].order_key.is_none());
    }

    #[test]
    fn test_an_iso_date_is_read() {
        let found = scan("档案编号 2003-03-12 号");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].resolved, "2003-03-12");
    }

    #[test]
    fn test_relative_distances_are_estimated() {
        let found = scan("三天后他回来了，两个月前他们还见过面，半小时以后门关上了。");
        let offsets: Vec<f64> = found.iter().map(|f| f.offset_days).collect();
        // The estimate is in days, with 月 rounded to 30 and 半小时 to half an hour.
        assert!((offsets[0] - 3.0).abs() < 1e-9);
        assert!((offsets[1] + 60.0).abs() < 1e-9);
        assert!((offsets[2] - 1.0 / 48.0).abs() < 1e-9);
        assert!(found.iter().all(|f| f.kind == "relative"));
        assert!(found.iter().all(|f| f.order_key.is_none()));
    }

    #[test]
    fn test_a_day_number_needs_digits() {
        // `六日` is as likely to be six days as the sixth; `12日` can only be a date.
        let written = scan("六日之后他走了");
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].kind, "relative");
        assert_eq!(written[0].offset_days, 6.0);
        let numbered = scan("12日他走了");
        assert_eq!(numbered[0].precision, "day");
    }

    #[test]
    fn test_vague_words_are_found_and_positionless() {
        let found = scan("那天傍晚，冬天来了。");
        let kinds: Vec<&str> = found.iter().map(|f| f.precision).collect();
        assert_eq!(kinds, vec!["deictic", "time_of_day", "season"]);
        assert!(found.iter().all(|f| f.kind == "vague"));
    }

    #[test]
    fn test_deictic_words_are_relative() {
        let found = scan("第二天他走了，去年也是这里。");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].precision, "deictic");
        assert_eq!(found[0].offset_days, 1.0);
        assert_eq!(found[1].offset_days, -365.0);
    }

    #[test]
    fn test_dates_that_go_backwards_are_reported_once_each() {
        let doc = doc_of(vec![
            para("第一章", &long_body("2011年5月1日，他回来了。")),
            para("第二章", &long_body("1998年3月2日，他还没出生。")),
        ]);
        let stats = run_timeline_analysis(&doc).unwrap();
        assert_eq!(stats.backsteps.len(), 1);
        assert_eq!(stats.backsteps[0].from_date, "2011-05-01");
        assert_eq!(stats.backsteps[0].to_date, "1998-03-02");
        assert!(stats.backsteps[0].days < 0);
        let issue = stats
            .issues
            .iter()
            .find(|i| i.kind == "backstep")
            .expect("the backstep is also an issue");
        // The wording refuses to decide which kind of backwards it is.
        assert!(issue.detail.contains("倒叙"));
    }

    #[test]
    fn test_a_bare_year_is_not_compared_with_a_dated_one() {
        let doc = doc_of(vec![
            para("上部", &long_body("2003年，他开始记账。")),
            para("上部", &long_body("2003年6月，账本丢了。")),
        ]);
        let stats = run_timeline_analysis(&doc).unwrap();
        assert_eq!(stats.absolute_count, 2);
        assert!(stats.backsteps.is_empty(), "a bare year is not a step back");
        // Only the month-precision date can be placed, and one date is not an ordering.
        assert!(stats.first_date.is_empty());
        assert!(stats.issues.iter().any(|i| i.kind == "no_order"));
    }

    #[test]
    fn test_a_long_jump_is_reported() {
        let doc = doc_of(vec![
            para("上部", &long_body("1990年1月1日，下雪。")),
            para("上部", &long_body("2003年6月1日，同一个窗口。")),
        ]);
        let stats = run_timeline_analysis(&doc).unwrap();
        assert_eq!(stats.jumps.len(), 1);
        assert!(stats.jumps[0].days > 4800);
        assert_eq!(stats.span_days, stats.jumps[0].days);
        assert_eq!(stats.first_date, "1990-01-01");
        assert_eq!(stats.last_date, "2003-06-01");
        assert!(stats
            .issues
            .iter()
            .any(|i| i.kind == "long_jump" && i.detail.contains("跳过")));
    }

    #[test]
    fn test_the_curve_shares_the_section_axis() {
        let doc = doc_of(vec![
            para("第一章", &long_body("1990年1月1日，下雪。")),
            para("第二章", &long_body("")),
            para("第三章", &long_body("2003年6月1日，同一个窗口。")),
        ]);
        let stats = run_timeline_analysis(&doc).unwrap();
        assert_eq!(stats.curve.len(), 3);
        assert_eq!(stats.curve[0].label, "第一章");
        assert_eq!(stats.curve[0].markers, 1);
        assert_eq!(stats.curve[1].markers, 0);
        assert!(!stats.chunked);
    }

    #[test]
    fn test_a_run_of_quiet_sections_is_named() {
        let doc = doc_of(vec![
            para("第一章", &long_body("1990年1月1日，下雪。")),
            para("第二章", &long_body("")),
            para("第三章", &long_body("")),
            para("第四章", &long_body("")),
            para("第五章", &long_body("2003年6月1日，同一个窗口。")),
        ]);
        let stats = run_timeline_analysis(&doc).unwrap();
        assert_eq!(
            stats.unmarked_stretches,
            vec![("第二章".to_string(), "第四章".to_string(), 3)]
        );
        // The detail says what the count cannot: a quiet stretch is not an empty one.
        let issue = stats
            .issues
            .iter()
            .find(|i| i.kind == "unmarked_stretch")
            .expect("a three-segment stretch is worth saying");
        assert!(issue.detail.contains("不等于"));
    }

    #[test]
    fn test_relative_markers_never_enter_the_ordering() {
        let doc = doc_of(vec![para(
            "上部",
            &long_body("第二天他走了，去年也是这里。"),
        )]);
        let stats = run_timeline_analysis(&doc).unwrap();
        assert_eq!(stats.relative_count, 2);
        assert!(stats.backsteps.is_empty());
        assert!(stats.jumps.is_empty());
        assert!(stats.span_days == 0);
        assert!(stats.issues.iter().any(|i| i.kind == "no_order"));
    }

    #[test]
    fn test_positions_run_with_the_reading_order() {
        let doc = doc_of(vec![
            para("上部", &long_body("1990年1月1日，下雪。")),
            para("上部", &long_body("2003年6月1日，同一个窗口。")),
        ]);
        let stats = run_timeline_analysis(&doc).unwrap();
        let positions: Vec<f64> = stats.markers.iter().map(|m| m.position).collect();
        assert_eq!(positions.len(), 2);
        assert!(positions[0] < positions[1]);
        assert!(positions.iter().all(|p| (0.0..1.0).contains(p)));
    }

    #[test]
    fn test_a_document_without_time_expressions_says_so() {
        let doc = doc_of(vec![para("第一章", &long_body(""))]);
        let stats = run_timeline_analysis(&doc).unwrap();
        assert!(stats.markers.is_empty());
        assert!(!stats.readable);
        assert_eq!(stats.issues.len(), 1);
        assert_eq!(stats.issues[0].kind, "no_markers");
        assert!(stats.note.contains("没有读到"));
    }

    #[test]
    fn test_a_short_document_withholds_the_ordering() {
        let doc = doc_of(vec![para("第一章", "2011年5月1日。1998年3月2日。")]);
        let stats = run_timeline_analysis(&doc).unwrap();
        assert_eq!(stats.absolute_count, 2);
        assert!(!stats.readable);
        assert!(
            stats.backsteps.is_empty(),
            "two dates in one line prove nothing"
        );
        assert_eq!(stats.issues[0].kind, "too_short");
    }

    /// A block carrying a date the NER pass marked and the scan cannot read.
    fn para_with_entity(path: &str, body: &str, entity: &str) -> SemanticBlock {
        let mut block = para(path, body);
        block.entities.push(studio_core::entity::Entity {
            text: entity.into(),
            category: EntityCategory::Date,
            confidence: 0.9,
            source: "test".into(),
            keep: true,
            filter: None,
            filter_reason: None,
            span: (0, 4),
        });
        block
    }

    #[test]
    fn test_unreadable_dates_are_not_called_an_absence_of_dates() {
        // The import saw dates the scan could not read. Claiming the story carries no
        // dates would contradict the data sitting in the same document.
        let doc = doc_of(vec![para_with_entity(
            "第一章",
            &long_body(""),
            "昭和五十八年",
        )]);
        let stats = run_timeline_analysis(&doc).unwrap();
        assert_eq!(stats.entity_date_count, 1);
        assert_eq!(stats.issues[0].kind, "no_markers");
        assert!(stats.issues[0].detail.contains("实体标注"));
        assert!(!stats.issues[0].detail.contains("不靠日期推进"));
        assert!(stats.note.contains("导入数据"));
    }
}
