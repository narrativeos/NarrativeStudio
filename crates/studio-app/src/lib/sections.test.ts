import { describe, expect, it } from "vitest";
import { activeSectionId, ANALYSIS_SECTIONS } from "./sections";

// The page sources, as text, so the anchors they render can be checked against
// the list the jump bar is built from. Seven sections are written in the page
// itself and ten in their own component, so every source is scanned. Same trick
// `theme.test.ts` uses.
const raw = import.meta.glob("../**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const sources = Object.entries(raw)
  .filter(([path]) => !path.endsWith(".test.ts"))
  .map(([, text]) => text)
  .join("\n");

/** Every `<section>` the sources render, with its anchor. */
function renderedSections() {
  return [...sources.matchAll(/<section\b([^>]*)>/g)].map(
    (m) => /id="([a-z-]+)"/.exec(m[1])?.[1] ?? null,
  );
}

describe("analysis sections", () => {
  it("lists each dimension once", () => {
    const ids = ANALYSIS_SECTIONS.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids.length).toBeGreaterThan(10);
  });

  it("names every section the page renders an anchor for", () => {
    // A section that renders an `id` nobody lists is a dead anchor; one that is
    // listed but never rendered is a link to nowhere. Both are this test.
    const rendered = renderedSections();
    for (const id of ANALYSIS_SECTIONS.map((s) => s.id)) {
      expect(rendered, `${id} is listed but no source renders it`).toContain(id);
    }
    for (const id of rendered.filter((x): x is string => x !== null)) {
      expect(
        ANALYSIS_SECTIONS.some((s) => s.id === id),
        `${id} is rendered but not in the jump bar`,
      ).toBe(true);
    }
  });

  it("gives every section the anchor margin, so a jump does not land under the bar", () => {
    expect(renderedSections().filter((id) => id !== null).length).toBe(
      ANALYSIS_SECTIONS.length,
    );
    const anchored = [...sources.matchAll(/<section\b[^>]*className=\{SECTION_ANCHOR\}>/g)];
    expect(anchored.length).toBe(ANALYSIS_SECTIONS.length);
  });
});


describe("active section", () => {
  const ids = ["a", "b", "c"];

  it("takes the last section that has passed the bar", () => {
    // a above the bar, b above it too, c still below.
    expect(activeSectionId(ids, [-400, -20, 300])).toBe("b");
  });

  it("highlights the first section before any of them has arrived", () => {
    expect(activeSectionId(ids, [120, 600, 900])).toBe("a");
  });

  it("follows the reader back up", () => {
    expect(activeSectionId(ids, [500, 900, 1400])).toBe("a");
    expect(activeSectionId(ids, [-10, 400, 900])).toBe("a");
  });

  it("is null only when there is nothing to point at", () => {
    expect(activeSectionId([], [])).toBeNull();
  });

  it("ignores a section it has no measurement for", () => {
    expect(activeSectionId(ids, [-10])).toBe("a");
  });
});
