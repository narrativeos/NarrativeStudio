import { radarLabelAnchor, radarPoint, radarPolygon } from "../../lib/radar";

interface RadarChartProps {
  /** Dimensions in display order; `score` is on the 0-100 scale. */
  dimensions: { label: string; score: number }[];
  size?: number;
  color?: string;
  fill?: string;
}

/**
 * Dependency-free SVG radar chart for the assessment dimensions.
 *
 * A radar shows the *shape* of a portrait — where the text is strong and where it
 * is weak — which a column of bars does not, and the five assessment dimensions are
 * few enough for every axis to stay readable.
 */
function RadarChart({
  dimensions,
  size = 240,
  color = "#818cf8",
  fill = "rgba(129, 140, 248, 0.18)",
}: RadarChartProps) {
  if (dimensions.length < 3) {
    // Fewer than three axes cannot enclose an area; a bar list says the same thing.
    return <p className="text-xs text-text-muted">维度不足，无法绘制雷达图</p>;
  }

  const c = size / 2;
  // Leave room outside the outer ring for the labels.
  const radius = c - 46;
  const scores = dimensions.map((d) => d.score);

  return (
    <svg viewBox={`0 0 ${size} ${size}`} className="w-full max-w-[260px]">
      {/* Rings: 25 / 50 / 75 / 100 */}
      {[0.25, 0.5, 0.75, 1].map((ring) => (
        <polygon
          key={ring}
          points={radarPolygon(c, c, radius * ring, scores.map(() => 100))}
          fill="none"
          stroke="#374151"
          strokeWidth="0.5"
          strokeDasharray="3 3"
        />
      ))}

      {/* Axes */}
      {dimensions.map((_, i) => {
        const outer = radarPoint(c, c, radius, i, dimensions.length, 100);
        return (
          <line
            key={i}
            x1={c}
            y1={c}
            x2={outer.x}
            y2={outer.y}
            stroke="#374151"
            strokeWidth="0.5"
          />
        );
      })}

      {/* The portrait itself */}
      <polygon
        points={radarPolygon(c, c, radius, scores)}
        fill={fill}
        stroke={color}
        strokeWidth="2"
        strokeLinejoin="round"
      />

      {dimensions.map((d, i) => {
        const p = radarPoint(c, c, radius, i, dimensions.length, d.score);
        return (
          <circle key={i} cx={p.x} cy={p.y} r="3" fill={color}>
            <title>{`${d.label}: ${Math.round(d.score)} / 100`}</title>
          </circle>
        );
      })}

      {/* Labels sit just outside the outer ring, with the score beside them */}
      {dimensions.map((d, i) => {
        const p = radarPoint(c, c, radius + 14, i, dimensions.length, 100);
        return (
          <text
            key={i}
            x={p.x}
            y={p.y}
            textAnchor={radarLabelAnchor(i, dimensions.length)}
            dominantBaseline="middle"
            className="fill-gray-400"
            fontSize="10"
          >
            {d.label} {Math.round(d.score)}
          </text>
        );
      })}
    </svg>
  );
}

export default RadarChart;