import { Injectable } from "@nestjs/common";

import { equityCurveFrom, EquityPoint, maxDrawdownPct, sharpeApprox } from "./stats.math";
import { ClosedTrade, generateSyntheticTradeHistory } from "./trade-history";

export interface StatsOverview {
  startingEquity: number;
  equityCurve: EquityPoint[];
  totalTrades: number;
  winRate: number;
  expectancy: number;
  maxDrawdownPct: number;
  sharpeApprox: number;
}

const STARTING_EQUITY = 100_000;

/** §11.1: "P&L, equity curve, stats" — see `trade-history.ts`'s doc
 * comment for why the input is synthetic. */
@Injectable()
export class StatsService {
  private readonly trades: ClosedTrade[] = generateSyntheticTradeHistory();

  overview(): StatsOverview {
    const curve = equityCurveFrom(this.trades, STARTING_EQUITY);
    const wins = this.trades.filter((t) => t.pnl > 0).length;
    return {
      startingEquity: STARTING_EQUITY,
      equityCurve: curve,
      totalTrades: this.trades.length,
      winRate: this.trades.length === 0 ? 0 : wins / this.trades.length,
      expectancy: this.trades.length === 0 ? 0 : this.trades.reduce((s, t) => s + t.pnl, 0) / this.trades.length,
      maxDrawdownPct: maxDrawdownPct(curve),
      sharpeApprox: sharpeApprox(this.trades),
    };
  }
}
