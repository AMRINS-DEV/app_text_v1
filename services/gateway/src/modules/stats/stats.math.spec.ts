import { equityCurveFrom, maxDrawdownPct, sharpeApprox } from "./stats.math";
import type { ClosedTrade } from "./trade-history";

function tradesFrom(pnls: number[]): ClosedTrade[] {
  return pnls.map((pnl, i) => ({ pnl, closedAt: i }));
}

describe("equityCurveFrom", () => {
  it("starts at the given starting equity and accumulates pnl in order", () => {
    const curve = equityCurveFrom(tradesFrom([100, -50, 25]), 1_000);
    expect(curve.map((p) => p.equity)).toEqual([1_000, 1_100, 1_050, 1_075]);
  });

  it("returns just the starting point for an empty trade list", () => {
    const curve = equityCurveFrom([], 1_000);
    expect(curve).toHaveLength(1);
    expect(curve[0].equity).toBe(1_000);
  });
});

describe("maxDrawdownPct", () => {
  it("is zero for a monotonically increasing equity curve", () => {
    const curve = equityCurveFrom(tradesFrom([10, 10, 10]), 1_000);
    expect(maxDrawdownPct(curve)).toBe(0);
  });

  it("computes the worst peak-to-trough percentage, not just the last drop", () => {
    // 1000 -> 1200 (peak) -> 900 (a 25% drawdown from peak) -> 1100 (partial recovery, still a smaller drop)
    const curve = equityCurveFrom(tradesFrom([200, -300, 200]), 1_000);
    expect(maxDrawdownPct(curve)).toBeCloseTo(25, 6);
  });

  it("keeps tracking the worst drawdown even after a new peak is set later", () => {
    // 1000 -> 500 (50% dd) -> 2000 (new peak) -> 1900 (small dd from new peak)
    // Overall worst is still the earlier 50% drawdown.
    const curve = equityCurveFrom(tradesFrom([-500, 1500, -100]), 1_000);
    expect(maxDrawdownPct(curve)).toBeCloseTo(50, 6);
  });
});

describe("sharpeApprox", () => {
  it("is zero when there are fewer than two trades", () => {
    expect(sharpeApprox([])).toBe(0);
    expect(sharpeApprox(tradesFrom([10]))).toBe(0);
  });

  it("is zero when every trade has identical pnl (zero variance)", () => {
    expect(sharpeApprox(tradesFrom([50, 50, 50]))).toBe(0);
  });

  it("is positive for a consistently profitable history and negative for a losing one", () => {
    expect(sharpeApprox(tradesFrom([10, 20, 30, 5, 15]))).toBeGreaterThan(0);
    expect(sharpeApprox(tradesFrom([-10, -20, -30, -5, -15]))).toBeLessThan(0);
  });
});
