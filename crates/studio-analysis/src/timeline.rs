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
//! The lexicon reaches past `2003年3月12日`, because a Chinese text keeps time in more than
//! one calendar, and each of those needs a different promise:
//!
//! - 年号 and 民国 years (`乾隆三年`, `民国三十八年`) come out of a table of when each era
//!   was proclaimed, and resolve to a Gregorian year — and no finer, because an era was
//!   proclaimed in the middle of a year.
//! - 干支 years (`庚子年`) are a position in a 60-year wheel, not a year. They are placed
//!   only against dates the document itself already gives, and stay unplaced when that
//!   still leaves more than one answer.
//! - 农历 dates (`腊月初八`, `癸卯年八月十五`) are converted with an astronomical calendar
//!   engine. A month and day that no calendar in that year held is refused rather than
//!   repaired.
//! - 节气, 节日, 时辰 and weekday words are counted as vague: they say roughly when, and
//!   the same characters can be a person's name.
//!
//! The ordering check reports 时间倒回: a resolvable date earlier in the document lying
//! later in the calendar than the one after it. That is offered as something to look at,
//! never as a fault — a flashback is a legitimate device and the analysis cannot tell the
//! two apart. There is no score: a story with no dated events is not worse than one with a
//! calendar on every page.
//!
//! Deterministic: the scan walks each block once, longest match first, never producing
//! overlapping matches, and every list is in reading order.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use studio_core::document::DocumentData;
use studio_core::entity::EntityCategory;

use crate::calendar;
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
    /// `deictic` | `season` | `time_of_day` | `weekday` | `solar_term` | `festival` |
    /// `era_year` | `era_day` | `ganzhi_year` | `ganzhi_day` | `lunar_month` |
    /// `lunar_month_day` | `lunar_day`.
    pub precision: String,
    /// The segment the marker sits in, on the shared curve axis.
    pub section: String,
    /// Document fraction at the marker (0-1).
    pub position: f64,
    /// The calendar date as `2003-03-12` / `2003-03` / `2003`; a 年号 or 干支 year as the
    /// Gregorian year it resolves to (`乾隆三年` → `1738`); empty unless the expression
    /// carries a year the analysis could place.
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
    /// `unmarked_stretch` | `unanchored_ganzhi` | `ambiguous_era` | `calendar_note` |
    /// `term_collision`.
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
    /// How the expressions are written, most common first: `day`, `month_day`, `offset`,
    /// `season`, and the calendar-table forms — `era_year`, `ganzhi_year`,
    /// `lunar_month_day`, `solar_term`, `festival`. A form is listed only when the text
    /// really used it.
    pub forms: Vec<(String, u32)>,
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

/// A 干支 year the scan cannot place yet: a position in the 60-year wheel, plus the lunar
/// month and day that followed it, if any. The document's own dates anchor it later.
struct GanzhiYear {
    index: usize,
    lunar: Option<(u32, u32, bool)>,
}

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
    /// A 干支 year waiting for an anchor.
    ganzhi: Option<GanzhiYear>,
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
///
/// Bounds-safe: a caller that has just consumed a count word can hand over the offset one
/// past the text, and a word table tried at nothing matches nothing.
fn match_buckets(buckets: &Buckets, chars: &[char], at: usize) -> Option<(usize, usize)> {
    let entries = buckets.get(chars.get(at)?)?;
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
            ganzhi: None,
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
        ganzhi: None,
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
        ganzhi: None,
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
        ganzhi: None,
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
        ganzhi: None,
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
        ganzhi: None,
    })
}

/// A month and day written right after a named year: `万历十五年三月十二`,
/// `民国三十八年三月五日`.
///
/// Whether they are lunar is read off how they are written, which is the convention the
/// writing itself keeps: 初三 / 廿三 / 十五 name a lunar day, a day carrying 日 or 号 is a
/// solar one, and 正月 / 冬月 / 腊月 / 闰X月 are lunar names whatever follows them.
struct TrailingDate {
    month: u32,
    day: u32,
    leap: bool,
    lunar: bool,
    end: usize,
}

