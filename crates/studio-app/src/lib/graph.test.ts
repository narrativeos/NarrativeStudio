import { describe, expect, it } from "vitest";
import {
  GRAPH_BOX,
  buildGraph,
  graphEdges,
  graphFor,
  graphNodes,
  labelPosition,
  ringAngle,
  ringLayout,
} from "./graph";
import type { GraphNode } from "./graph";
import type { CharacterProfile } from "../types";

function profile(name: string, mentions: number, partners: [string, number][] = []): CharacterProfile {
  return {
    name,
    mentions,
    first_section: "第一章",
    last_section: "第二章",
    span_ratio: 1,
    top_cooccurrences: partners,
  };
}

describe("graphNodes", () => {
  it("keeps the most-mentioned characters", () => {
    const picked = graphNodes(
      [profile("甲", 3), profile("乙", 9), profile("丙", 5)],
      2
    );
    expect(picked.map((c) => c.name)).toEqual(["乙", "丙"]);
  });

  it("breaks mention ties by code unit, the way the backend sorts names", () => {
    // 乙 is U+4E59 and 甲 is U+7532, so 乙 comes first when names are compared as
    // code units. A locale-aware compare would order them by pinyin instead and pick
    // different characters than the Rust side lists.
    const picked = graphNodes([profile("乙", 4), profile("甲", 4)], 2);
    expect(picked.map((c) => c.name)).toEqual(["乙", "甲"]);
  });

  it("does not mutate the input", () => {
    const input = [profile("甲", 1), profile("乙", 9)];
    graphNodes(input, 2);
    expect(input.map((c) => c.name)).toEqual(["甲", "乙"]);
  });
});

describe("graphEdges", () => {
  it("keeps one line per pair even though each side lists the other", () => {
    const characters = [profile("甲", 5, [["乙", 3]]), profile("乙", 4, [["甲", 3]])];
    const edges = graphEdges(characters, new Set(["甲", "乙"]));
    expect(edges).toHaveLength(1);
    expect(edges[0]).toEqual({ a: "乙", b: "甲", weight: 3 });
  });

  it("sorts the ends so the pair is named the same way every run", () => {
    const forward = graphEdges([profile("甲", 5, [["乙", 2]])], new Set(["甲", "乙"]));
    const backward = graphEdges([profile("乙", 5, [["甲", 2]])], new Set(["甲", "乙"]));
    expect(forward).toEqual(backward);
  });

  it("drops a partner that is not drawn", () => {
    const edges = graphEdges([profile("甲", 5, [["乙", 9]])], new Set(["甲"]));
    expect(edges).toEqual([]);
  });

  it("ignores a character paired with itself", () => {
    const edges = graphEdges([profile("甲", 5, [["甲", 9]])], new Set(["甲"]));
    expect(edges).toEqual([]);
  });

  it("orders the heaviest line first", () => {
    const characters = [
      profile("甲", 9, [["乙", 1], ["丙", 7]]),
      profile("乙", 5, [["甲", 1]]),
      profile("丙", 5, [["甲", 7]]),
    ];
    const edges = graphEdges(characters, new Set(["甲", "乙", "丙"]));
    expect(edges.map((e) => e.weight)).toEqual([7, 1]);
  });
});

describe("ringAngle", () => {
  it("starts at the top and divides the ring evenly", () => {
    expect(ringAngle(0, 6)).toBeCloseTo(-Math.PI / 2);
    expect(ringAngle(1, 6) - ringAngle(0, 6)).toBeCloseTo((2 * Math.PI) / 6);
  });

  it("does not divide by zero for an empty ring", () => {
    expect(Number.isFinite(ringAngle(0, 0))).toBe(true);
  });
});

describe("ringLayout", () => {
  it("puts the first character straight up", () => {
    const nodes = ringLayout([profile("甲", 5)], 100, 100, 50, 8);
    expect(nodes[0].x).toBeCloseTo(100);
    expect(nodes[0].y).toBeCloseTo(50);
  });

  it("sizes the most-mentioned character at the cap and shrinks the rest", () => {
    const nodes = ringLayout([profile("甲", 16), profile("乙", 4)], 100, 100, 50, 10);
    expect(nodes[0].r).toBeCloseTo(10);
    // Half the mentions on a square-root scale is not half the disc.
    expect(nodes[1].r).toBeCloseTo(5);
  });

  it("anchors labels away from the ring so text does not cross it", () => {
    const nodes = ringLayout([profile("甲", 4), profile("乙", 3)], 100, 100, 50, 8);
    // Two nodes: one at the top, one at the bottom — both centred.
    expect(nodes.every((n) => n.anchor === "middle")).toBe(true);
  });

  it("marks the top half of the ring as needing its label above the node", () => {
    const nodes = ringLayout([profile("甲", 4), profile("乙", 3)], 100, 100, 50, 8);
    expect(nodes[0].above).toBe(true);
    expect(nodes[1].above).toBe(false);
  });
});

