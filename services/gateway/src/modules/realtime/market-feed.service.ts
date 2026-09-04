import { Injectable, OnModuleDestroy, OnModuleInit } from "@nestjs/common";

import { mulberry32, seedFromString } from "../../common/prng";
import { TopicBus } from "./topic-bus";

export interface TickMessage {
  symbol: string;
  bid: number;
  ask: number;
  seq: number;
  ts: number;
}

export interface BarMessage {
  symbol: string;
  tf: string;
  openTime: number;
  closeTime: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

interface SymbolConfig {
  symbol: string;
  basePrice: number;
  volatility: number;
  spread: number;
}

const SYMBOLS: SymbolConfig[] = [
  { symbol: "EURUSD", basePrice: 1.085, volatility: 0.00006, spread: 0.00008 },
  { symbol: "GBPUSD", basePrice: 1.265, volatility: 0.00008, spread: 0.0001 },
  { symbol: "USDJPY", basePrice: 149.5, volatility: 0.008, spread: 0.012 },
  { symbol: "XAUUSD", basePrice: 2020, volatility: 0.35, spread: 0.4 },
];

interface SymbolState {
  config: SymbolConfig;
  price: number;
  rng: () => number;
  seq: number;
  bar: (BarMessage & { openTime: number }) | null;
}

/**
 * §11.3's realtime feed has no live MT5/core connection to draw from in
 * this sandbox — the same "real transport, synthetic source" split already
 * used by `mock-mt5-bridge` in Phase 1. Ticks are a bounded random walk per
 * symbol at 20ms (≈200 ticks/s aggregate across 4 symbols, matching §12's
 * own "200 tick/s aggregate feed" acceptance figure); 5-second bars are
 * aggregated from the same tick stream and published on bar close.
 */
@Injectable()
export class MarketFeedService implements OnModuleInit, OnModuleDestroy {
  static readonly BAR_TIMEFRAME = "5s";
  static readonly BAR_PERIOD_MS = 5_000;
  static readonly TICK_INTERVAL_MS = 20;
  static readonly SYMBOLS = SYMBOLS.map((s) => s.symbol);

  private readonly state = new Map<string, SymbolState>();
  private readonly timers: NodeJS.Timeout[] = [];

  constructor(private readonly bus: TopicBus) {}

  onModuleInit(): void {
    for (const config of SYMBOLS) {
      this.state.set(config.symbol, {
        config,
        price: config.basePrice,
        rng: mulberry32(seedFromString(config.symbol)),
        seq: 0,
        bar: null,
      });
      this.timers.push(setInterval(() => this.emitTick(config.symbol), MarketFeedService.TICK_INTERVAL_MS));
    }
  }

  onModuleDestroy(): void {
    for (const timer of this.timers) clearInterval(timer);
  }

  private emitTick(symbol: string): void {
    const st = this.state.get(symbol);
    if (!st) return;

    const drift = (st.rng() - 0.5) * st.config.volatility;
    st.price = Math.max(st.config.volatility, st.price + drift);
    const ts = Date.now();
    const tick: TickMessage = {
      symbol,
      bid: st.price - st.config.spread / 2,
      ask: st.price + st.config.spread / 2,
      seq: st.seq++,
      ts,
    };
    this.bus.publish(`ticks:${symbol}`, tick);
    this.accumulateBar(st, tick);
  }

  private accumulateBar(st: SymbolState, tick: TickMessage): void {
    const openTime = Math.floor(tick.ts / MarketFeedService.BAR_PERIOD_MS) * MarketFeedService.BAR_PERIOD_MS;
    if (!st.bar || st.bar.openTime !== openTime) {
      if (st.bar) {
        this.bus.publish(`bars:${st.config.symbol}:${MarketFeedService.BAR_TIMEFRAME}`, st.bar);
      }
      st.bar = {
        symbol: st.config.symbol,
        tf: MarketFeedService.BAR_TIMEFRAME,
        openTime,
        closeTime: openTime + MarketFeedService.BAR_PERIOD_MS,
        open: tick.bid,
        high: tick.bid,
        low: tick.bid,
        close: tick.bid,
        volume: 0,
      };
    }
    st.bar.high = Math.max(st.bar.high, tick.bid);
    st.bar.low = Math.min(st.bar.low, tick.bid);
    st.bar.close = tick.bid;
    st.bar.volume += 1;
  }
}
