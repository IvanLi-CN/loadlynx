import { describe, expect, it } from "bun:test";
import { normalizePoints, piecewiseLinear } from "./piecewise.ts";

describe("calibration/piecewise", () => {
  describe("normalizePoints", () => {
    it("sorts points by x", () => {
      const input = [
        { x: 10, y: 1 },
        { x: 5, y: 0.5 },
        { x: 20, y: 2 },
      ];
      const output = normalizePoints(input);
      expect(output).toEqual([
        { x: 5, y: 0.5 },
        { x: 10, y: 1 },
        { x: 20, y: 2 },
      ]);
    });

    it("removes duplicates, keeping the last one", () => {
      const input = [
        { x: 10, y: 1 },
        { x: 10, y: 1.5 }, // Should keep this
        { x: 5, y: 0.5 },
      ];
      const output = normalizePoints(input);
      expect(output).toEqual([
        { x: 5, y: 0.5 },
        { x: 10, y: 1.5 },
      ]);
    });

    it("handles empty list", () => {
      expect(normalizePoints([])).toEqual([]);
    });
  });

  describe("piecewiseLinear", () => {
    it("returns x when no points provided", () => {
      expect(piecewiseLinear([], 123)).toBe(123);
    });

    it("uses 0,0 reference for single point (gain only)", () => {
      const points = [{ x: 100, y: 10 }];
      expect(piecewiseLinear(points, 50)).toBe(5);
      expect(piecewiseLinear(points, 200)).toBe(20);
    });

    it("interpolates between two points", () => {
      const points = [
        { x: 0, y: 0 },
        { x: 100, y: 10 },
      ];
      expect(piecewiseLinear(points, 50)).toBe(5);
      expect(piecewiseLinear(points, 25)).toBe(2.5);
    });

    it("extrapolates below first point", () => {
      const points = [
        { x: 10, y: 1 },
        { x: 20, y: 2 },
      ];
      expect(piecewiseLinear(points, 0)).toBe(0);
      expect(piecewiseLinear(points, 5)).toBe(0.5);
    });

    it("extrapolates above last point", () => {
      const points = [
        { x: 10, y: 1 },
        { x: 20, y: 2 },
      ];
      expect(piecewiseLinear(points, 30)).toBe(3);
    });

    it("interpolates correctly with multiple segments", () => {
      const points = [
        { x: 0, y: 0 },
        { x: 10, y: 1 }, // slope 0.1
        { x: 20, y: 3 }, // slope 0.2
      ];
      expect(piecewiseLinear(points, 5)).toBe(0.5);
      expect(piecewiseLinear(points, 15)).toBe(2); // 1 + 0.2 * 5
    });

    it("handles unsorted input implicit via normalize", () => {
      const points = [
        { x: 20, y: 2 },
        { x: 10, y: 1 },
      ];
      expect(piecewiseLinear(points, 15)).toBe(1.5);
    });
  });
});
