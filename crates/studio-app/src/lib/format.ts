//! Presentation helpers shared by the analysis and comparison views.
//!
//! Kept free of JSX and Tauri imports so they can be unit-tested with Vitest
//! (see development.md §7.1: frontend unit tests cover components/utils).

import type { T0Stats } from "../types";

/** Total entity occurrences across all categories. */
export function entityTotal(t0: T0Stats): number {
  return t0.entity_counts.reduce((sum, [, count]) => sum + count, 0);
}

/** Thousands-separated integer for display. */
export function formatInt(value: number): string {
  return value.toLocaleString();
}

/** A 0-100 score rendered as a whole number. */
export function formatScore(score: number): string {
  return score.toFixed(0);
}

/** A 0-1 ratio rendered as a whole percent, e.g. 0.427 -> "43%". */
export function formatPercent(ratio: number): string {
  return `${(ratio * 100).toFixed(0)}%`;
}

/** A fixed-precision decimal for averages (sentence length, readability…). */
export function formatFixed(value: number, digits = 1): string {
  return value.toFixed(digits);
}

/** Tailwind background class for a 0-100 score (green / amber / red). */
export function scoreColor(score: number): string {
  if (score >= 80) return "bg-emerald-400";
  if (score >= 60) return "bg-amber-400";
  return "bg-red-400";
}

/** Tailwind text-color class for a 0-100 score (green / amber / red). */
export function scoreTextColor(score: number): string {
  if (score >= 80) return "text-emerald-400";
  if (score >= 60) return "text-amber-400";
  return "text-red-400";
}

const SEVERITY_STYLES: Record<string, string> = {
  high: "bg-red-500/15 text-red-400 border-red-500/30",
  medium: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  low: "bg-sky-500/15 text-sky-400 border-sky-500/30",
};

const SEVERITY_LABELS: Record<string, string> = {
  high: "高",
  medium: "中",
  low: "低",
};

/** Badge classes for a concern/recommendation severity. Unknown falls back to low. */
export function severityStyle(severity: string): string {
  return SEVERITY_STYLES[severity] ?? SEVERITY_STYLES.low;
}

/** Localized label for a severity. Unknown severities fall through verbatim. */
export function severityLabel(severity: string): string {
  return SEVERITY_LABELS[severity] ?? severity;
}

const ARC_SHAPE_LABELS: Record<string, string> = {
  mountain: "先扬后抑",
  rising: "渐强",
  falling: "渐弱",
  steady: "平稳",
};

/** Localized label for a narrative-arc shape. */
export function arcShapeLabel(shape: string): string {
  return ARC_SHAPE_LABELS[shape] ?? shape;
}

/**
 * Localized label for a plotline shape.
 *
 * Mirrors `plot_shape_label` in `studio-report::markdown` so the screen and the
 * exported report name the same thread the same way. Unknown shapes fall through
 * verbatim, exactly as for the arc.
 */
const PLOT_SHAPE_LABELS: Record<string, string> = {
  sustained: "贯穿",
  intermittent: "断续",
  burst: "集中",
  setup_only: "仅铺垫",
  late_arrival: "迟到",
  minor: "次要",
};

export function plotShapeLabel(shape: string): string {
  return PLOT_SHAPE_LABELS[shape] ?? shape;
}

/**
 * Badge classes for a plotline shape: the two shapes that ask for a rewrite
 * (setup dropped early, a thread arriving in the last fifth) read as warnings, a
 * thread that runs through the document reads as the healthy case.
 */
const PLOT_SHAPE_STYLES: Record<string, string> = {
  sustained: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  intermittent: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  burst: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  setup_only: "bg-red-500/15 text-red-400 border-red-500/30",
  late_arrival: "bg-red-500/15 text-red-400 border-red-500/30",
  minor: "bg-gray-500/15 text-text-muted border-gray-500/30",
};

