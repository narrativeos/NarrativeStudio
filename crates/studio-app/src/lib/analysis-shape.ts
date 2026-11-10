/**
 * Whether what came back can actually be rendered.
 *
 * `invoke<ProjectAnalysis>(…)` is a cast, not a check: it tells the compiler to
 * trust the JSON, and the analysis page then dereferences every part of it
 * (`t1.sections`, `conflict.points`) without looking. A payload that is missing
 * one of them does not render a blank panel — it throws during render, React
 * tears down the whole tree, and the window keeps the pixels it last painted
 * while every button in it goes inert. That is how a stale analysis cache, or a
 * dimension added by a newer engine, turns into "nothing is clickable" with no
 * message anywhere.
 *
 * So the shape is checked once, here, where it can be tested, and the page gets
 * to say what is missing instead of dying.
 */
const ANALYSIS_PARTS = [
  "t0",
  "t1",
  "plot",
  "pacing",
  "conflict",
  "theme",
  "setting",
  "cliche",
  "dialogue",
  "explicit",
  "structure",
  "timeline",
  "voice",
  "checklist",
  "assessment",
] as const;

/**
 * Lists the parts of a `ProjectAnalysis` the page cannot work without.
 *
 * The result is meant for a message, so it names things the way the page does.
 * An empty array means every part is present and shaped as the render expects.
 */
export function analysisShapeProblems(value: unknown): string[] {
  if (typeof value !== "object" || value === null) {
    return ["分析结果不是一个对象"];
  }
  const record = value as Record<string, unknown>;
  const problems: string[] = [];

  for (const part of ANALYSIS_PARTS) {
    const field = record[part];
    if (field === null || typeof field !== "object") {
      problems.push(`${part} 缺失或不是对象`);
    }
  }

  // Three of these are spread or measured directly during render, so being an
  // object is not enough — the page needs an array to walk. Only checked where
  // the part above it arrived: a missing `t1` is one problem, not two.
  const lists: Array<[string, unknown]> = [
    ["t1.sections", (record.t1 as { sections?: unknown } | undefined)?.sections],
    ["pacing.points", (record.pacing as { points?: unknown } | undefined)?.points],
    ["conflict.points", (record.conflict as { points?: unknown } | undefined)?.points],
  ];
  for (const [name, list] of lists) {
    const parent = name.split(".")[0];
    if (problems.some((p) => p.startsWith(`${parent} `))) continue;
    if (!Array.isArray(list)) problems.push(`${name} 缺失或不是数组`);
  }

  return problems;
}
