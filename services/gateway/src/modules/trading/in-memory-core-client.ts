import { Injectable } from "@nestjs/common";

import type { AccountSnapshot, CoreClient, KillSwitchResult, PnlSnapshot, Position, TradingMode } from "./trading.types";

const SEED_POSITIONS: Position[] = [
  { id: "pos-1", symbol: "EURUSD", side: "buy", quantity: 0.5, entryPrice: 1.0845, sl: 1.081, tp: 1.091, openedAt: Date.now() - 3_600_000 },
  { id: "pos-2", symbol: "XAUUSD", side: "sell", quantity: 0.1, entryPrice: 2018.4, sl: 2026.0, tp: 2002.0, openedAt: Date.now() - 1_800_000 },
];

/**
 * §11.1's real deployment dials `tradeos-core` over gRPC; this sandbox has
 * no live core process to dial, so this implements the exact `CoreClient`
 * port a gRPC client would — same split as `SimBroker` in Phase 2.
 */
@Injectable()
export class InMemoryCoreClient implements CoreClient {
  private positions = new Map<string, Position>(SEED_POSITIONS.map((p) => [p.id, p]));
  private mode: TradingMode = "paper";
  private killSwitchEngaged = false;

  getAccount(): AccountSnapshot {
    return { mode: this.mode, killSwitchEngaged: this.killSwitchEngaged, positions: [...this.positions.values()] };
  }

  getPnl(): PnlSnapshot {
    const unrealizedPnl = [...this.positions.values()].reduce((sum, p) => {
      const direction = p.side === "buy" ? 1 : -1;
      // No live price feed wired to positions here; this is a placeholder
      // mark, not a real unrealized P&L computation (that needs the
      // realtime tick feed joined to each position's symbol — later work).
      return sum + direction * 0 * p.quantity;
    }, 0);
    return { equity: 100_000, unrealizedPnl, realizedPnlToday: 0, ts: Date.now() };
  }

  /** Immediate flatten+halt (§11.2), atomic within this single-process
   * store — no partial-failure state is representable here since there's
   * no real broker round-trip to fail partway through. */
  killSwitch(): KillSwitchResult {
    const started = process.hrtime.bigint();
    const flattenedCount = this.positions.size;
    this.positions.clear();
    this.mode = "halted";
    this.killSwitchEngaged = true;
    const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;
    return { flattenedCount, elapsedMs };
  }

  resetKillSwitch(): void {
    this.killSwitchEngaged = false;
    this.mode = "paper";
  }

  setMode(mode: TradingMode): void {
    if (this.killSwitchEngaged) throw new Error("cannot change mode while the kill switch is engaged");
    this.mode = mode;
  }

  closePosition(id: string, fraction?: number): Position | null {
    const position = this.positions.get(id);
    if (!position) return null;
    if (fraction === undefined || fraction >= 1) {
      this.positions.delete(id);
      return null;
    }
    if (fraction <= 0) throw new Error("fraction must be in (0, 1]");
    position.quantity *= 1 - fraction;
    return position;
  }
}
