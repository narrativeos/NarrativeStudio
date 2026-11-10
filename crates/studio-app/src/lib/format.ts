//! Presentation helpers shared by the analysis and comparison views.
//!
//! Kept free of JSX and Tauri imports so they can be unit-tested with Vitest
//! (see development.md §7.1: frontend unit tests cover components/utils).
//!
//! The localized label and badge-class tables live in `labels.ts`, kept
//! together so the mirror of the report's wording is one file to check; this
//! file keeps the number formatting and the analysis-level helpers. The
//! re-export below keeps every existing `lib/format` import site working.

export * from "./labels";

import type { ProjectAnalysis, T0Stats } from "../types";

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
  if (score >= 80) return "bg-good";
  if (score >= 60) return "bg-warn";
  return "bg-bad";
}

/** Tailwind text-color class for a 0-100 score (green / amber / red). */
export function scoreTextColor(score: number): string {
  if (score >= 80) return "text-good";
  if (score >= 60) return "text-warn";
  return "text-bad";
}

/**
 * A score that may be absent.
 *
 * Conflict and theme analysis withhold the score rather than reporting 0 when the
 * vocabulary they need is missing, so every view has one way to say "未评分".
 */
/** Whether the word-level numbers were measured at all.
 *
 *  A text-only import carries no segmentation, so the word count, the part of
 *  speech table, the adverb and adjective tables and the repeated phrases are
 *  all zero. Zero there means *not measured*, and rendering it as a number
 *  would report an absence as a result. */
export function wordsMeasured(t0: T0Stats): boolean {
  return t0.token_source !== "none";
}

/** What a text-only import cannot answer, in the order the page shows it.
 *
 *  The distinction the banner exists to make: an absent number is not evidence
 *  of an absent quality. */
export function unreadableSections(data: ProjectAnalysis): string[] {
  const missing: string[] = [];
  if (!wordsMeasured(data.t0)) {
    missing.push("词频与重复短语");
  }
  if (!data.plot.readable) {
    missing.push("情节线");
  }
  if (!data.setting.places_measured) {
    missing.push("地点清单");
  }
  if (data.theme.source === "none") {
    missing.push("主题词表");
  }
  return missing;
}

/** The banner sentence, or null when nothing is missing. */
export function coverageNote(data: ProjectAnalysis): string | null {
  const missing = unreadableSections(data);
  if (missing.length === 0) return null;
  return `本项目是纯文本导入，没有分词、词性与实体，因此${missing.join("、")}读不到。读不到不等于没有：这些数字没有被测量，而不是测量结果为零。`;
}

export function formatOptionalScore(score: number | null): string {
  return score === null ? "未评分" : `${formatScore(score)} / 100`;
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
