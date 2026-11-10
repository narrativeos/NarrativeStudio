import { GRAPH_BOX, graphFor, labelPosition } from "../../lib/graph";
import type { CharacterProfile } from "../../types";

interface RelationshipGraphProps {
  characters: CharacterProfile[];
  /** Characters to draw; the ring gets unreadable past a dozen. */
  limit?: number;
}

/** Line width for an edge: the heaviest line is thick, the lightest still visible. */
function edgeWidth(weight: number, maxWeight: number): number {
  if (maxWeight <= 0) return 0.6;
  return 0.6 + 2.6 * (weight / maxWeight);
}

/** A name short enough to sit beside a node without crowding its neighbour. */
function shortName(name: string): string {
  return name.length > 8 ? `${name.slice(0, 8)}…` : name;
}

/**
 * Dependency-free SVG relationship graph: the most-mentioned characters on a ring,
 * joined by the blocks they share.
 *
 * A line means only that two names appear in the same block. It says nothing about
 * what they are to each other — the analysis has no relation types, only co-occurrence.
 */
function RelationshipGraph({ characters, limit = 12 }: RelationshipGraphProps) {
  if (characters.length === 0) {
    return <p className="text-xs text-text-muted">未检测到人物实体，没有可画的关系。</p>;
  }

  const graph = graphFor(characters, limit);
  const hidden = characters.length - graph.nodes.length;

  return (
    <div>
      <svg
        role="img"
        aria-label={`人物共现关系图：${graph.nodes.length} 个角色，${graph.edges.length} 条共现连线。一条线只表示两个名字出现在同一个文本块里，不代表他们之间的关系。`}
        viewBox={`0 0 ${GRAPH_BOX.width} ${GRAPH_BOX.height}`}
        className="w-full"
      >
        {graph.edges.map((edge) => {
          const a = graph.nodes.find((n) => n.name === edge.a);
          const b = graph.nodes.find((n) => n.name === edge.b);
          if (!a || !b) return null;
          return (
            <line
              key={`${edge.a}\u0000${edge.b}`}
              x1={a.x}
              y1={a.y}
              x2={b.x}
              y2={b.y}
              stroke="var(--color-chart)"
              strokeOpacity={0.45}
              strokeWidth={edgeWidth(edge.weight, graph.maxWeight)}
            >
              <title>{`${edge.a} ↔ ${edge.b}: 同块共现 ${edge.weight} 次`}</title>
            </line>
          );
        })}
        {graph.nodes.map((node) => {
          const label = labelPosition(node);
          return (
            <g key={node.name}>
              <circle cx={node.x} cy={node.y} r={node.r} fill="var(--color-graph-node)" fillOpacity={0.85}>
                <title>{`${node.name}: 提及 ${node.mentions} 次`}</title>
              </circle>
              <text
                x={label.x}
                y={label.y}
                textAnchor={node.anchor}
                className="fill-text-muted"
                fontSize="11"
              >
                {shortName(node.name)}
              </text>
            </g>
          );
        })}
      </svg>
      <p className="text-xs text-text-muted mt-2">
        线只表示两个名字出现在同一个块里，不表示他们之间的关系；圈的大小是按提及次数。
        {hidden > 0 && `只画提及最多的 ${graph.nodes.length} 个，另有 ${hidden} 个未画。`}
      </p>
    </div>
  );
}

export default RelationshipGraph;