fn match_trailing_date(chars: &[char], at: usize) -> Option<TrailingDate> {
    let (month, leap, after_month) = match calendar::lunar_month_name(chars, at) {
        Some((month, leap, end)) => (month, leap, end),
        None => {
            let (month, after) = calendar::small_count(chars, at)?;
            if !(1..=12).contains(&month) || chars.get(after) != Some(&'月') {
                return None;
            }
            (month, false, after + 1)
        }
    };
    if let Some((day, end)) = calendar::lunar_day_name(chars, after_month) {
        return Some(TrailingDate {
            month,
            day,
            leap,
            lunar: true,
            end,
        });
    }
    let (day, end) = calendar::small_count(chars, after_month)?;
    if (1..=31).contains(&day) && matches!(chars.get(end), Some('日') | Some('号')) {
        return Some(TrailingDate {
            month,
            day,
            leap,
            lunar: false,
            end: end + 1,
        });
    }
    None
}

/// A named year and the month and day after it, as far as a calendar can place them.
///
/// A month and day that no calendar in that year held — 乾隆三年十二月三十 — keeps the
/// year and loses the rest: the year is what the text asserted, the date is not.
fn place_named_year(
    year: i32,
    trailing: Option<&TrailingDate>,
) -> (&'static str, String, Option<NaiveDate>) {
    let Some(t) = trailing else {
        return ("era_year", format!("{year}"), None);
    };
    let date = if t.lunar {
        calendar::lunar_to_solar_date(year, t.month, t.day, t.leap)
    } else {
        NaiveDate::from_ymd_opt(year, t.month, t.day)
    };
    match date {
        Some(d) => (
            if t.lunar { "lunar_day" } else { "era_day" },
            d.format("%Y-%m-%d").to_string(),
            Some(d),
        ),
        None => ("era_year", format!("{year}"), None),
    }
}

/// `乾隆三年`, `万历十五年三月十二`, `民国三十八年`.
///
/// The regnal number sits between the name and 年, which is what keeps the era table from
/// firing on ordinary words: 「正统的观点」 has no number and no 年, 「崇祯十七年」 has both.
/// A regnal year the era never reached is refused outright.
fn match_named_year(chars: &[char], at: usize, eras: &Buckets) -> Option<Found> {
    let (idx, after_name) = match_buckets(eras, chars, at)?;
    let (name, _, _) = calendar::ERAS[idx];
    let (regnal, after_regnal) = if chars.get(after_name) == Some(&'元') {
        (1, after_name + 1)
    } else {
        calendar::small_count(chars, after_name)?
    };
    if chars.get(after_regnal) != Some(&'年') {
        return None;
    }
    let mut end = after_regnal + 1;
    let trailing = match_trailing_date(chars, end);
    if let Some(t) = &trailing {
        end = t.end;
    }

    let candidates = calendar::era_candidates(name, regnal);
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() > 1 {
        // Two reigns used this name and the number fits both — 至元三年 is 1266 or 1337
        // and the text does not say which. Counted, not placed.
        return Some(Found {
            start: at,
            end,
            kind: "partial",
            precision: "era_year",
            resolved: String::new(),
            order_key: None,
            offset_days: 0.0,
            ganzhi: None,
        });
    }
    let (precision, resolved, order_key) = place_named_year(candidates[0], trailing.as_ref());
    Some(Found {
        start: at,
        end,
        kind: "absolute",
        precision,
        resolved,
        order_key,
        offset_days: 0.0,
        ganzhi: None,
    })
}

/// `庚子年`, `癸卯年八月十五`.
///
/// A 干支 is a position in a 60-year wheel, not a year: 庚子 was 1840, 1900 and 1960. The
/// marker is left unplaced here and anchored against the document's own dates afterwards;
/// if that still leaves more than one answer, it stays unplaced.
fn match_ganzhi_year(chars: &[char], at: usize) -> Option<Found> {
    if chars.get(at + 2) != Some(&'年') {
        return None;
    }
    let text: String = chars[at..at + 2].iter().collect();
    let index = calendar::ganzhi_index(&text)?;
    let mut end = at + 3;
    let trailing = match_trailing_date(chars, end);
    let lunar = match &trailing {
        Some(t) if t.lunar => {
            end = t.end;
            Some((t.month, t.day, t.leap))
        }
        // A solar-looking day after a 干支 year is left to the other matchers rather than
        // half-consumed here.
        _ => None,
    };
    let precision = if lunar.is_some() {
        "ganzhi_day"
    } else {
        "ganzhi_year"
    };
    Some(Found {
        start: at,
        end,
        kind: "partial",
        precision,
        resolved: String::new(),
        order_key: None,
        offset_days: 0.0,
        ganzhi: Some(GanzhiYear { index, lunar }),
    })
}

