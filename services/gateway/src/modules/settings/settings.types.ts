export interface RiskProfile {
  riskPerTradePct: number;
  fractionalKellyCap: number;
  maxDailyDrawdownPct: number;
  maxTotalDrawdownPct: number;
}

export interface Settings {
  riskProfile: RiskProfile;
  allowedPairs: string[];
  defaultMode: "live" | "paper";
  /** Agent config / model routing (§11.1) is Phase 5 scope — no agents
   * exist yet to route to. Kept as an explicit empty map rather than
   * omitted, so the settings page has a stable place to grow into. */
  modelRouting: Record<string, string>;
}

export const DEFAULT_SETTINGS: Settings = {
  riskProfile: {
    riskPerTradePct: 0.5,
    fractionalKellyCap: 0.25,
    maxDailyDrawdownPct: 3,
    maxTotalDrawdownPct: 10,
  },
  allowedPairs: ["EURUSD", "GBPUSD", "USDJPY", "XAUUSD"],
  defaultMode: "paper",
  modelRouting: {},
};
