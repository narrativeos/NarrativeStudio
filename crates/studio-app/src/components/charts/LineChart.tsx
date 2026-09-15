interface LineChartProps {
  points: { label: string; value: number }[];
  height?: number;
  color?: string;
  fill?: string;
}

/**
 * Dependency-free SVG line/area chart for sequential series (e.g. the
 * narrative arc intensity curve). Values are expected in 0..1.
 */
function LineChart({
  points,
  height = 180,
  color = "#818cf8",
  fill = "rgba(129, 140, 248, 0.15)",
}: LineChartProps) {
  if (points.length === 0) {
    return <p className="text-xs text-text-muted">暂无数据</p>;
  }

  const width = 640;
  const padX = 10;
  const padTop = 14;
  const padBottom = 26;
  const innerW = width - padX * 2;
  const innerH = height - padTop - padBottom;

  const x = (i: number) =>
    padX + (points.length === 1 ? innerW / 2 : (i / (points.length - 1)) * innerW);
  const y = (v: number) => padTop + (1 - Math.max(0, Math.min(1, v))) * innerH;

  const path = points
    .map((p, i) => `${i === 0 ? "M" : "L"} ${x(i).toFixed(1)} ${y(p.value).toFixed(1)}`)
    .join(" ");
  const area = `${path} L ${x(points.length - 1).toFixed(1)} ${y(0).toFixed(1)} L ${x(0).toFixed(1)} ${y(0).toFixed(1)} Z`;

  // Show at most 5 x-axis labels: first, evenly spaced, last.
  const labelCount = Math.min(5, points.length);
  const labelIdx =
    labelCount === 1
      ? [0]
      : Array.from({ length: labelCount }, (_, i) =>
          Math.round((i / (labelCount - 1)) * (points.length - 1))
        );

  return (
    <svg viewBox={`0 0 ${width} ${height}`} className="w-full">
      {[0.25, 0.5, 0.75, 1].map((g) => (
        <line
          key={g}
          x1={padX}
          x2={width - padX}
          y1={y(g)}
          y2={y(g)}
          stroke="#374151"
          strokeWidth="0.5"
          strokeDasharray="3 3"
        />
      ))}
      <path d={area} fill={fill} />
      <path
        d={path}
        fill="none"
        stroke={color}
        strokeWidth="2"
        strokeLinejoin="round"
        strokeLinecap="round"
      />
      {points.map((p, i) => (
        <circle key={i} cx={x(i)} cy={y(p.value)} r="3" fill={color}>
          <title>{`${p.label}: ${(p.value * 100).toFixed(0)}`}</title>
        </circle>
      ))}
      {labelIdx.map((i) => (
        <text
          key={i}
          x={x(i)}
          y={height - 8}
          textAnchor={i === 0 ? "start" : i === points.length - 1 ? "end" : "middle"}
          className="fill-gray-500"
          fontSize="10"
        >
          {points[i].label.length > 10 ? `${points[i].label.slice(0, 10)}…` : points[i].label}
        </text>
      ))}
    </svg>
  );
}

export default LineChart;
