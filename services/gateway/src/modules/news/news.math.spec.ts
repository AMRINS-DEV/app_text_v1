import { FixedHorizonMove } from "../../common/outcome-resolution";
import { NewsEventRecord } from "./news-history";
import { newsImpactStability } from "./news.math";

function record(
  overrides: Partial<Omit<NewsEventRecord, "move">> & { move?: Partial<FixedHorizonMove> },
): NewsEventRecord {
  const { move: moveOverrides, ...rest } = overrides;
  return {
    id: "ev-x",
    ts: Date.UTC(2023, 10, 14), // 2023-11-14 (Q4)
    headline: "x",
    eventType: "rate_decision",
    symbol: "EURUSD",
    impactTier: "high",
    sentiment: 0,
    expectedDirection: "Long",
    horizonMin: 15,
    ...rest,
    move: {
      movePips: 50,
      moveAtr: 2.5,
      direction: "Long",
      directionHit: true,
      ...moveOverrides,
    },
  };
}

describe("newsImpactStability", () => {
  it("buckets by quarter and filters by event type, symbol, and horizon", () => {
    const history: NewsEventRecord[] = [
      record({}),
      record({ horizonMin: 60 }), // wrong horizon -> excluded
      record({ eventType: "cpi" }), // wrong event type -> excluded
      record({ symbol: "GBPUSD" }), // wrong symbol -> excluded
    ];

    const periods = newsImpactStability(history, "rate_decision", "EURUSD", 15);

    expect(periods).toHaveLength(1);
    expect(periods[0].n).toBe(1);
    expect(periods[0].quarter).toBe("2023Q4");
    expect(periods[0].directionHitRate).toBe(1.0);
  });

  it("computes avg_impact from the absolute ATR-normalized move", () => {
    const history: NewsEventRecord[] = [
      record({ move: { moveAtr: -2.0 } }),
      record({ move: { moveAtr: 4.0 } }),
    ];
    const periods = newsImpactStability(history, "rate_decision", "EURUSD", 15);
    expect(periods[0].avgImpact).toBeCloseTo(3.0, 6); // (2.0 + 4.0) / 2
  });

  it("returns an empty list with no matches", () => {
    expect(newsImpactStability([], "rate_decision", "EURUSD", 15)).toEqual([]);
  });
});