/// `腊月初八`, `八月初三`, `八月十五`, `闰二月` — a lunar date with no year in front of
/// it, so it is counted rather than placed.
///
/// A numbered month counts only when the day after it could not be anything but lunar:
/// 八月初三 and 八月十五 are lunar, 八月十二 is a solar date another matcher reads.
fn match_lunar_date(chars: &[char], at: usize) -> Option<Found> {
    let named = calendar::lunar_month_name(chars, at);
    let after_month = match named {
        Some((_, _, end)) => end,
        None => {
            let (month, after) = calendar::small_count(chars, at)?;
            if !(1..=12).contains(&month) || chars.get(after) != Some(&'月') {
                return None;
            }
            after + 1
        }
    };
    let day = calendar::lunar_day_name(chars, after_month);
    let (precision, end) = match day {
        Some((_, end)) => ("lunar_month_day", end),
        // A month named in lunar terms is a month even with no day after it. A numbered
        // month with no lunar day name is the solar matcher's business.
        None => match named {
            Some(_) => ("lunar_month", after_month),
            None => return None,
        },
    };
    Some(Found {
        start: at,
        end,
        kind: "partial",
        precision,
        resolved: String::new(),
        order_key: None,
        offset_days: 0.0,
        ganzhi: None,
    })
}

/// `初八`, `廿三` on their own: a day of a month the reader already knows. Only 初 and 廿
/// start these; a bare 三十 is more likely to be the number thirty.
fn match_lunar_day_only(chars: &[char], at: usize) -> Option<Found> {
    if !matches!(chars.get(at), Some('初') | Some('廿')) {
        return None;
    }
    let (_, end) = calendar::lunar_day_name(chars, at)?;
    Some(Found {
        start: at,
        end,
        kind: "partial",
        precision: "lunar_day",
        resolved: String::new(),
        order_key: None,
        offset_days: 0.0,
        ganzhi: None,
    })
}

/// The word tables, bucketed once per document.
struct Lexicons {
    units: Buckets,
    deictic: Buckets,
    vague: Buckets,
    /// Reign era and 民国 names, valued by their row in the calendar table.
    eras: Buckets,
    /// 节气, 节日, 时辰 and weekday words, valued by their index into `term_precisions`.
    terms: Buckets,
    /// The precision of each term, in the order the term table was built.
    term_precisions: Vec<&'static str>,
}

/// 节气, 节日, 时辰 and weekday words. They say roughly when and nothing about which year,
/// so none of them is ever placed or ordered; they are kept apart from the other vague
/// words only so the report can say which of them a text leans on.
fn term_table() -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(&'static str, &'static str)> = Vec::new();
    out.extend(calendar::SOLAR_TERMS.iter().map(|w| (*w, "solar_term")));
    out.extend(calendar::FESTIVALS.iter().map(|w| (*w, "festival")));
    out.extend(calendar::HOUR_PERIODS.iter().map(|w| (*w, "time_of_day")));
    out.extend(calendar::WEEKDAYS.iter().map(|w| (*w, "weekday")));
    out
}

/// 周 is a dynasty, a surname and a turning before it is a week. A weekday word that opens
/// with 周 is refused when the character in front of it belongs to a dynasty name: 西周末期
/// is the end of the Western Zhou and 后周末期 the end of the Later Zhou, neither is a
/// weekend. The surname case (周三多 as an author) cannot be told apart this way, which is
/// what the 字面识别 note is for.
fn dynasty_zhou(chars: &[char], pos: usize) -> bool {
    const DYNASTY_BEFORE: [char; 7] = ['东', '西', '南', '北', '前', '后', '战'];
    pos > 0 && DYNASTY_BEFORE.contains(&chars[pos - 1])
}

