import { describe, expect, it } from "vitest";
import { radarAngle, radarLabelAnchor, radarPoint, radarPolygon } from "./radar";

const C = 100;
const R = 50;

describe("radarAngle", () => {
  it("starts at the top and divides the circle evenly", () => {
    expect(radarAngle(0, 5)).toBeCloseTo(-Math.PI / 2);
    expect(radarAngle(1, 5) - radarAngle(0, 5)).toBeCloseTo((2 * Math.PI) / 5);
  });
});

describe("radarPoint", () => {
  it("puts the first axis straight up", () => {
    const p = radarPoint(C, C, R, 0, 5, 100);
    expect(p.x).toBeCloseTo(C);
    expect(p.y).toBeCloseTo(C - R);
  });

  it("runs the remaining axes clockwise", () => {
    const right = radarPoint(C, C, R, 1, 4, 100);
    expect(right.x).toBeCloseTo(C + R);
    expect(right.y).toBeCloseTo(C);
  });

  it("scales the radius with the score", () => {
    const half = radarPoint(C, C, R, 0, 5, 50);
    expect(half.y).toBeCloseTo(C - R / 2);
  });

  it("sends every value of 0 to the centre", () => {
    for (let i = 0; i < 5; i++) {
      const p = radarPoint(C, C, R, i, 5, 0);
      expect(p.x).toBeCloseTo(C);
      expect(p.y).toBeCloseTo(C);
    }
  });

  it("clamps out-of-range scores instead of bursting the chart", () => {
    expect(radarPoint(C, C, R, 0, 5, 150).y).toBeCloseTo(C - R);
    expect(radarPoint(C, C, R, 0, 5, -20).y).toBeCloseTo(C);
  });
});

describe("radarPolygon", () => {
  it("draws a diamond for four full-score axes", () => {
    expect(radarPolygon(C, C, R, [100, 100, 100, 100])).toBe(
      "100.0,50.0 150.0,100.0 100.0,150.0 50.0,100.0"
    );
  });

  it("emits exactly one vertex per value", () => {
    expect(radarPolygon(C, C, R, [10, 20, 30]).split(" ")).toHaveLength(3);
  });
});

describe("radarLabelAnchor", () => {
  it("anchors the five assessment labels away from the shape", () => {
    const anchors = [0, 1, 2, 3, 4].map((i) => radarLabelAnchor(i, 5));
    expect(anchors).toEqual(["middle", "start", "start", "end", "end"]);
  });
});