export function plotShapeStyle(shape: string): string {
  return PLOT_SHAPE_STYLES[shape] ?? PLOT_SHAPE_STYLES.minor;
}

/**
 * Localized label for a pacing verdict.
 *
 * Mirrors `pacing_verdict_label` in `studio-report::markdown` so the screen and
 * the exported report describe the same curve with the same words. Unknown
 * verdicts fall through verbatim.
 */
const PACING_VERDICT_LABELS: Record<string, string> = {
  flat: "单调",
  varied: "起伏",
  spiky: "跳跃",
};

export function pacingVerdictLabel(verdict: string): string {
  return PACING_VERDICT_LABELS[verdict] ?? verdict;
}

/**
 * Badge classes for a pacing verdict: a curve that never moves is the case that
 * asks for a rewrite, a curve that moves is the healthy one, and a curve that
 * swings hard is worth a look rather than an alarm.
 */
const PACING_VERDICT_STYLES: Record<string, string> = {
  flat: "bg-red-500/15 text-red-400 border-red-500/30",
  varied: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  spiky: "bg-amber-500/15 text-amber-400 border-amber-500/30",
};

export function pacingVerdictStyle(verdict: string): string {
  return PACING_VERDICT_STYLES[verdict] ?? PACING_VERDICT_STYLES.flat;
}

/**
 * Localized label for a conflict family. Mirrors `conflict_type_label` in
 * `studio-report::markdown`; `""` is a segment with no conflict vocabulary at all.
 */
const CONFLICT_TYPE_LABELS: Record<string, string> = {
  interpersonal: "人际",
  internal: "内心",
  societal: "社会",
  survival: "生存",
  "": "无",
};

export function conflictTypeLabel(kind: string): string {
  return CONFLICT_TYPE_LABELS[kind] ?? kind;
}

/**
 * Localized label for how much conflict vocabulary a document carries.
 *
 * `none` deliberately reads as "no conflict *vocabulary*", not "no conflict": the
 * text may be non-narrative, or the lexicon may simply miss its wording.
 */
const CONFLICT_VERDICT_LABELS: Record<string, string> = {
  none: "未见用词",
  thin: "偏少",
  present: "适中",
  heavy: "密集",
};

export function conflictVerdictLabel(verdict: string): string {
  return CONFLICT_VERDICT_LABELS[verdict] ?? verdict;
}

/**
 * Badge classes for a conflict verdict. A thin spread is the case worth a look;
 * "no vocabulary found" is neutral, because the engine cannot tell a conflict-free
 * story from a text it has no words for.
 */
const CONFLICT_VERDICT_STYLES: Record<string, string> = {
  none: "bg-gray-500/15 text-text-muted border-gray-500/30",
  thin: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  present: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  heavy: "bg-sky-500/15 text-sky-400 border-sky-500/30",
};

export function conflictVerdictStyle(verdict: string): string {
  return CONFLICT_VERDICT_STYLES[verdict] ?? CONFLICT_VERDICT_STYLES.none;
}

/**
 * A score that may be absent.
 *
 * Conflict and theme analysis withhold the score rather than reporting 0 when the
 * vocabulary they need is missing, so every view has one way to say "未评分".
 */
export function formatOptionalScore(score: number | null): string {
  return score === null ? "未评分" : `${formatScore(score)} / 100`;
}

/**
 * Localized label for the vocabulary the theme pass read.
 *
 * Naming the source matters: a theme list built from the importer's noun signals
 * and one rebuilt from POS tags are not equally trustworthy, so the screen shows
 * which one it is looking at. Mirrors `theme_source_label` in
 * `studio-report::markdown`.
 */
const THEME_SOURCE_LABELS: Record<string, string> = {
  noun_signals: "名词信号",
  noun_tokens: "名词词性",
  none: "无",
};

export function themeSourceLabel(source: string): string {
  return THEME_SOURCE_LABELS[source] ?? source;
}

/**
 * Localized label for how far a theme reaches across the document. Mirrors
 * `theme_reach_label` in `studio-report::markdown`.
 */
