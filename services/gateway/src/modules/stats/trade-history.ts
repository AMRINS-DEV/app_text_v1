import { mulberry32, nextGaussian, seedFromString } from "../../common/prng";

export interface ClosedTrade {
  closedAt: number;
  pnl: number;
}

/**
 * §11.1's stats module reads real closed trades from `crates/journal`'s
 * storage once Phase 3+'s `crates/storage` is backed by a real database;
 * this sandbox has neither, so `/api/stats/overview` is computed over a
 * deterministic synthetic trade history instead — same "real computation,
 * synthetic input" split as Phase 3's ML training data. Seeded, so repeated
 * calls in the same process see the same history (it's generated once, at
 * `StatsService` construction, not regenerated per request).
 */
// A fixed reference instant, not `Date.now()` — the whole point of seeding
// this generator is that it produces the *same* history every time it's
// called, which `Date.now()` as a default would silently break (two
// `StatsService` instances built a millisecond apart would each get a
// shifted `closedAt` series).
const FIXED_REFERENCE_INSTANT = Date.UTC(2026, 0, 1);

export function generateSyntheticTradeHistory(
  count = 90,
  meanPnl = 35,
  stdDevPnl = 220,
  seed = seedFromString("tradeos-synthetic-history"),
  endTime = FIXED_REFERENCE_INSTANT,
  spanMs = 30 * 24 * 60 * 60 * 1000,
): ClosedTrade[] {
  const rng = mulberry32(seed);
  const trades: ClosedTrade[] = [];
  for (let i = 0; i < count; i++) {
    const closedAt = endTime - spanMs + Math.floor((i / count) * spanMs);
    const pnl = meanPnl + nextGaussian(rng) * stdDevPnl;
    trades.push({ closedAt, pnl });
  }
  return trades;
}
