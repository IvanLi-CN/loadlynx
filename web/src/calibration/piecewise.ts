import Decimal from "decimal.js";

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

/**
 * Decimal.js-backed version of piecewiseLinear. Use this for UI/calibration code
 * paths that must not lose precision to IEEE-754 rounding.
 */
export function piecewiseLinearDecimal(points: Point[], x: number): Decimal {
  const dataset = normalizePoints(points);
  const dx = new Decimal(x);

  if (dataset.length === 0) {
    return dx; // Fallback: 1:1 if no points
  }

  if (dataset.length === 1) {
    const p = dataset[0];
    const px = new Decimal(p.x);
    if (px.isZero()) return new Decimal(p.y);
    return new Decimal(p.y).div(px).mul(dx);
  }

  for (let i = 0; i < dataset.length - 1; i++) {
    const p0 = dataset[i];
    const p1 = dataset[i + 1];
    if (x >= p0.x && x <= p1.x) {
      const p0x = new Decimal(p0.x);
      const p1x = new Decimal(p1.x);
      const t = dx.minus(p0x).div(p1x.minus(p0x));
      return new Decimal(p0.y).plus(t.mul(new Decimal(p1.y).minus(p0.y)));
    }
  }

  if (x < dataset[0].x) {
    const p0 = dataset[0];
    const p1 = dataset[1];
    const p0x = new Decimal(p0.x);
    const p1x = new Decimal(p1.x);
    const slope = new Decimal(p1.y).minus(p0.y).div(p1x.minus(p0x));
    return new Decimal(p0.y).plus(slope.mul(dx.minus(p0x)));
  }

  const n = dataset.length;
  const pLast = dataset[n - 1];
  const pPrev = dataset[n - 2];
  const pLastX = new Decimal(pLast.x);
  const pPrevX = new Decimal(pPrev.x);
  const slope = new Decimal(pLast.y).minus(pPrev.y).div(pLastX.minus(pPrevX));
  return new Decimal(pLast.y).plus(slope.mul(dx.minus(pLastX)));
}