const THEME_REACH_LABELS: Record<string, string> = {
  sustained: "贯穿",
  recurring: "复现",
  episodic: "偶发",
  occasional: "零星",
};

export function themeReachLabel(reach: string): string {
  return THEME_REACH_LABELS[reach] ?? reach;
}

/**
 * Badge classes for a theme's reach: a subject that runs through the document is
 * the case the feature is looking for, one that appears twice is worth a look,
 * and a one-off is noise rather than a theme.
 */
const THEME_REACH_STYLES: Record<string, string> = {
  sustained: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  recurring: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  episodic: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  occasional: "bg-gray-500/15 text-text-muted border-gray-500/30",
};

export function themeReachStyle(reach: string): string {
  return THEME_REACH_STYLES[reach] ?? THEME_REACH_STYLES.occasional;
}

/**
 * Localized label for where a theme's mentions sit. Mirrors
 * `theme_concentration_label` in `studio-report::markdown`: a subject that only
 * ever appears in one stretch is a topic of that passage, not of the book.
 */
const THEME_CONCENTRATION_LABELS: Record<string, string> = {
  spread: "均匀",
  front: "集中在开头",
  middle: "集中在中段",
  back: "集中在结尾",
};

export function themeConcentrationLabel(concentration: string): string {
  return THEME_CONCENTRATION_LABELS[concentration] ?? concentration;
}

/**
 * Localized label for how much environment vocabulary a document carries.
 *
 * `none` says no setting *vocabulary*, not no setting: a scene can be anchored by
 * a name the entity model caught, or by description, or by neither. Mirrors
 * `setting_verdict_label` in `studio-report::markdown`.
 */
const SETTING_VERDICT_LABELS: Record<string, string> = {
  none: "未见用词",
  thin: "偏少",
  present: "适中",
  heavy: "密集",
};

export function settingVerdictLabel(verdict: string): string {
  return SETTING_VERDICT_LABELS[verdict] ?? verdict;
}

/**
 * Badge classes for an environment verdict. Thin description is the case worth a
 * look; "no vocabulary found" is neutral, because the engine cannot tell a bare
 * scene from one it has no words for.
 */
const SETTING_VERDICT_STYLES: Record<string, string> = {
  none: "bg-gray-500/15 text-text-muted border-gray-500/30",
  thin: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  present: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  heavy: "bg-sky-500/15 text-sky-400 border-sky-500/30",
};

export function settingVerdictStyle(verdict: string): string {
  return SETTING_VERDICT_STYLES[verdict] ?? SETTING_VERDICT_STYLES.none;
}

/** Localized label for the kind of place the importer found. */
const PLACE_KIND_LABELS: Record<string, string> = {
  LOCATION: "地点",
  FACILITY: "场所",
};

export function placeKindLabel(kind: string): string {
  return PLACE_KIND_LABELS[kind] ?? kind;
}

/**
 * Localized label for what the setting gave the reader, or withheld. Mirrors
 * `setting_issue_label` in `studio-report::markdown`.
 */
const SETTING_ISSUE_LABELS: Record<string, string> = {
  unanchored: "无地点锚点",
  crowded: "地点过密",
  long_absence: "长时间离开地点",
  thin_environment: "环境描写偏少",
};

export function settingIssueLabel(kind: string): string {
  return SETTING_ISSUE_LABELS[kind] ?? kind;
}

/**
 * Badge classes for a scene issue: a scene that never says where it is the case
 * that loses a reader, a place left for a while is only worth a look.
 */
const SETTING_ISSUE_STYLES: Record<string, string> = {
  unanchored: "bg-red-500/15 text-red-400 border-red-500/30",
  crowded: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  long_absence: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  thin_environment: "bg-amber-500/15 text-amber-400 border-amber-500/30",
};

export function settingIssueStyle(kind: string): string {
  return SETTING_ISSUE_STYLES[kind] ?? SETTING_ISSUE_STYLES.long_absence;
}

