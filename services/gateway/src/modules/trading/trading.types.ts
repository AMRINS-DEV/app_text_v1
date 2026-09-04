export type TradingMode = "live" | "paper" | "halted";

export interface Position {
  id: string;
  symbol: string;
  side: "buy" | "sell";
  quantity: number;
  entryPrice: number;
  sl: number;
  tp: number;
  openedAt: number;
}

export interface AccountSnapshot {
  mode: TradingMode;
  killSwitchEngaged: boolean;
  positions: Position[];
}

export interface PnlSnapshot {
  equity: number;
  unrealizedPnl: number;
  realizedPnlToday: number;
  ts: number;
}

export interface KillSwitchResult {
  flattenedCount: number;
  elapsedMs: number;
}

/**
 * The exact surface §11.1's "gRPC → core: positions, orders, mode switch,
 * kill switch" implies. This sandbox has no live `tradeos-core` process to
 * dial over gRPC, so `InMemoryCoreClient` implements this same port
 * in-process — swapping in a real gRPC client later changes only the
 * implementation registered in `TradingModule`, not any caller.
 */
export interface CoreClient {
  getAccount(): AccountSnapshot;
  getPnl(): PnlSnapshot;
  killSwitch(): KillSwitchResult;
  resetKillSwitch(): void;
  setMode(mode: TradingMode): void;
  closePosition(id: string, fraction?: number): Position | null;
}
