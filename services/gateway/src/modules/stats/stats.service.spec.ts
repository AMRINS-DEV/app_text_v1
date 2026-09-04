import { StatsService } from "./stats.service";

describe("StatsService", () => {
  it("produces an equity curve that starts at the starting equity", () => {
    const overview = new StatsService().overview();
    expect(overview.equityCurve[0].equity).toBe(overview.startingEquity);
  });

  it("produces an equity curve whose last point equals starting equity plus total pnl", () => {
    const stats = new StatsService();
    const overview = stats.overview();
    const totalPnl = overview.expectancy * overview.totalTrades;
    const lastPoint = overview.equityCurve[overview.equityCurve.length - 1];
    expect(lastPoint.equity).toBeCloseTo(overview.startingEquity + totalPnl, 6);
  });

  it("winRate is between 0 and 1", () => {
    const overview = new StatsService().overview();
    expect(overview.winRate).toBeGreaterThanOrEqual(0);
    expect(overview.winRate).toBeLessThanOrEqual(1);
  });

  it("maxDrawdownPct is non-negative and at most 100", () => {
    const overview = new StatsService().overview();
    expect(overview.maxDrawdownPct).toBeGreaterThanOrEqual(0);
    expect(overview.maxDrawdownPct).toBeLessThanOrEqual(100);
  });

  it("is deterministic across instances (same seed every time)", () => {
    const a = new StatsService().overview();
    const b = new StatsService().overview();
    expect(a).toEqual(b);
  });
});