/**
 * Localized label for how much stock phrasing a document carries.
 *
 * `none` says the library found nothing, not that the prose is fresh: the pass only
 * knows the phrases it carries. Mirrors `cliche_verdict_label` in
 * `studio-report::markdown`.
 */
const CLICHE_VERDICT_LABELS: Record<string, string> = {
  none: "未见用词",
  light: "偏少",
  moderate: "中等",
  heavy: "密集",
};

export function clicheVerdictLabel(verdict: string): string {
  return CLICHE_VERDICT_LABELS[verdict] ?? verdict;
}

/**
 * Badge classes for a cliché verdict. A heavy rate is the case worth opening the
 * list for; "no vocabulary found" is neutral, because the library is a net with a
 * known mesh size rather than a judge.
 */
const CLICHE_VERDICT_STYLES: Record<string, string> = {
  none: "bg-gray-500/15 text-text-muted border-gray-500/30",
  light: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  moderate: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  heavy: "bg-red-500/15 text-red-400 border-red-500/30",
};

export function clicheVerdictStyle(verdict: string): string {
  return CLICHE_VERDICT_STYLES[verdict] ?? CLICHE_VERDICT_STYLES.none;
}

/**
 * Localized label for the kind of stock phrase that was found. Mirrors
 * `cliche_category_label` in `studio-report::markdown`.
 */
const CLICHE_CATEGORY_LABELS: Record<string, string> = {
  expression: "表情动作",
  body: "身体反应",
  emotion: "情绪直陈",
  simile: "套路比喻",
  atmosphere: "场面套话",
  speech_tag: "说话套话",
  transition: "时间过渡",
  look: "外貌套话",
};

export function clicheCategoryLabel(category: string): string {
  return CLICHE_CATEGORY_LABELS[category] ?? category;
}

/**
 * Localized label for what the phrase use does to the prose. Mirrors
 * `cliche_issue_label` in `studio-report::markdown`.
 */
const CLICHE_ISSUE_LABELS: Record<string, string> = {
  repeated: "反复使用",
  dense_segment: "局部密集",
  one_note: "依赖同一句",
  speech_tags: "对白标签过多",
};

export function clicheIssueLabel(kind: string): string {
  return CLICHE_ISSUE_LABELS[kind] ?? kind;
}

/**
 * Badge classes for a phrase issue: repeating one phrase is the habit readers
 * notice first, a single dense segment is usually one paragraph to redo.
 */
const CLICHE_ISSUE_STYLES: Record<string, string> = {
  repeated: "bg-red-500/15 text-red-400 border-red-500/30",
  dense_segment: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  one_note: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  speech_tags: "bg-sky-500/15 text-sky-400 border-sky-500/30",
};

export function clicheIssueStyle(kind: string): string {
  return CLICHE_ISSUE_STYLES[kind] ?? CLICHE_ISSUE_STYLES.dense_segment;
}

/**
 * Localized label for how much of the text is spoken.
 *
 * `none` says no speech was detected, not that nobody speaks: a text whose source
 * carries no quotation marks looks the same to the detector. Mirrors
 * `dialogue_verdict_label` in `studio-report::markdown`.
 */
const DIALOGUE_VERDICT_LABELS: Record<string, string> = {
  none: "未见对白",
  sparse: "对白偏少",
  balanced: "对白与叙述混合",
  heavy: "以对白为主",
};

export function dialogueVerdictLabel(verdict: string): string {
  return DIALOGUE_VERDICT_LABELS[verdict] ?? verdict;
}

/**
 * Badge classes for a dialogue verdict. No verdict is red: a ratio is a measurement,
 * not a grade, so the colors only separate the shapes (quiet, mixed, speech-led)
 * rather than rank them.
 */
