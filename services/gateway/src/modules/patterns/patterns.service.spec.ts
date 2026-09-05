import { PatternsService } from "./patterns.service";

describe("PatternsService", () => {
  it("is deterministic across instances (same seed every time)", () => {
    const a = new PatternsService().list();
    const b = new PatternsService().list();
    expect(a).toEqual(b);
  });

  it("filters by symbol and regime", () => {
    const service = new PatternsService();
    const all = service.list();
    const filtered = service.list("EURUSD", "Trending");
    expect(filtered.length).toBeLessThanOrEqual(all.length);
    for (const record of filtered) {
      expect(record.symbol).toBe("EURUSD");
      expect(record.regime).toBe("Trending");
    }
  });

  it("every instance carries a real verdict, not an unresolved placeholder", () => {
    const service = new PatternsService();
    for (const record of service.list()) {
      expect(["confirmed", "failed", "timeout"]).toContain(record.resolution.verdict);
    }
  });

  it("double_top has a higher historical hit rate than double_bottom (seeded)", () => {
    const service = new PatternsService();
    let topHits = 0;
    let topTotal = 0;
    let bottomHits = 0;
    let bottomTotal = 0;
    for (const record of service.list()) {
      if (record.kind === "double_top") {
        topTotal++;
        if (record.resolution.verdict === "confirmed") topHits++;
      } else {
        bottomTotal++;
        if (record.resolution.verdict === "confirmed") bottomHits++;
      }
    }
    expect(topHits / topTotal).toBeGreaterThan(bottomHits / bottomTotal);
  });
});
