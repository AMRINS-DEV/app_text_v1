import type { ClosedTrade } from "./trade-history";

export interface EquityPoint {
  t: number;
  equity: number;
}

export function equityCurveFrom(trades: readonly ClosedTrade[], startingEquity: number): EquityPoint[] {
  const curve: EquityPoint[] = [{ t: trades[0]?.closedAt ?? Date.now(), equity: startingEquity }];
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