const DIALOGUE_VERDICT_STYLES: Record<string, string> = {
  none: "bg-gray-500/15 text-text-muted border-gray-500/30",
  sparse: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  balanced: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  heavy: "bg-amber-500/15 text-amber-400 border-amber-500/30",
};

export function dialogueVerdictStyle(verdict: string): string {
  return DIALOGUE_VERDICT_STYLES[verdict] ?? DIALOGUE_VERDICT_STYLES.none;
}

/**
 * Localized label for how one piece of speech was recognised. Mirrors
 * `dialogue_kind_label` in `studio-report::markdown`.
 */
const DIALOGUE_KIND_LABELS: Record<string, string> = {
  quote: "引号",
  dash: "破折号",
};

export function dialogueKindLabel(kind: string): string {
  return DIALOGUE_KIND_LABELS[kind] ?? kind;
}

/**
 * Localized label for what the mix does to the reading. Mirrors `dialogue_issue_label`
 * in `studio-report::markdown`.
 */
const DIALOGUE_ISSUE_LABELS: Record<string, string> = {
  no_dialogue: "没有对白",
  silent_segment: "长时间无对白",
  flooded_segment: "局部对白过密",
  monologue: "长独白",
  telegraphic: "对白过短",
  unquoted: "对白未加引号",
  uneven: "各段比例不均",
};

export function dialogueIssueLabel(kind: string): string {
  return DIALOGUE_ISSUE_LABELS[kind] ?? kind;
}

/**
 * Badge classes for a dialogue issue. Amber marks the shapes usually worth a look
 * (a 300-character monologue, replies too short to be conversation); sky marks what
 * is only a property of the text or its punctuation, and gray states a fact about a
 * text that may not be narrative at all.
 */
const DIALOGUE_ISSUE_STYLES: Record<string, string> = {
  no_dialogue: "bg-gray-500/15 text-text-muted border-gray-500/30",
  silent_segment: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  flooded_segment: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  monologue: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  telegraphic: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  unquoted: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  uneven: "bg-amber-500/15 text-amber-400 border-amber-500/30",
};

export function dialogueIssueStyle(kind: string): string {
  return DIALOGUE_ISSUE_STYLES[kind] ?? DIALOGUE_ISSUE_STYLES.silent_segment;
}

/**
 * Localized label for the strongest wording the pass found.
 *
 * This is a content notice, not a mark against the manuscript: `strong` says the
 * text contains strong wording, not that the text is weak. `none` says the library
 * found nothing — a net with a known mesh size, not a clean bill of health. Mirrors
 * `explicit_advisory_label` in `studio-report::markdown`.
 */
const EXPLICIT_ADVISORY_LABELS: Record<string, string> = {
  none: "词库未见",
  mild: "轻微",
  moderate: "中等",
  strong: "强烈",
};

export function explicitAdvisoryLabel(advisory: string): string {
  return EXPLICIT_ADVISORY_LABELS[advisory] ?? advisory;
}

/**
 * Badge classes for an advisory level. The heat scale is the strength of the
 * wording, not a verdict on the manuscript — which is why the card carries the
 * "不评分" note next to the badge rather than a score where the other dimensions
 * put one. Mirrors the tier colors below.
 */
const EXPLICIT_ADVISORY_STYLES: Record<string, string> = {
  none: "bg-gray-500/15 text-text-muted border-gray-500/30",
  mild: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  moderate: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  strong: "bg-red-500/15 text-red-400 border-red-500/30",
};

export function explicitAdvisoryStyle(advisory: string): string {
  return EXPLICIT_ADVISORY_STYLES[advisory] ?? EXPLICIT_ADVISORY_STYLES.none;
}

/**
 * Localized label for how strong one term's wording is. The report keeps these to a
 * single character so the term table stays narrow. Mirrors `explicit_tier_label` in
 * `studio-report::markdown`.
 */
const EXPLICIT_TIER_LABELS: Record<string, string> = {
  mild: "轻",
  moderate: "中",
  strong: "重",
};

