interface LineChartProps {
  points: { label: string; value: number }[];
  /**
   * What the curve measures, for the accessible name.
   *
   * Required rather than defaulted: nine of these render on one page and a
   * generic "line chart" announced nine times tells a screen-reader nothing,
   * while the panel heading right above each one already knows the answer.
   */
  label: string;
  height?: number;
  color?: string;
}

/**
 * Dependency-free SVG line/area chart for sequential series (e.g. the
 * narrative arc intensity curve). Values are expected in 0..1.
 */
function LineChart({
  points,
  label,
  height = 180,
  color = "var(--color-chart)",
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

  // The accessible name carries the one thing the shape would otherwise have to
  // be seen for: where the peak is. An SVG with no role and no name is skipped
  // outright by a screen reader, so every chart on the page would be silent.
  const peak = points.reduce(
    (best, p, i) => (p.value > points[best].value ? i : best),
    0,
  );

  return (
    <svg
      role="img"
      aria-label={`${label}折线图，共 ${points.length} 个位置，最高点在「${points[peak].label}」`}
      viewBox={`0 0 ${width} ${height}`}
      className="w-full"
    >
      {[0.25, 0.5, 0.75, 1].map((g) => (
        <line
          key={g}
          x1={padX}
          x2={width - padX}
          y1={y(g)}
          y2={y(g)}
          stroke="var(--color-line)"
          strokeWidth="0.5"
          strokeDasharray="3 3"
        />
      ))}
      {/* The band under the line is the same hue, faint — one colour per series. */}
      <path d={area} fill={color} fillOpacity={0.15} />
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
          className="fill-text-muted"
          fontSize="10"
        >
          {points[i].label.length > 10 ? `${points[i].label.slice(0, 10)}…` : points[i].label}
        </text>
      ))}
    </svg>
  );
}

export default LineChart;
