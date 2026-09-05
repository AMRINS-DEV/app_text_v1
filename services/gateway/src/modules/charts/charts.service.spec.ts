import { ChartsService } from "./charts.service";
import { generateHistoricalBars } from "./historical-bars";

describe("generateHistoricalBars", () => {
  it("is deterministic: the same window queried twice returns identical bars", () => {
    const a = generateHistoricalBars("EURUSD", "1m", 0, 10 * 60_000);
    const b = generateHistoricalBars("EURUSD", "1m", 0, 10 * 60_000);
    expect(a).toEqual(b);
  });

  it("produces bars with high >= max(open, close) and low <= min(open, close)", () => {
    const bars = generateHistoricalBars("XAUUSD", "5m", 0, 60 * 60_000);
    expect(bars.length).toBeGreaterThan(0);
    for (const bar of bars) {
      expect(bar.high).toBeGreaterThanOrEqual(Math.max(bar.open, bar.close));
      expect(bar.low).toBeLessThanOrEqual(Math.min(bar.open, bar.close));
    }
  });

  it("rejects an unknown symbol", () => {
    expect(() => generateHistoricalBars("NOTASYMBOL", "1m", 0, 60_000)).toThrow();
  });

  it("rejects an unknown timeframe", () => {
    expect(() => generateHistoricalBars("EURUSD", "3m", 0, 60_000)).toThrow();
  });
});

describe("ChartsService", () => {
  it("returns every bar in range when no max_points is given", () => {
    const service = new ChartsService();
    const bars = service.bars({ sym: "EURUSD", tf: "1m", from: 0, to: 100 * 60_000 });
    expect(bars).toHaveLength(100);
  });

  it("downsamples to at most max_points when the range has more bars than that", () => {
    const service = new ChartsService();
    const bars = service.bars({ sym: "EURUSD", tf: "1m", from: 0, to: 1_000 * 60_000, max_points: 50 });
    expect(bars.length).toBeLessThanOrEqual(50);
    expect(bars.length).toBeGreaterThan(1);
  });
});
