import { describe, expect, it } from "vitest";

import type { T0Stats } from "../types";
import {
  arcShapeLabel,
  conflictTypeLabel,
  conflictVerdictLabel,
  conflictVerdictStyle,
  entityTotal,
  formatFixed,
  formatInt,
  formatOptionalScore,
  formatPercent,
  formatScore,
  pacingVerdictLabel,
  pacingVerdictStyle,
  plotShapeLabel,
  plotShapeStyle,
  scoreColor,
  severityLabel,
  severityStyle,
  themeConcentrationLabel,
  themeReachLabel,
  themeReachStyle,
  themeSourceLabel,
} from "./format";

/** Only `entity_counts` matters for these helpers; the rest of T0Stats is unused. */
function t0With(entityCounts: [string, number][]): T0Stats {
  return { entity_counts: entityCounts } as T0Stats;
}

describe("entityTotal", () => {
  it("sums occurrences across categories", () => {
    expect(entityTotal(t0With([["PERSON", 3], ["LOCATION", 10], ["DATE", 2]]))).toBe(15);
  });

  it("is zero when no entities were detected", () => {
    expect(entityTotal(t0With([]))).toBe(0);
  });
});

describe("number formatting", () => {
  it("groups thousands (locale-aware separator)", () => {
    expect(formatInt(1234)).toMatch(/^1[,\s\u00A0]234$/);
  });

  it("renders scores as whole numbers", () => {
    expect(formatScore(86.4)).toBe("86");
    expect(formatScore(0)).toBe("0");
  });

  it("renders ratios as whole percents", () => {
    expect(formatPercent(0.427)).toBe("43%");
    expect(formatPercent(0)).toBe("0%");
    expect(formatPercent(1)).toBe("100%");
  });

  it("defaults to one decimal for averages", () => {
    expect(formatFixed(23.456)).toBe("23.5");
    expect(formatFixed(23.456, 2)).toBe("23.46");
  });
});

describe("scoreColor", () => {
  it("uses green at or above 80", () => {
    expect(scoreColor(80)).toBe("bg-emerald-400");
    expect(scoreColor(100)).toBe("bg-emerald-400");
  });

  it("uses amber between 60 and 79.99", () => {
    expect(scoreColor(79.9)).toBe("bg-amber-400");
    expect(scoreColor(60)).toBe("bg-amber-400");
  });

  it("uses red below 60", () => {
    expect(scoreColor(59.9)).toBe("bg-red-400");
    expect(scoreColor(0)).toBe("bg-red-400");
  });
});

describe("severity helpers", () => {
  it("maps known severities to badge classes and labels", () => {
    expect(severityStyle("high")).toContain("bg-red-500/15");
    expect(severityStyle("medium")).toContain("bg-amber-500/15");
    expect(severityLabel("high")).toBe("高");
    expect(severityLabel("low")).toBe("低");
  });

  it("falls back to the low style and the raw label for unknown severities", () => {
    expect(severityStyle("critical")).toBe(severityStyle("low"));
    expect(severityLabel("critical")).toBe("critical");
  });
});

describe("arcShapeLabel", () => {
  it("localizes the four known shapes", () => {
    expect(arcShapeLabel("mountain")).toBe("先扬后抑");
    expect(arcShapeLabel("rising")).toBe("渐强");
    expect(arcShapeLabel("falling")).toBe("渐弱");
    expect(arcShapeLabel("steady")).toBe("平稳");
  });

  it("passes unknown shapes through instead of hiding them", () => {
    expect(arcShapeLabel("wave")).toBe("wave");
  });
});

describe("plot shape helpers", () => {
  it("localizes every shape the analysis can emit", () => {
    expect(plotShapeLabel("sustained")).toBe("贯穿");
    expect(plotShapeLabel("intermittent")).toBe("断续");
    expect(plotShapeLabel("burst")).toBe("集中");
    expect(plotShapeLabel("setup_only")).toBe("仅铺垫");
    expect(plotShapeLabel("late_arrival")).toBe("迟到");
    expect(plotShapeLabel("minor")).toBe("次要");
  });

  it("passes unknown shapes through instead of hiding them", () => {
    expect(plotShapeLabel("wave")).toBe("wave");
  });

  it("marks the two shapes that ask for a rewrite as warnings", () => {
    expect(plotShapeStyle("setup_only")).toContain("text-red-400");
    expect(plotShapeStyle("late_arrival")).toContain("text-red-400");
    expect(plotShapeStyle("sustained")).toContain("text-emerald-400");
  });

  it("falls back to the neutral style for unknown shapes", () => {
    expect(plotShapeStyle("wave")).toBe(plotShapeStyle("minor"));
  });
});

