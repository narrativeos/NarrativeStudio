//! Geometry and edge building for the character relationship graph.
//!
//! Kept out of the component and free of JSX so the math can be unit-tested with
//! Vitest, the same way the radar geometry is.
//!
//! The layout is a ring, not a force simulation. A force layout moves its nodes
//! between runs and settles differently depending on iteration order, so the same
//! document would draw a different picture on each visit and nothing here could be
//! asserted. A ring ordered by mention count puts the most-mentioned character at the
//! top and is the same drawing every time.

import type { CharacterProfile } from "../types";

/** One line between two characters: they share this many blocks. */
export interface GraphEdge {
  /** The two ends, always in sorted order so a pair appears once. */
  a: string;
  b: string;
  /** Same-block co-occurrences; the number the line's width is scaled by. */
  weight: number;
}

/** A character placed on the ring. */
export interface GraphNode {
  name: string;
  mentions: number;
  x: number;
  y: number;
  /** Circle radius in SVG units, scaled by mention count. */
  r: number;
  /** `text-anchor` for the label drawn just outside this node. */
  anchor: "start" | "middle" | "end";
  /**
   * Whether a centred label belongs above the node rather than below it. Only matters
   * for the nodes near the top and bottom of the ring, where the label cannot go to
   * one side without crossing the ring itself.
   */
  above: boolean;
}

/** The whole drawing: nodes in ring order plus the edges between them. */
export interface GraphLayout {
  nodes: GraphNode[];
  edges: GraphEdge[];
  /** The heaviest edge, so the component can scale line widths. */
  maxWeight: number;
}

/**
 * Compare two names the way the backend sorts them: by code unit.
 *
 * `localeCompare` would consult the runtime's collation, which orders Chinese by
 * pinyin on one machine and by stroke on another. The Rust side sorts names by bytes,
 * so the graph picks the same characters on a tie only if it compares them the same
 * blunt way.
 */
function compareNames(a: string, b: string): number {
  if (a < b) return -1;
  return a > b ? 1 : 0;
}

/** Characters worth drawing: the most-mentioned ones, ties by name. */
export function graphNodes(characters: CharacterProfile[], limit: number): CharacterProfile[] {
  return [...characters]
    .sort((x, y) => y.mentions - x.mentions || compareNames(x.name, y.name))
    .slice(0, Math.max(0, limit));
}

/**
 * The lines between the drawn characters.
 *
 * Each profile lists its own top partners, so a pair arrives twice; the ends are
 * sorted before keying so each pair is kept once, and the larger of the two counts
 * wins in case the two sides ever disagree. A partner that is not itself drawn is
 * dropped: a line to an absent node cannot be drawn, and a node for a single
 * mention would clutter the ring for nothing.
 */
export function graphEdges(
  characters: CharacterProfile[],
  drawn: Set<string>
): GraphEdge[] {
  const best = new Map<string, GraphEdge>();
  for (const character of characters) {
    if (!drawn.has(character.name)) {
      continue;
    }
    for (const [partner, count] of character.top_cooccurrences) {
      if (!drawn.has(partner) || partner === character.name) {
        continue;
      }
      const [a, b] = [character.name, partner].sort();
      const key = `${a}\u0000${b}`;
      const existing = best.get(key);
      if (existing === undefined || count > existing.weight) {
        best.set(key, { a, b, weight: count });
      }
    }
  }
  return [...best.values()].sort(
    (x, y) => y.weight - x.weight || compareNames(x.a, y.a) || compareNames(x.b, y.b)
  );
}

/** Angle of ring slot `index` in radians: slot 0 points straight up, the rest clockwise. */
export function ringAngle(index: number, count: number): number {
  return -Math.PI / 2 + (index * 2 * Math.PI) / Math.max(1, count);
}

/**
 * Place the characters on a ring.
 *
 * Node size follows mention count on a square-root scale: a character named forty
 * times should look bigger than one named ten times, but not four times the disc.
 * The square root keeps the largest node from swallowing the chart on documents
 * where one character dominates.
 */
export function ringLayout(
  characters: CharacterProfile[],
  cx: number,
  cy: number,
  ringRadius: number,
  maxNodeRadius: number
): GraphNode[] {
  const maxMentions = Math.max(1, ...characters.map((c) => c.mentions));
  const count = characters.length;
  return characters.map((character, index) => {
    const angle = ringAngle(index, count);
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    return {
      name: character.name,
      mentions: character.mentions,
      x: cx + ringRadius * cos,
      y: cy + ringRadius * sin,
      r: maxNodeRadius * Math.sqrt(character.mentions / maxMentions),
      anchor: cos > 0.2 ? "start" : cos < -0.2 ? "end" : "middle",
      above: sin < 0,
    };
  });
}

/**
 * The canvas the graph is drawn on.
 *
 * One place for the numbers so the layout can be checked against them: a ring that
 * fits with twelve nodes and eight-character names is a ring that fits, and a test
 * can say so.
 */
export const GRAPH_BOX = {
  width: 640,
  height: 340,
  cx: 320,
  cy: 170,
  ringRadius: 118,
  maxNodeRadius: 13,
} as const;

/** The layout on the shared canvas: what the component draws is what this returns. */
export function graphFor(characters: CharacterProfile[], limit: number): GraphLayout {
  return buildGraph(
    characters,
    GRAPH_BOX.cx,
    GRAPH_BOX.cy,
    GRAPH_BOX.ringRadius,
    GRAPH_BOX.maxNodeRadius,
    limit
  );
}

/** Where a node's label sits: outside the node, never across the ring. */
export function labelPosition(node: GraphNode): { x: number; y: number } {
  if (node.anchor === "start") {
    return { x: node.x + node.r + 4, y: node.y + 4 };
  }
  if (node.anchor === "end") {
    return { x: node.x - node.r - 4, y: node.y + 4 };
  }
  // Top and bottom of the ring: the name goes outside the circle, not into it, or it
  // would sit on top of the lines leaving that node.
  return { x: node.x, y: node.above ? node.y - node.r - 8 : node.y + node.r + 12 };
}

/** The whole graph for one document's character profiles. */
export function buildGraph(
  characters: CharacterProfile[],
  cx: number,
  cy: number,
  ringRadius: number,
  maxNodeRadius: number,
  limit: number
): GraphLayout {
  const picked = graphNodes(characters, limit);
  const nodes = ringLayout(picked, cx, cy, ringRadius, maxNodeRadius);
  const edges = graphEdges(characters, new Set(nodes.map((n) => n.name)));
  return { nodes, edges, maxWeight: Math.max(0, ...edges.map((e) => e.weight)) };
}