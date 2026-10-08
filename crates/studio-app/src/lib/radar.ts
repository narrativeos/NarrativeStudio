//! Polar geometry for the assessment radar chart.
//!
//! Kept out of the component and free of JSX so the math can be unit-tested with
//! Vitest, the same way the format helpers are.

export interface RadarPoint {
  x: number;
  y: number;
}

/** Angle of an axis in radians: axis 0 points straight up, the rest clockwise. */
export function radarAngle(axisIndex: number, axisCount: number): number {
  return -Math.PI / 2 + (axisIndex * 2 * Math.PI) / axisCount;
}

/**
 * Position of `axisIndex`'s vertex for a value on the 0-100 scale.
 *
 * Starting at the top and going clockwise is what readers expect from a radar, and
 * it keeps the first dimension (the one the assessment weights first) at the top.
 * Values outside 0-100 are clamped rather than allowed to burst the chart.
 */
export function radarPoint(
  cx: number,
  cy: number,
  radius: number,
  axisIndex: number,
  axisCount: number,
  value: number
): RadarPoint {
  const angle = radarAngle(axisIndex, axisCount);
  const r = (Math.max(0, Math.min(100, value)) / 100) * radius;
  return { x: cx + r * Math.cos(angle), y: cy + r * Math.sin(angle) };
}

/** The SVG `points` attribute joining one vertex per entry of `values`. */
export function radarPolygon(
  cx: number,
  cy: number,
  radius: number,
  values: number[]
): string {
  return values
    .map((value, i) => {
      const p = radarPoint(cx, cy, radius, i, values.length, value);
      return `${p.x.toFixed(1)},${p.y.toFixed(1)}`;
    })
    .join(" ");
}

/** `text-anchor` for a label placed just outside this axis. */
export function radarLabelAnchor(
  axisIndex: number,
  axisCount: number
): "start" | "middle" | "end" {
  const cos = Math.cos(radarAngle(axisIndex, axisCount));
  if (cos > 0.2) return "start";
  if (cos < -0.2) return "end";
  return "middle";
}