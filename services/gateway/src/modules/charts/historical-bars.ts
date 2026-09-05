import { mulberry32, seedFromString } from "../../common/prng";
import { MarketFeedService } from "../realtime/market-feed.service";
import type { BarMessage } from "../realtime/market-feed.service";

const TIMEFRAME_MS: Record<string, number> = {
  "5s": 5_000,
  "1m": 60_000,
  "5m": 5 * 60_000,
  "1h": 60 * 60_000,
};

/**
 * §11.2's `GET /api/charts/bars` reads historical bars from QuestDB in the
 * real deployment; this sandbox has no market data archive, so bars for an
 * arbitrary `[from, to)` window are generated deterministically instead —
 * keyed on `(symbol, tf, openTime)`, so the same bar always resolves to the
 * same OHLC no matter when it's requested (the property a real historical
 * store gives you for free, and the one thing a naive "just call `Math.random`
 * per request" stand-in would not have). These bars are independent of
 * `MarketFeedService`'s live random walk — there is no real historical
 * archive backing the live feed here, so the two series are not
 * continuous with each other; documented, not hidden.
 */
export function generateHistoricalBars(symbol: string, timeframe: string, fromMs: number, toMs: number): BarMessage[] {
  const periodMs = TIMEFRAME_MS[timeframe];
  if (!periodMs) throw new Error(`unknown timeframe: ${timeframe}`);
  if (!MarketFeedService.SYMBOLS.includes(symbol)) throw new Error(`unknown symbol: ${symbol}`);

  const bars: BarMessage[] = [];
  const firstOpen = Math.floor(fromMs / periodMs) * periodMs;
  for (let openTime = firstOpen; openTime < toMs; openTime += periodMs) {
    const rng = mulberry32(seedFromString(`${symbol}:${timeframe}:${openTime}`));
    const base = 1 + rng() * 2; // arbitrary but bar-stable base level
    const open = base;
    const close = base + (rng() - 0.5) * 0.02 * base;
    const high = Math.max(open, close) + rng() * 0.01 * base;
    const low = Math.min(open, close) - rng() * 0.01 * base;
    bars.push({
      symbol,
      tf: timeframe,
      openTime,
      closeTime: openTime + periodMs,
      open,
      high,
      low,
      close,
      volume: Math.floor(rng() * 1000),
    });
  }
  return bars;
}