/// The era table as buckets, valued by the row each name sits on.
fn era_buckets() -> Buckets {
    let rows: Vec<(&'static str, usize)> = calendar::ERAS
        .iter()
        .enumerate()
        .map(|(i, (name, _, _))| (*name, i))
        .collect();
    build_buckets(&rows)
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
/// plus a stray 后. A lunar month-and-day is tried before the bare month for the same
/// reason: `八月初三` is one expression, not 八月 followed by something missed.
fn scan_block(chars: &[char], lex: &Lexicons) -> Vec<Found> {
    let mut found: Vec<Found> = Vec::new();
    let mut pos = 0;
    while pos < chars.len() {
        let numeric = match_absolute(chars, pos)
            .or_else(|| match_month_day(chars, pos))
            .or_else(|| match_named_year(chars, pos, &lex.eras))
            .or_else(|| match_ganzhi_year(chars, pos))
            .or_else(|| match_lunar_date(chars, pos))
            .or_else(|| match_relative(chars, pos, &lex.units))
            .or_else(|| match_month_only(chars, pos))
            .or_else(|| match_day_only(chars, pos))
            .or_else(|| match_lunar_day_only(chars, pos));
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
                ganzhi: None,
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
                ganzhi: None,
            });
            pos = end;
            continue;
        }
        if let Some((idx, end)) = match_buckets(&lex.terms, chars, pos) {
            if lex.term_precisions[idx] == "weekday" && dynasty_zhou(chars, pos) {
                // 西周末期 is the end of a dynasty, 后周末期 the end of another one. The
                // word inside them is not a week.
                pos += 1;
                continue;
            }
            found.push(Found {
                start: pos,
                end,
                kind: "vague",
                precision: lex.term_precisions[idx],
                resolved: String::new(),
                order_key: None,
                offset_days: 0.0,
                ganzhi: None,
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
    let terms = term_table();
    let term_precisions = terms.iter().map(|(_, precision)| *precision).collect();
    let lex = Lexicons {
        units: build_buckets(UNITS),
        deictic: build_buckets(DEICTIC),
        vague: build_buckets(VAGUE),
        eras: era_buckets(),
        terms: build_buckets(&terms),
        term_precisions,
    };

    // One pass to find the expressions, a second to place them: a 干支 year cannot be
    // placed until the document's own dated anchors are known.
    struct Scan {
        segment: Option<usize>,
        label: String,
        position: f64,
        text: String,
        excerpt: String,
        found: Found,
    }
    let mut scans: Vec<Scan> = Vec::new();
    let mut cursor: u64 = 0;

    for (block_idx, block) in doc.blocks.iter().enumerate() {
        let chars: Vec<char> = block.content.chars().collect();
        let block_chars = visible_char_count(&block.content) as u64;
        let segment = plan.segment_of.get(block_idx).copied();
        let label = segment.map_or_else(String::new, |s| plan.labels[s].clone());
        for found in scan_block(&chars, &lex) {
            if scans.len() >= MAX_MARKERS {
                break;
            }
            scans.push(Scan {
                segment,
                label: label.clone(),
                position: if char_count > 0 {
                    (cursor + found.start as u64) as f64 / char_count as f64
                } else {
                    0.0
                },
                text: chars[found.start..found.end].iter().collect(),
                excerpt: excerpt(&chars, found.start, found.end),
                found,
            });
        }
        cursor += block_chars;
    }

    // The years the document states outright, with where in the reading each one sits.
    let anchors: Vec<(usize, i32)> = scans
        .iter()
        .enumerate()
        .filter_map(|(i, scan)| match scan.found.order_key {
            Some(date) => Some((i, date.year())),
            None => scan
                .found
                .resolved
                .parse::<i32>()
                .ok()
                .map(|year| (i, year)),
        })
        .collect();

    // Anchor a 干支 year to the nearest dated anchor in the reading, then to the wheel
    // position inside a 60-year window around it: 29 years below and 30 above holds
    // exactly one, so the answer is forced rather than chosen. With no anchor, or with an
    // anchor too far from any single cycle position, it stays unplaced.
    let mut unanchored: Vec<String> = Vec::new();
    for (i, scan) in scans.iter_mut().enumerate() {
        let Some(ganzhi) = scan.found.ganzhi.take() else {
            continue;
        };
        let placed = anchors
            .iter()
            .filter(|(at, _)| *at != i)
            .min_by_key(|(at, _)| (i as i64 - *at as i64).abs())
            .and_then(|(_, year)| {
                let candidates = calendar::ganzhi_years(ganzhi.index, year - 29, year + 30);
                (candidates.len() == 1).then_some(candidates[0])
            });
        if let Some(year) = placed {
            let (precision, resolved, order_key) = match ganzhi.lunar {
                Some((month, day, leap)) => {
                    match calendar::lunar_to_solar_date(year, month, day, leap) {
                        Some(date) => {
                            ("lunar_day", date.format("%Y-%m-%d").to_string(), Some(date))
                        }
                        None => ("ganzhi_year", format!("{year}"), None),
                    }
                }
                None => ("ganzhi_year", format!("{year}"), None),
            };
            scan.found.kind = "absolute";
            scan.found.precision = precision;
            scan.found.resolved = resolved;
            scan.found.order_key = order_key;
        } else {
            unanchored.push(scan.text.clone());
        }
    }

    let mut markers: Vec<TimelineMarker> = Vec::new();
    // The dates that can be placed on a calendar: (segment label, as written, key).
    let mut dated: Vec<(String, String, NaiveDate)> = Vec::new();
    let mut per_segment = vec![0u32; plan.labels.len()];
    for scan in scans {
        if let Some(s) = scan.segment {
            per_segment[s] += 1;
        }
        if let Some(key) = scan.found.order_key {
            dated.push((scan.label.clone(), scan.found.resolved.clone(), key));
        }
        markers.push(TimelineMarker {
            text: scan.text,
            kind: scan.found.kind.to_string(),
            precision: scan.found.precision.to_string(),
            section: scan.label,
            position: scan.position,
            resolved: scan.found.resolved,
            offset_days: scan.found.offset_days,
            excerpt: scan.excerpt,
        });
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
    // How the expressions are written, most common first. This is where a form that only
    // exists because of a calendar table becomes visible: 年号, 干支, 农历.
    let mut form_counts: HashMap<String, u32> = HashMap::new();
    for marker in &markers {
        *form_counts.entry(marker.precision.clone()).or_default() += 1;
    }
    let mut forms: Vec<(String, u32)> = form_counts.into_iter().collect();
    forms.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

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
        forms,
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
    build_findings(&mut stats, &dated, &unmarked_stretches, &unanchored);
    Ok(stats)
}

/// Turn the scan into findings: what the calendar says about the reading order, and —
/// said just as plainly — what cannot be said at all.
fn build_findings(
    stats: &mut TimelineStats,
    dated: &[(String, String, NaiveDate)],
    stretches: &[(String, String, u32)],
    unanchored: &[String],
) {
    let mut issues: Vec<TimelineIssue> = Vec::new();
    if stats.markers.is_empty() {
        let detail = if stats.entity_date_count > 0 {
            format!(
                "扫描没有读到可解析的时间表达，可导入的实体标注里有 {} 处日期：写法不在识别范围内（外文日期、生造的写法，或根本不是日期）",
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

    // What the calendar tables could not settle, said as plainly as the rest.
    if !unanchored.is_empty() {
        let listed = unanchored
            .iter()
            .take(3)
            .map(|t| format!("「{t}」"))
            .collect::<Vec<_>>()
            .join("、");
        issues.push(TimelineIssue {
            kind: "unanchored_ganzhi".into(),
            section: String::new(),
            detail: format!(
                "{listed} 是干支纪年，六十年一轮回；文中没有可靠的年份可靠近，只记下未定年"
            ),
        });
    }
    let shared_eras: Vec<&str> = stats
        .markers
        .iter()
        .filter(|m| m.precision == "era_year" && m.resolved.is_empty())
        .map(|m| m.text.as_str())
        .collect();
    if !shared_eras.is_empty() {
        let listed = shared_eras
            .iter()
            .take(3)
            .map(|t| format!("「{t}」"))
            .collect::<Vec<_>>()
            .join("、");
        issues.push(TimelineIssue {
            kind: "ambiguous_era".into(),
            section: String::new(),
            detail: format!("{listed} 这个年号不止一位皇帝用过，在位年数两段都对得上，无法定年"),
        });
    }
    let converted = stats
        .markers
        .iter()
        .filter(|m| m.precision == "lunar_day")
        .count();
    if converted > 0 {
        issues.push(TimelineIssue {
            kind: "calendar_note".into(),
            section: String::new(),
            detail: format!(
                "{converted} 处月日按农历换算成公历：年号与干支后面的月日按农历读，\
                 换算出自开源历法引擎，改元之年只精确到年"
            ),
        });
    }
    let named_terms = stats
        .markers
        .iter()
        .filter(|m| matches!(m.precision.as_str(), "solar_term" | "festival" | "weekday"))
        .count();
    if named_terms > 0 {
        issues.push(TimelineIssue {
            kind: "term_collision".into(),
            section: String::new(),
            detail: format!(
                "{named_terms} 处节气、节日与星期词按字面识别：同样的字也可能是人名或书名\
                 （清明、白露都可能用作名字，清明上河图不是清明，周三多也可能是作者）"
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
        let terms = term_table();
        let lex = Lexicons {
            units: build_buckets(UNITS),
            deictic: build_buckets(DEICTIC),
            vague: build_buckets(VAGUE),
            eras: era_buckets(),
            term_precisions: terms.iter().map(|(_, precision)| *precision).collect(),
            terms: build_buckets(&terms),
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
            stable_id: None,
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

    #[test]
    fn test_a_reign_year_becomes_a_gregorian_year() {
        let found = scan("乾隆三年，他第一次进宫。");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "absolute");
        assert_eq!(found[0].precision, "era_year");
        assert_eq!(found[0].resolved, "1738");
        // A year on its own is still not something to order.
        assert!(found[0].order_key.is_none());
    }

    #[test]
    fn test_a_reign_year_and_a_lunar_day_become_one_date() {
        let found = scan("乾隆三年三月初五，他第一次进宫。");
        assert_eq!(
            found.len(),
            1,
            "the year and the lunar day are one expression"
        );
        assert_eq!(found[0].precision, "lunar_day");
        assert_eq!(found[0].resolved, "1738-04-23");
        assert!(found[0].order_key.is_some());
    }

    #[test]
    fn test_a_reign_year_the_era_never_reached_is_refused() {
        assert!(scan("乾隆七十一年，他还在进宫。").is_empty());
    }

    #[test]
    fn test_an_era_name_without_a_reign_year_is_not_a_date() {
        // 正统 is an era name and an ordinary word; only the number in front of 年 tells
        // them apart.
        assert!(scan("他批评这份抄本不够正统。").is_empty());
        assert!(scan("乾隆年间的事，没人记得。").is_empty());
    }

    #[test]
    fn test_an_era_shared_by_two_reigns_is_counted_not_placed() {
        let found = scan("至元三年，船从泉州出发。");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "partial");
        assert!(
            found[0].resolved.is_empty(),
            "1266 or 1337: the text does not say which"
        );
    }

    #[test]
    fn test_a_minguo_year_reads_as_a_solar_date() {
        let found = scan("民国三十八年3月5日，他上了船。");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].precision, "era_day");
        assert_eq!(found[0].resolved, "1949-03-05");
    }

    #[test]
    fn test_a_lunar_day_is_counted_without_a_year() {
        let found = scan("腊月初八，粥是凉的。");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].kind, "partial");
        assert_eq!(found[0].precision, "lunar_month_day");
        assert!(found[0].resolved.is_empty());
    }

    #[test]
    fn test_a_lunar_day_name_makes_the_month_lunar() {
        // 八月初三 can only be a lunar day; 八月十二日 is a solar date, and the other
        // matcher keeps that one.
        let lunar = scan("八月初三，他回来了。");
        assert_eq!(lunar.len(), 1);
        assert_eq!(lunar[0].precision, "lunar_month_day");
        let solar = scan("八月十二日，他回来了。");
        assert_eq!(solar.len(), 1);
        assert_eq!(solar[0].precision, "month_day");
    }

    #[test]
    fn test_solar_terms_and_festivals_stay_vague() {
        let found = scan("冬至那天，她提到了中秋。");
        let terms: Vec<&str> = found.iter().map(|f| f.precision).collect();
        assert!(terms.contains(&"solar_term"), "got {terms:?}");
        assert!(terms.contains(&"festival"), "got {terms:?}");
        for f in &found {
            if matches!(f.precision, "solar_term" | "festival") {
                assert_eq!(f.kind, "vague");
                assert!(f.resolved.is_empty());
            }
        }
    }

    #[test]
    fn test_a_ganzhi_year_is_anchored_to_the_documents_dates() {
        let doc = doc_of(vec![
            para("上部", &long_body("1900年5月3日，河开了冻。")),
            para("下部", &long_body("庚子年八月十五，月亮很亮。")),
        ]);
        let stats = run_timeline_analysis(&doc).unwrap();
        let marker = stats
            .markers
            .iter()
            .find(|m| m.text == "庚子年八月十五")
            .expect("the ganzhi year is a marker");
        assert_eq!(marker.kind, "absolute");
        assert_eq!(marker.resolved, "1900-09-08");
        // Both dates can be placed now, and the lunar one is the later of the two.
        assert_eq!(stats.last_date, "1900-09-08");
        assert!(stats.backsteps.is_empty());
    }

    #[test]
    fn test_a_ganzhi_year_without_an_anchor_stays_unplaced() {
        let doc = doc_of(vec![para("上部", &long_body("庚子年，河开了冻。"))]);
        let stats = run_timeline_analysis(&doc).unwrap();
        let marker = stats
            .markers
            .iter()
            .find(|m| m.text == "庚子年")
            .expect("the ganzhi year is a marker");
        assert_eq!(marker.kind, "partial");
        assert!(marker.resolved.is_empty());
        let issue = stats
            .issues
            .iter()
            .find(|i| i.kind == "unanchored_ganzhi")
            .expect("not being able to place it is worth saying");
        assert!(issue.detail.contains("六十年"));
    }

    #[test]
    fn test_the_lunar_conversion_says_so() {
        let doc = doc_of(vec![para(
            "上部",
            &long_body("乾隆三年三月初五，他第一次进宫。"),
        )]);
        let stats = run_timeline_analysis(&doc).unwrap();
        let issue = stats
            .issues
            .iter()
            .find(|i| i.kind == "calendar_note")
            .expect("a conversion by table and engine is a claim worth labelling");
        assert!(issue.detail.contains("农历"));
    }

    #[test]
    fn test_eras_from_the_wider_table_are_read() {
        // 蜀汉 late, 东晋, 南朝, 后周: names a chronicle keeps and a shortlist missed.
        for (text, year) in [
            ("景耀六年，他出城投降。", "263"),
            ("义熙十四年，他回来了。", "418"),
            ("中大通六年，寺里重修。", "534"),
            ("广顺三年，他受了禅。", "953"),
        ] {
            let found = scan(text);
            assert_eq!(found.len(), 1, "{text}");
            assert_eq!(found[0].kind, "absolute", "{text}");
            assert_eq!(found[0].precision, "era_year", "{text}");
            assert_eq!(found[0].resolved, year, "{text}");
        }
    }

    #[test]
    fn test_a_word_that_is_both_an_era_and_a_festival_follows_the_text() {
        // 上元 names a reign and the fifteenth of the first month. Only the number in
        // front of 年 makes it a reign, and only that number says which reign it means.
        let festival = scan("上元节那晚，街上全是灯。");
        let lights: Vec<&Found> = festival
            .iter()
            .filter(|f| f.precision == "festival")
            .collect();
        assert_eq!(lights.len(), 1);
        assert_eq!(
            (lights[0].start, lights[0].end),
            (0, 2),
            "just the two characters"
        );

        let reign = scan("开元十三年，他下了第。");
        assert_eq!(reign.len(), 1);
        assert_eq!(reign[0].kind, "absolute");
        assert_eq!(reign[0].resolved, "725");

        let both = scan("上元二年，他下了第。");
        assert_eq!(both.len(), 1);
        assert_eq!(both[0].kind, "partial", "675 or 761: the number fits both");
    }

    #[test]
    fn test_text_that_ends_on_a_count_word_does_not_reach_past_the_end() {
        // A real book found this: a block whose text ends with 六. The unit table is tried
        // one character after the count, which there is no character at.
        for text in ["他说了六", "再过三", "那年乾", "二〇二"] {
            let _ = scan(text);
        }
        assert_eq!(
            scan("他说了六").len(),
            0,
            "a bare count is a count, not a date"
        );
    }

    #[test]
    fn test_a_dynasty_is_not_a_week() {
        // 周末 inside 西周末期 is the end of a dynasty; the same two characters after a
        // demonstrative are a weekend.
        for text in ["西周末期，人们改用竹简。", "后周末期，他南下投军。"] {
            assert!(scan(text).is_empty(), "{text} says nothing about a week");
        }
        let weekend = scan("这个周末他回了家。");
        assert_eq!(weekend.len(), 1);
        assert_eq!(weekend[0].precision, "weekday");
        assert_eq!(weekend[0].kind, "vague");
    }

    #[test]
    fn test_the_forms_are_counted() {
        let doc = doc_of(vec![para(
            "上部",
            &long_body("2003年3月12日出发，腊月初八回来，冬至又下雪。"),
        )]);
        let stats = run_timeline_analysis(&doc).unwrap();
        let has = |name: &str| stats.forms.iter().any(|(form, _)| form == name);
        assert!(has("day"), "forms: {:?}", stats.forms);
        assert!(has("lunar_month_day"), "forms: {:?}", stats.forms);
        assert!(has("solar_term"), "forms: {:?}", stats.forms);
    }
}
