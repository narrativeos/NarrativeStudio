import { describe, expect, it } from "vitest";

import type { T0Stats } from "../types";
import {
  arcShapeLabel,
  entityTotal,
  formatFixed,
  formatInt,
  formatPercent,
  formatScore,
  scoreColor,
  severityLabel,
  severityStyle,
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