export function explicitTierLabel(tier: string): string {
  return EXPLICIT_TIER_LABELS[tier] ?? tier;
}

/** Badge classes for a tier; the same heat scale as the advisory level. */
const EXPLICIT_TIER_STYLES: Record<string, string> = {
  mild: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  moderate: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  strong: "bg-red-500/15 text-red-400 border-red-500/30",
};

export function explicitTierStyle(tier: string): string {
  return EXPLICIT_TIER_STYLES[tier] ?? EXPLICIT_TIER_STYLES.mild;
}

/**
 * Localized label for the family of content a hit belongs to. Mirrors
 * `explicit_category_label` in `studio-report::markdown`.
 */
const EXPLICIT_CATEGORY_LABELS: Record<string, string> = {
  profanity: "粗口辱骂",
  sexual: "性描写",
  sexual_violence: "性暴力",
  violence: "暴力血腥",
  self_harm: "自伤",
  hate: "贬损歧视",
  substance: "毒品",
};

export function explicitCategoryLabel(category: string): string {
  return EXPLICIT_CATEGORY_LABELS[category] ?? category;
}

/**
 * Localized label for why a family is being surfaced. Mirrors `explicit_issue_label`
 * in `studio-report::markdown`.
 */
const EXPLICIT_ISSUE_LABELS: Record<string, string> = {
  self_harm: "自伤描写",
  sexual_violence: "性暴力",
  sexual_content: "露骨性描写",
  gore: "血腥细节",
  hate_slur: "贬损歧视用语",
  substance: "毒品描写",
  dense_segment: "局部密集",
};

export function explicitIssueLabel(kind: string): string {
  return EXPLICIT_ISSUE_LABELS[kind] ?? kind;
}

/**
 * Badge classes for a content notice. Red marks the two families the library surfaces
 * from few hits — self-harm, where one mention matters more than the rate, and sexual
 * violence; amber marks the families that need a threshold of strong wording before
 * they say anything; sky marks `dense_segment`, which is a property of how the hits are
 * spread rather than of what they are.
 */
const EXPLICIT_ISSUE_STYLES: Record<string, string> = {
  self_harm: "bg-red-500/15 text-red-400 border-red-500/30",
  sexual_violence: "bg-red-500/15 text-red-400 border-red-500/30",
  sexual_content: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  gore: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  hate_slur: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  substance: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  dense_segment: "bg-sky-500/15 text-sky-400 border-sky-500/30",
};

export function explicitIssueStyle(kind: string): string {
  return EXPLICIT_ISSUE_STYLES[kind] ?? EXPLICIT_ISSUE_STYLES.dense_segment;
}

/**
 * Localized label for a classic shape tried against the document. Mirrors
 * `structure_model_label` in `studio-report::markdown`.
 */
const STRUCTURE_MODEL_LABELS: Record<string, string> = {
  three_act: "三幕结构",
  kisho_tenketsu: "起承转合",
  hero_journey: "英雄之旅",
};

export function structureModelLabel(model: string): string {
  return STRUCTURE_MODEL_LABELS[model] ?? model;
}

/**
 * Localized label for one beat of a model. Mirrors `structure_beat_label` in
 * `studio-report::markdown`.
 */
const STRUCTURE_BEAT_LABELS: Record<string, string> = {
  setup: "铺垫",
  first_turn: "第一转折点",
  midpoint: "中点",
  second_turn: "第二转折点",
  climax: "高潮",
  resolution: "收束",
  kisho: "起",
  sho: "承",
  ten: "转",
  ketsu: "合",
  ordinary_world: "日常世界",
  call: "冒险的召唤",
  threshold: "跨越门槛",
  tests: "试炼与伙伴",
  inmost_cave: "最深洞穴",
  resurrection: "复活",
  return: "携宝归来",
};

export function structureBeatLabel(beat: string): string {
  return STRUCTURE_BEAT_LABELS[beat] ?? beat;
}

