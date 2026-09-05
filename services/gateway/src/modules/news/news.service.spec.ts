import { NewsService } from "./news.service";

describe("NewsService", () => {
  it("is deterministic across instances (same seed every time)", () => {
    const a = new NewsService().timeline();
    const b = new NewsService().timeline();
    expect(a).toEqual(b);
  });

  it("filters the timeline by symbol", () => {
    const service = new NewsService();
    const filtered = service.timeline("EURUSD");
    for (const record of filtered) {
      expect(record.symbol).toBe("EURUSD");
    }
  });

  it("impactStability returns periods with rates in [0, 1]", () => {
    const service = new NewsService();
    const timeline = service.timeline();
    const first = timeline[0];
    const periods = service.impactStability(first.eventType, first.symbol, first.horizonMin);
    expect(periods.length).toBeGreaterThan(0);
    for (const period of periods) {
      expect(period.directionHitRate).toBeGreaterThanOrEqual(0);
      expect(period.directionHitRate).toBeLessThanOrEqual(1);
      expect(period.avgImpact).toBeGreaterThanOrEqual(0);
    }
  });
});
