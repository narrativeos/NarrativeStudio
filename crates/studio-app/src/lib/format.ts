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