/**
 * Localized label for where a beat was answered. `missing` says the signals showed
 * nothing there, which is not the same as the story lacking the beat.
 */
const STRUCTURE_STATUS_LABELS: Record<string, string> = {
  found: "命中",
  displaced: "错位",
  missing: "未见",
};

export function structureStatusLabel(status: string): string {
  return STRUCTURE_STATUS_LABELS[status] ?? status;
}

/**
 * Badge classes for a beat. Emerald is a landmark inside the window, amber one the
 * document puts outside it, gray the absence — never red, because an absent beat is
 * not a fault.
 */
const STRUCTURE_STATUS_STYLES: Record<string, string> = {
  found: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  displaced: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  missing: "bg-gray-500/15 text-text-muted border-gray-500/30",
};

export function structureStatusStyle(status: string): string {
  return STRUCTURE_STATUS_STYLES[status] ?? STRUCTURE_STATUS_STYLES.missing;
}

/** How many beats landed where the model expects them, in words not points. */
const STRUCTURE_FIT_LABELS: Record<string, string> = {
  matched: "吻合",
  partial: "部分吻合",
  unclear: "信号不足",
};

export function structureFitLabel(fit: string): string {
  return STRUCTURE_FIT_LABELS[fit] ?? fit;
}

const STRUCTURE_FIT_STYLES: Record<string, string> = {
  matched: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  partial: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  unclear: "bg-gray-500/15 text-text-muted border-gray-500/30",
};

export function structureFitStyle(fit: string): string {
  return STRUCTURE_FIT_STYLES[fit] ?? STRUCTURE_FIT_STYLES.unclear;
}

/** What kind of signal made a landmark. */
const STRUCTURE_LANDMARK_LABELS: Record<string, string> = {
  opening: "开篇",
  turn: "线程转折",
  peak: "张力高点",
  trough: "张力低点",
  climax: "张力最高点",
};

export function structureLandmarkLabel(kind: string): string {
  return STRUCTURE_LANDMARK_LABELS[kind] ?? kind;
}

/**
 * Badge classes for a landmark. The two tension extremes share a hue because they are
 * the same measurement read in opposite directions; a turn comes from the thread set
 * and is coloured by how sharply it changes.
 */
const STRUCTURE_LANDMARK_STYLES: Record<string, string> = {
  opening: "bg-gray-500/15 text-text-muted border-gray-500/30",
  turn: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  peak: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  trough: "bg-sky-500/15 text-sky-400 border-sky-500/30",
  climax: "bg-amber-500/15 text-amber-400 border-amber-500/30",
};

export function structureLandmarkStyle(kind: string): string {
  return STRUCTURE_LANDMARK_STYLES[kind] ?? STRUCTURE_LANDMARK_STYLES.opening;
}

/** Which part of the document's own three-way split. */
const STRUCTURE_ACT_LABELS: Record<string, string> = {
  act_one: "第一幕",
  act_two: "第二幕",
  act_three: "第三幕",
};

export function structureActLabel(name: string): string {
  return STRUCTURE_ACT_LABELS[name] ?? name;
}

/** What the shape does that a writer may want to look at again. */
const STRUCTURE_ISSUE_LABELS: Record<string, string> = {
  no_early_turn: "开头无变化",
  missing_midpoint: "中段空白",
  climax_early: "张力最高点偏早",
  no_descent: "结尾未回落",
  act_imbalance: "幕长失衡",
  too_few_landmarks: "地标太少",
};

export function structureIssueLabel(kind: string): string {
  return STRUCTURE_ISSUE_LABELS[kind] ?? kind;
}

/**
 * Badge classes for a structure note. Amber marks the five claims about where the
 * shape sits; gray marks `too_few_landmarks`, which is a limit on the reading rather
 * than a property of the story.
 */
const STRUCTURE_ISSUE_STYLES: Record<string, string> = {
  no_early_turn: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  missing_midpoint: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  climax_early: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  no_descent: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  act_imbalance: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  too_few_landmarks: "bg-gray-500/15 text-text-muted border-gray-500/30",
};

