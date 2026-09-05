import type { ClosedTrade } from "./trade-history";

export interface EquityPoint {
  t: number;
  equity: number;
}

export function equityCurveFrom(trades: readonly ClosedTrade[], startingEquity: number): EquityPoint[] {
  // The synthetic starting point must be strictly *before* the first
  // trade, not at the same instant — a real bug found while manually
  // verifying the Phase 6 dashboard pages: chart-engine's `ChartHost`
  // asserts strictly-ascending, duplicate-free timestamps, and reusing
  // `trades[0].closedAt` here made points[0] and points[1] identical,
  // crashing every render of the overview page's equity curve.
  const firstTradeTime = trades[0]?.closedAt ?? Date.now();
  const curve: EquityPoint[] = [{ t: firstTradeTime - 1, equity: startingEquity }];
  let equity = startingEquity;
  for (const trade of trades) {
    equity += trade.pnl;
    curve.push({ t: trade.closedAt, equity });
  }
  return curve;
}

export function maxDrawdownPct(curve: readonly EquityPoint[]): number {
  let peak = curve[0]?.equity ?? 0;
  let worst = 0;
  for (const point of curve) {
    peak = Math.max(peak, point.equity);
    if (peak > 0) worst = Math.max(worst, (peak - point.equity) / peak);
  }
  return worst * 100;
}

export function sharpeApprox(trades: readonly ClosedTrade[]): number {
  if (trades.length < 2) return 0;
  const mean = trades.reduce((sum, t) => sum + t.pnl, 0) / trades.length;
  const variance = trades.reduce((sum, t) => sum + (t.pnl - mean) ** 2, 0) / (trades.length - 1);
  const stdDev = Math.sqrt(variance);
  if (stdDev === 0) return 0;
  // Trade-level Sharpe scaled as if trades were daily returns (§14's
  // "golden metrics" call for a real per-strategy Sharpe once there's a
  // real daily-return series to compute one from) — an approximation,
  // named as such, not the real risk-adjusted return.
  return (mean / stdDev) * Math.sqrt(252);
}
