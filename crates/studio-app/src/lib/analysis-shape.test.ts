import { describe, expect, it } from "vitest";
import { analysisShapeProblems } from "./analysis-shape";

/** A payload shaped the way the analysis page reads it. */
function complete() {
  const part = (extra: Record<string, unknown> = {}) => ({ ...extra });
  return {
    project_id: "p1",
    name: "测试稿",
    t0: part(),
    t1: part({ sections: [] }),
    plot: part(),
    pacing: part({ points: [] }),
    conflict: part({ points: [] }),
    theme: part(),
    setting: part(),
    cliche: part(),
    dialogue: part(),
    explicit: part(),
    structure: part(),
    timeline: part(),
    voice: part(),
    checklist: part(),
    assessment: part(),
    cached: true,
  };
}

describe("analysisShapeProblems", () => {
  it("accepts a payload the page can render", () => {
    expect(analysisShapeProblems(complete())).toEqual([]);
  });

  it("names a part dropped by an older cache", () => {
    const stale = complete() as Record<string, unknown>;
    delete stale.t1;
    expect(analysisShapeProblems(stale)).toEqual(["t1 缺失或不是对象"]);
  });

  it("catches a part that arrived as null", () => {
    // JSON gives null for a field the backend left unset, which is the one shape
    // a truthiness check at the call site would let straight through to a throw.
    const stale = { ...complete(), voice: null };
    expect(analysisShapeProblems(stale)).toContain("voice 缺失或不是对象");
  });

  it("catches a list the page spreads but that is not a list", () => {
    const stale = { ...complete(), t1: { sections: null } };
    expect(analysisShapeProblems(stale)).toContain("t1.sections 缺失或不是数组");
  });

  it("reports every problem at once, not only the first", () => {
    // One trip to the message is enough to see the whole gap between what is on
    // disk and what this build reads.
    const stale = complete() as Record<string, unknown>;
    delete stale.t1;
    delete stale.voice;
    // Both gaps in one pass rather than one per visit. A missing `t1` is not
    // also reported as a missing `t1.sections` — that is the same gap twice.
    expect(analysisShapeProblems(stale)).toEqual([
      "t1 缺失或不是对象",
      "voice 缺失或不是对象",
    ]);
  });

  it("rejects a payload that is not an object at all", () => {
    for (const value of [null, undefined, 7, "ok", []]) {
      expect(analysisShapeProblems(value).length).toBeGreaterThan(0);
    }
  });
});