describe("graphFor", () => {
  it("keeps twelve nodes and their labels inside the canvas", () => {
    // Twelve characters, each named long enough to be truncated to the label cap the
    // component draws: the widest case the ring has to survive.
    const characters = Array.from({ length: 12 }, (_, i) =>
      profile(`角色${i + 1}一二三四五六七八`, 20 - i, [["角色1一二三四五六七八", 3]])
    );
    const graph = graphFor(characters, 12);
    // Eight characters plus the ellipsis, at the component's font size of 11.
    const labelWidth = 9 * 11;
    const labelHeight = 11;
    expect(graph.nodes).toHaveLength(12);
    for (const node of graph.nodes) {
      const label = labelPosition(node);
      const left =
        node.anchor === "end"
          ? label.x - labelWidth
          : node.anchor === "middle"
            ? label.x - labelWidth / 2
            : label.x;
      expect(left).toBeGreaterThanOrEqual(0);
      expect(left + labelWidth).toBeLessThanOrEqual(GRAPH_BOX.width);
      expect(label.y - labelHeight).toBeGreaterThanOrEqual(0);
      expect(label.y).toBeLessThanOrEqual(GRAPH_BOX.height);
      expect(node.x - node.r).toBeGreaterThanOrEqual(0);
      expect(node.x + node.r).toBeLessThanOrEqual(GRAPH_BOX.width);
      expect(node.y - node.r).toBeGreaterThanOrEqual(0);
      expect(node.y + node.r).toBeLessThanOrEqual(GRAPH_BOX.height);
    }
  });
});

describe("labelPosition", () => {
  function node(over: Partial<GraphNode>): GraphNode {
    return {
      name: "甲",
      mentions: 5,
      x: 200,
      y: 100,
      r: 10,
      anchor: "middle",
      above: false,
      ...over,
    };
  }

  it("puts a right-side label to the right of its node", () => {
    expect(labelPosition(node({ anchor: "start" }))).toEqual({ x: 214, y: 104 });
  });

  it("puts a left-side label to the left of its node", () => {
    expect(labelPosition(node({ anchor: "end" }))).toEqual({ x: 186, y: 104 });
  });

  it("lifts a top-of-ring label clear of the lines leaving the node", () => {
    const p = labelPosition(node({ anchor: "middle", above: true, y: 52, r: 13 }));
    expect(p.y).toBeLessThan(52 - 13);
  });

  it("drops a bottom-of-ring label below the node", () => {
    const p = labelPosition(node({ anchor: "middle", above: false, y: 200, r: 13 }));
    expect(p.y).toBeGreaterThan(200 + 13);
  });
});

describe("buildGraph", () => {
  it("draws only the picked characters and the lines between them", () => {
    const characters = [
      profile("甲", 9, [["乙", 4], ["丁", 2]]),
      profile("乙", 7, [["甲", 4]]),
      profile("丙", 1),
      profile("丁", 6, [["甲", 2]]),
    ];
    const graph = buildGraph(characters, 100, 100, 50, 8, 3);
    expect(graph.nodes.map((n) => n.name)).toEqual(["甲", "乙", "丁"]);
    expect(graph.edges).toHaveLength(2);
    expect(graph.maxWeight).toBe(4);
  });

  it("reports no heaviest line when nothing is connected", () => {
    const graph = buildGraph([profile("甲", 3)], 100, 100, 50, 8, 5);
    expect(graph.edges).toEqual([]);
    expect(graph.maxWeight).toBe(0);
  });

  it("gives the same drawing for the same input every time", () => {
    const characters = [
      profile("甲", 9, [["乙", 4]]),
      profile("乙", 7, [["甲", 4]]),
      profile("丙", 5),
    ];
    const first = buildGraph(characters, 100, 100, 50, 8, 5);
    const second = buildGraph(characters, 100, 100, 50, 8, 5);
    expect(first).toEqual(second);
  });
});