/**
 * The analysis page's dimension sections, in the order the page renders them.
 *
 * The page is long: seventeen sections, most of them a table. This list is the
 * single place that knows what they are and which order they go in, so the jump
 * bar and the anchors cannot drift apart — a section that renders without an
 * `id`, or with one not listed here, fails the test in `sections.test.ts`
 * rather than quietly leaving a dead link in the bar.
 */
export interface AnalysisSection {
  id: string;
  label: string;
}

export const ANALYSIS_SECTIONS: AnalysisSection[] = [
  { id: "overview", label: "统计概览" },
  { id: "statistics", label: "统计分析" },
  { id: "text", label: "文本诊断" },
  { id: "structure", label: "结构分析" },
  { id: "narrative", label: "叙事分析" },
  { id: "plot", label: "情节分析" },
  { id: "pacing", label: "节奏分析" },
  { id: "conflict", label: "冲突分析" },
  { id: "theme", label: "主题分析" },
  { id: "setting", label: "场景/设定" },
  { id: "cliche", label: "陈词滥调" },
  { id: "dialogue", label: "对话/叙事比例" },
  { id: "explicit", label: "显式语言" },
  { id: "story-structure", label: "故事结构" },
  { id: "timeline", label: "时间线" },
  { id: "voice", label: "作者声音" },
  { id: "checklist", label: "最终审查清单" },
];

/**
 * Which section the reader is in.
 *
 * `tops` is each section's distance from the top of the viewport, in the same
 * order as `ids`. The active section is the last one that has passed the jump
 * bar; before the first one has, the bar highlights the first. Measuring from
 * the viewport rather than from a scroll offset keeps this right whichever
 * element actually scrolls, which is why the caller reads rectangles and this
 * does not have to know.
 */
export function activeSectionId(ids: string[], tops: number[]): string | null {
  let active: string | null = null;
  for (let i = 0; i < ids.length && i < tops.length; i++) {
    if (tops[i] <= 0) active = ids[i];
  }
  return active ?? ids[0] ?? null;
}