export function structureIssueStyle(kind: string): string {
  return STRUCTURE_ISSUE_STYLES[kind] ?? STRUCTURE_ISSUE_STYLES.too_few_landmarks;
}

/** What kind of time expression a marker is. */
const TIMELINE_KIND_LABELS: Record<string, string> = {
  absolute: "绝对日期",
  partial: "缺年的日期",
  relative: "相对表达",
  vague: "模糊表达",
};

export function timelineKindLabel(kind: string): string {
  return TIMELINE_KIND_LABELS[kind] ?? kind;
}

/** How precisely an expression pins the moment down. */
const TIMELINE_PRECISION_LABELS: Record<string, string> = {
  year: "只到年",
  month: "到月",
  day: "到日",
  month_day: "月日",
  offset: "距离",
  deictic: "指示",
  season: "季节",
  time_of_day: "时段",
  weekday: "星期",
  solar_term: "节气",
  festival: "节日",
  era_year: "年号·到年",
  era_day: "年号·到日",
  ganzhi_year: "干支·到年",
  ganzhi_day: "干支·到日",
  lunar_month: "农历月",
  lunar_month_day: "农历月日",
  lunar_day: "农历日",
};

export function timelinePrecisionLabel(precision: string): string {
  return TIMELINE_PRECISION_LABELS[precision] ?? precision;
}

/** A finding about how the document keeps time. */
const TIMELINE_ISSUE_LABELS: Record<string, string> = {
  no_markers: "读不到时间",
  too_short: "文本过短",
  no_order: "无法排序",
  backstep: "时间倒回",
  long_jump: "大跳跃",
  unmarked_stretch: "长时间无标记",
  unanchored_ganzhi: "干支未定年",
  ambiguous_era: "年号重名",
  calendar_note: "历法换算",
  term_collision: "字面识别",
};

export function timelineIssueLabel(kind: string): string {
  return TIMELINE_ISSUE_LABELS[kind] ?? kind;
}

/**
 * Badge classes for a time-line note. Rose is the one finding the calendar settles —
 * the dates really do go backwards. Amber marks the readings about distance and
 * coverage; gray marks the cases where the reading itself was impossible or where the
 * note describes the method rather than the story.
 */
const TIMELINE_ISSUE_STYLES: Record<string, string> = {
  backstep: "bg-rose-500/15 text-rose-400 border-rose-500/30",
  long_jump: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  unmarked_stretch: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  unanchored_ganzhi: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  ambiguous_era: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  calendar_note: "bg-gray-500/15 text-text-muted border-gray-500/30",
  term_collision: "bg-gray-500/15 text-text-muted border-gray-500/30",
  no_markers: "bg-gray-500/15 text-text-muted border-gray-500/30",
  too_short: "bg-gray-500/15 text-text-muted border-gray-500/30",
  no_order: "bg-gray-500/15 text-text-muted border-gray-500/30",
};

export function timelineIssueStyle(kind: string): string {
  return TIMELINE_ISSUE_STYLES[kind] ?? TIMELINE_ISSUE_STYLES.no_order;
}

/**
 * A distance in days, said the way a reader would say it rather than the way it was
 * measured. Months and years are the analysis' own approximations (30 and 365 days),
 * which the section states next to any number that uses them.
 */
export function timelineSpanLabel(days: number): string {
  const abs = Math.abs(days);
  if (abs < 1) return "不足一天";
  if (abs < 30) return `${abs} 天`;
  if (abs < 365) return `${Math.round(abs / 30)} 个月`;
  return `${(abs / 365).toFixed(1)} 年`;
}

/** The same distance with the direction it was measured in. */
export function timelineSignedSpan(days: number): string {
  if (days === 0) return "同一天";
  return `${days > 0 ? "+" : "−"}${timelineSpanLabel(days)}`;
}

