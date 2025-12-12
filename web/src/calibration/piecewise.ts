export interface Point {
  x: number; // Raw value
  y: number; // Physical value
}

/**
 * Normalizes a list of points:
 * 1. Sorts by x (raw) ascending.
 * 2. Removes duplicates (keeping the last occurrence of the same x).
 */
export function normalizePoints(points: Point[]): Point[] {
  if (points.length === 0) return [];

  // Sort by x
  const sorted = [...points].sort((a, b) => a.x - b.x);

  // Dedup - keep last
  const unique: Point[] = [];
  for (let i = 0; i < sorted.length; i++) {
    const current = sorted[i];
    const next = sorted[i + 1];
    if (next && next.x === current.x) {
      continue; // Skip this one, it's a duplicate and not the last one
    }
    unique.push(current);
  }

  return unique;
}

/**
 * Calculates Y for a given X using piecewise linear interpolation.
 * Supports extrapolation.
 */
export function piecewiseLinear(points: Point[], x: number): number {
  const dataset = normalizePoints(points);

  if (dataset.length === 0) {
    return x; // Fallback: 1:1 if no points
  }

  if (dataset.length === 1) {
    // 1-point calibration: Assume 0,0 is the other point (linear gain)
    // y = (y1/x1) * x
    const p = dataset[0];
    if (p.x === 0) return p.y; // Avoid division by zero, though unlikely 0->non-zero mapping without offset
    return (p.y / p.x) * x;
  }

  // Find the segment
  for (let i = 0; i < dataset.length - 1; i++) {
    const p0 = dataset[i];
    const p1 = dataset[i + 1];

    if (x >= p0.x && x <= p1.x) {
      // Interpolate
      const t = (x - p0.x) / (p1.x - p0.x);
      return p0.y + t * (p1.y - p0.y);
    }
  }

  // Extrapolate
  if (x < dataset[0].x) {
    // Below first point
    const p0 = dataset[0];
    const p1 = dataset[1]; // We know length >= 2
    const slope = (p1.y - p0.y) / (p1.x - p0.x);
    return p0.y + slope * (x - p0.x);
  } else {
    // Above last point
    const n = dataset.length;
    const p_last = dataset[n - 1];
    const p_prev = dataset[n - 2];
    const slope = (p_last.y - p_prev.y) / (p_last.x - p_prev.x);
    return p_last.y + slope * (x - p_last.x);
  }
}