describe("pacing verdict helpers", () => {
  it("localizes every verdict the analysis can emit", () => {
    expect(pacingVerdictLabel("flat")).toBe("单调");
    expect(pacingVerdictLabel("varied")).toBe("起伏");
    expect(pacingVerdictLabel("spiky")).toBe("跳跃");
  });

  it("passes an unknown verdict through instead of hiding it", () => {
    expect(pacingVerdictLabel("wave")).toBe("wave");
  });

  it("marks a curve that never moves as the case to fix", () => {
    expect(pacingVerdictStyle("flat")).toContain("text-red-400");
    expect(pacingVerdictStyle("varied")).toContain("text-emerald-400");
    expect(pacingVerdictStyle("spiky")).toContain("text-amber-400");
  });

  it("falls back to the warning style for an unknown verdict", () => {
    expect(pacingVerdictStyle("wave")).toBe(pacingVerdictStyle("flat"));
  });
});

describe("conflict helpers", () => {
  it("localizes every family the analysis can emit", () => {
    expect(conflictTypeLabel("interpersonal")).toBe("人际");
    expect(conflictTypeLabel("internal")).toBe("内心");
    expect(conflictTypeLabel("societal")).toBe("社会");
    expect(conflictTypeLabel("survival")).toBe("生存");
  });

  it("names the empty family as 无 rather than hiding it", () => {
    expect(conflictTypeLabel("")).toBe("无");
  });

  it("passes an unknown family through instead of hiding it", () => {
    expect(conflictTypeLabel("cosmic")).toBe("cosmic");
  });

  it("localizes every verdict the analysis can emit", () => {
    expect(conflictVerdictLabel("none")).toBe("未见用词");
    expect(conflictVerdictLabel("thin")).toBe("偏少");
    expect(conflictVerdictLabel("present")).toBe("适中");
    expect(conflictVerdictLabel("heavy")).toBe("密集");
  });

  it("keeps 未见用词 neutral: no vocabulary is not the same as no conflict", () => {
    expect(conflictVerdictStyle("none")).toContain("text-text-muted");
    expect(conflictVerdictStyle("thin")).toContain("text-amber-400");
    expect(conflictVerdictStyle("present")).toContain("text-emerald-400");
  });

  it("falls back to the neutral style for an unknown verdict", () => {
    expect(conflictVerdictStyle("wave")).toBe(conflictVerdictStyle("none"));
  });
});

describe("formatOptionalScore", () => {
  it("says 未评分 instead of inventing a zero", () => {
    expect(formatOptionalScore(null)).toBe("未评分");
  });

  it("formats a present score like the other scores", () => {
    expect(formatOptionalScore(72.4)).toBe("72 / 100");
  });
});

describe("theme labels", () => {
  it("names the vocabulary the analysis read", () => {
    expect(themeSourceLabel("noun_signals")).toBe("名词信号");
    expect(themeSourceLabel("noun_tokens")).toBe("名词词性");
    expect(themeSourceLabel("none")).toBe("无");
  });

  it("localizes every reach the analysis can emit", () => {
    expect(themeReachLabel("sustained")).toBe("贯穿");
    expect(themeReachLabel("recurring")).toBe("复现");
    expect(themeReachLabel("episodic")).toBe("偶发");
    expect(themeReachLabel("occasional")).toBe("零星");
  });

  it("reads a subject that runs through the document as the healthy case", () => {
    expect(themeReachStyle("sustained")).toContain("text-emerald-400");
    expect(themeReachStyle("episodic")).toContain("text-amber-400");
    expect(themeReachStyle("occasional")).toContain("text-text-muted");
  });

  it("falls back to the muted style for an unknown reach", () => {
    expect(themeReachStyle("scattered")).toBe(themeReachStyle("occasional"));
  });

  it("localizes where a theme's mentions sit", () => {
    expect(themeConcentrationLabel("spread")).toBe("均匀");
    expect(themeConcentrationLabel("front")).toBe("集中在开头");
    expect(themeConcentrationLabel("middle")).toBe("集中在中段");
    expect(themeConcentrationLabel("back")).toBe("集中在结尾");
  });

  it("passes an unknown label through instead of hiding it", () => {
    expect(themeSourceLabel("embeddings")).toBe("embeddings");
    expect(themeReachLabel("weekly")).toBe("weekly");
    expect(themeConcentrationLabel("sideways")).toBe("sideways");
  });
});
