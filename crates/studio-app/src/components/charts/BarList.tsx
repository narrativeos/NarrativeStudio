interface BarListProps {
  items: { label: string; value: number }[];
  colorClass?: string;
  limit?: number;
  formatValue?: (v: number) => string;
}

/**
 * Horizontal bar list — a dependency-free SVG/DOM chart for ranked
 * distributions (word frequency, entities, sections, ...). Bars are scaled
 * relative to the largest visible value.
 */
function BarList({ items, colorClass = "bg-accent", limit, formatValue }: BarListProps) {
  const shown = limit ? items.slice(0, limit) : items;
  if (shown.length === 0) {
    return <p className="text-xs text-text-muted">暂无数据</p>;
  }
  const max = Math.max(1, ...shown.map((i) => i.value));
  return (
    <div className="space-y-1.5">
      {shown.map((item, idx) => (
        <div key={`${item.label}-${idx}`} className="flex items-center gap-2 text-xs">
          <span className="w-28 shrink-0 truncate text-text" title={item.label}>
            {item.label}
          </span>
          <div className="flex-1 h-4 bg-gray-800/60 rounded overflow-hidden">
            <div
              className={`h-full ${colorClass} rounded`}
              style={{ width: `${(item.value / max) * 100}%` }}
            />
          </div>
          <span className="w-14 shrink-0 text-right text-text-muted tabular-nums">
            {formatValue ? formatValue(item.value) : item.value.toLocaleString()}
          </span>
        </div>
      ))}
    </div>
  );
}

export default BarList;
