import { OutcomeResolution } from "../../common/outcome-resolution";
import { PatternInstanceRecord } from "./pattern-history";
import { conditionalReliability } from "./patterns.math";

function record(
  overrides: Partial<Omit<PatternInstanceRecord, "resolution">> & { resolution?: Partial<OutcomeResolution> },
): PatternInstanceRecord {
  const { resolution: resolutionOverrides, ...rest } = overrides;
  return {
    id: "pi-x",
    kind: "double_top",
    symbol: "EURUSD",
    regime: "Trending",
    tsStart: 1_000,
    confidence: 0.7,
    entryPrice: 100,
    targetPrice: 104,
    invalidationPrice: 98,
    ...rest,
    resolution: {
      verdict: "confirmed",
      barsToResolution: 1,
      mfe: 1,
      mae: 1,
      rMultiple: 2.0,
      movePips: 40,
      moveAtr: 2.0,
      direction: "Long",
      ...resolutionOverrides,
    },
  };
}

describe("conditionalReliability", () => {
  it("aggregates only instances matching kind, symbol, regime, and since_ts", () => {
    const history: PatternInstanceRecord[] = [
      record({ resolution: { verdict: "confirmed", rMultiple: 2.0 } }),
      record({ resolution: { verdict: "failed", rMultiple: -1.0 } }),
      record({ symbol: "GBPUSD", resolution: { verdict: "confirmed", rMultiple: 3.0 } }),
      record({ regime: "Ranging", resolution: { verdict: "confirmed", rMultiple: 3.0 } }),
      record({ tsStart: 500, resolution: { verdict: "confirmed", rMultiple: 3.0 } }),
    ];

    const result = conditionalReliability(history, "double_top", "EURUSD", "Trending", 900);

    expect(result.n).toBe(2);
    expect(result.hitRate).toBe(0.5);
    expect(result.avgR).toBeCloseTo(0.5, 6);
    expect(result.medianR).toBeCloseTo(0.5, 6);
  });

  it("counts a timeout as not confirmed rather than excluding it", () => {
    const history: PatternInstanceRecord[] = [
      record({ resolution: { verdict: "confirmed", rMultiple: 2.0 } }),
      record({ resolution: { verdict: "timeout", rMultiple: 0.0 } }),
    ];

    const result = conditionalReliability(history, "double_top", "EURUSD", "Trending");

    expect(result.n).toBe(2);
    expect(result.hitRate).toBe(0.5);
  });

  it("reports n=0 with no matches", () => {
    const result = conditionalReliability([], "double_top", "EURUSD", "Trending");
    expect(result.n).toBe(0);
    expect(result.hitRate).toBe(0);
  });
});
