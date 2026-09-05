import { mulberry32, seedFromString } from "../../common/prng";
import { FixedHorizonMove, resolveFixedHorizonMove } from "../../common/outcome-resolution";

export interface NewsEventRecord {
  id: string;
  ts: number;
  headline: string;
  eventType: string;
  symbol: string;
  impactTier: "low" | "medium" | "high";
  sentiment: number;
  expectedDirection: "Long" | "Short";
  horizonMin: number;
  move: FixedHorizonMove;
}

const SYMBOLS = ["EURUSD", "GBPUSD", "USDJPY"];
const EVENT_TYPES = ["rate_decision", "nfp", "cpi"];
const IMPACT_TIERS: readonly NewsEventRecord["impactTier"][] = ["low", "medium", "high"];
const HORIZON_MIN = 15;

// Same fixed-reference-instant discipline as trade-history.ts.
const FIXED_REFERENCE_INSTANT = Date.UTC(2026, 0, 1);

/** §12.4's news timeline, resolved via `resolveFixedHorizonMove` against a
 * synthetic before/after price pair — real move computation, synthetic
 * price input (same split as `pattern-history.ts`). */
export function generateSyntheticNewsHistory(
  count = 150,
  seed = seedFromString("tradeos-news-history"),
  endTime = FIXED_REFERENCE_INSTANT,
  spanMs = 300 * 24 * 60 * 60 * 1000,
): NewsEventRecord[] {
  const rng = mulberry32(seed);
  const priceAtEvent = 1.1;
  const records: NewsEventRecord[] = [];

  for (let i = 0; i < count; i++) {
    const ts = endTime - spanMs + Math.floor((i / count) * spanMs);
    const symbol = SYMBOLS[Math.floor(rng() * SYMBOLS.length)];
    const eventType = EVENT_TYPES[Math.floor(rng() * EVENT_TYPES.length)];
    const expectedDirection: "Long" | "Short" = rng() < 0.5 ? "Long" : "Short";

    const matchesExpectation = rng() < 0.55;
    let sign = expectedDirection === "Long" ? 1 : -1;
    if (!matchesExpectation) sign = -sign;
    const priceAtHorizon = priceAtEvent + sign * (0.0005 + rng() * 0.0075);
    const move = resolveFixedHorizonMove(priceAtEvent, priceAtHorizon, expectedDirection, 0.0001, 0.002);

    records.push({
      id: `ev-${i}`,
      ts,
      headline: `${eventType} #${i}`,
      eventType,
      symbol,
      impactTier: IMPACT_TIERS[Math.floor(rng() * IMPACT_TIERS.length)],
      sentiment: rng() * 2 - 1,
      expectedDirection,
      horizonMin: HORIZON_MIN,
      move,
    });
  }

  return records;
}
