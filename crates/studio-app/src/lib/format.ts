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
