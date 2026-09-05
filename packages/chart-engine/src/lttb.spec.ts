import { describe, expect, it } from "vitest";

import { lttbDownsample } from "./lttb";

interface Point {
  t: number;
  v: number;
}

function series(n: number): Point[] {
  return Array.from({ length: n }, (_, i) => ({ t: i, v: Math.sin(i / 7) * 10 + i * 0.3 }));
}

describe("lttbDownsample", () => {
  it("returns the input unchanged when already at or below the threshold", () => {
    const data = series(50);
    expect(lttbDownsample(data, 100, (p) => p.t, (p) => p.v)).toEqual(data);
    expect(lttbDownsample(data, 50, (p) => p.t, (p) => p.v)).toEqual(data);
  });

  it("returns the input unchanged for two or fewer points regardless of threshold", () => {
    const data = series(2);
    expect(lttbDownsample(data, 1, (p) => p.t, (p) => p.v)).toEqual(data);
  });

  it("always keeps the first and last point", () => {
    const data = series(1000);
    const result = lttbDownsample(data, 50, (p) => p.t, (p) => p.v);
    expect(result[0]).toEqual(data[0]);
    expect(result[result.length - 1]).toEqual(data[data.length - 1]);
  });

  it("produces exactly `threshold` points for a large series", () => {
    const data = series(2000);
    const result = lttbDownsample(data, 100, (p) => p.t, (p) => p.v);
    expect(result).toHaveLength(100);
  });

  it("returns points that are a subset of the original series, in order", () => {
    const data = series(500);
    const result = lttbDownsample(data, 80, (p) => p.t, (p) => p.v);
    let cursor = -1;
    for (const point of result) {
      const index = data.indexOf(point);
      expect(index).toBeGreaterThan(cursor);
      cursor = index;
    }
  });

  it("never returns more points than were given, even at a threshold larger than the series", () => {
    const data = series(10);
    const result = lttbDownsample(data, 1000, (p) => p.t, (p) => p.v);
    expect(result.length).toBeLessThanOrEqual(data.length);
  });
});
