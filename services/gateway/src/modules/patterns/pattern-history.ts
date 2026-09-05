import { mulberry32, seedFromString } from "../../common/prng";
import { OutcomeResolution, resolvePatternOutcome } from "../../common/outcome-resolution";

export type PatternKind = "double_top" | "double_bottom";
export type RegimeLabel = "Trending" | "Ranging" | "Expansion" | "HighVolChoppy";

export interface PatternInstanceRecord {
  id: string;
  kind: PatternKind;
  symbol: string;
  regime: RegimeLabel;
  tsStart: number;
  confidence: number;
  entryPrice: number;
  targetPrice: number;
  invalidationPrice: number;
  resolution: OutcomeResolution;
}

const SYMBOLS = ["EURUSD", "GBPUSD", "USDJPY"];
const REGIMES: RegimeLabel[] = ["Trending", "Ranging", "Expansion", "HighVolChoppy"];
const KINDS: PatternKind[] = ["double_top", "double_bottom"];

// Same fixed-reference-instant discipline as trade-history.ts /
// historical-bars.ts — never `Date.now()` as a default.
const FIXED_REFERENCE_INSTANT = Date.UTC(2026, 0, 1);

/**
 * §12.3's pattern history, resolved via `resolvePatternOutcome` against a
 * deliberately-constructed (but genuinely walked, not hardcoded) synthetic
 * subsequent price path — the verdict on each record is computed by the
 * same real barrier-walk every live detection would go through, only the
 * *input* path is synthetic. `double_top` is seeded with a higher hit
 * rate than `double_bottom` (60% vs 45%) so `conditionalReliability` has a
 * real, discoverable signal instead of uniform noise — the same choice
 * `agents_graph.backfill` makes on the Python side.
 */
export function generateSyntheticPatternHistory(
  count = 300,
  seed = seedFromString("tradeos-pattern-history"),
  endTime = FIXED_REFERENCE_INSTANT,
  spanMs = 300 * 24 * 60 * 60 * 1000,
): PatternInstanceRecord[] {
  const rng = mulberry32(seed);
  const records: PatternInstanceRecord[] = [];

  for (let i = 0; i < count; i++) {
    const tsStart = endTime - spanMs + Math.floor((i / count) * spanMs);
    const symbol = SYMBOLS[Math.floor(rng() * SYMBOLS.length)];
    const regime = REGIMES[Math.floor(rng() * REGIMES.length)];
    const kind = KINDS[Math.floor(rng() * KINDS.length)];
    const isLong = kind === "double_bottom"; // double_top -> Short, double_bottom -> Long (agents_pattern's mapping)

    const entryPrice = 1.1 + rng() * 0.05;
    const distance = 0.002 + rng() * 0.006;
    const targetPrice = isLong ? entryPrice + distance : entryPrice - distance;
    const invalidationPrice = isLong ? entryPrice - distance * 0.6 : entryPrice + distance * 0.6;

    const hitChance = kind === "double_top" ? 0.6 : 0.45;
    const confirmed = rng() < hitChance;
    const barCount = 1 + Math.floor(rng() * 20);
    const highs: number[] = [];
    const lows: number[] = [];
    for (let bar = 0; bar < barCount; bar++) {
      const isLastBar = bar === barCount - 1;
      const reachedPrice = isLastBar
        ? confirmed
          ? targetPrice
          : invalidationPrice
        : entryPrice + (rng() - 0.5) * distance * 0.3;
      highs.push(Math.max(entryPrice, reachedPrice) + rng() * distance * 0.05);
      lows.push(Math.min(entryPrice, reachedPrice) - rng() * distance * 0.05);
    }

    const resolution = resolvePatternOutcome(
      isLong ? "Long" : "Short",
      entryPrice,
      targetPrice,
      invalidationPrice,
      highs,
      lows,
      0.0001,
      distance / 2,
    );

    records.push({
      id: `pi-${i}`,
      kind,
      symbol,
      regime,
      tsStart,
      confidence: 0.5 + rng() * 0.45,
      entryPrice,
      targetPrice,
      invalidationPrice,
      resolution,
    });
  }

  return records;
}
