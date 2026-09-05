"use client";

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { useAuthStore } from "../../../stores/auth";

type Verdict = "confirmed" | "failed" | "timeout";
type PatternKind = "double_top" | "double_bottom";
type RegimeLabel = "Trending" | "Ranging" | "Expansion" | "HighVolChoppy";

interface OutcomeResolution {
  verdict: Verdict;
  barsToResolution: number | null;
  mfe: number;
  mae: number;
  rMultiple: number;
  movePips: number;
  moveAtr: number;
  direction: "Long" | "Short";
}

interface PatternInstanceRecord {
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

interface ConditionalReliability {
  n: number;
  hitRate: number;
  avgR: number;
  medianR: number;
}

const SYMBOLS = ["EURUSD", "GBPUSD", "USDJPY"];
const REGIMES: RegimeLabel[] = ["Trending", "Ranging", "Expansion", "HighVolChoppy"];
const KINDS: PatternKind[] = ["double_top", "double_bottom"];

const VERDICT_LABEL: Record<Verdict, string> = {
  confirmed: "✅ CONFIRMED",
  failed: "❌ FAILED",
  timeout: "⏳ TIMEOUT",
};

/** §12.3's patterns page: cards with detection confidence, verification
 * status, and the historical prior from the graph (§7.2's conditional-
 * reliability query, via `GET /api/patterns/prior`). The live "detect" job
 * trigger and chart-overlay rendering from §12.3's full spec need a live
 * agent bridge and `packages/chart-engine`'s `PatternOverlay` primitive —
 * out of reach in this sandbox, see README's Phase 6 section. */
export default function PatternsPage() {
  const authorizedFetch = useAuthStore((state) => state.authorizedFetch);
  const [symbol, setSymbol] = useState<string>(SYMBOLS[0]);
  const [regime, setRegime] = useState<RegimeLabel>(REGIMES[0]);

  const { data: patterns } = useQuery({
    queryKey: ["patterns", symbol, regime],
    queryFn: () => authorizedFetch<PatternInstanceRecord[]>(`/api/patterns?symbol=${symbol}&regime=${regime}`),
  });

  const { data: priors } = useQuery({
    queryKey: ["patterns-prior", symbol, regime],
    queryFn: async () => {
      const entries = await Promise.all(
        KINDS.map(
          async (kind) =>
            [
              kind,
              await authorizedFetch<ConditionalReliability>(
                `/api/patterns/prior?kind=${kind}&symbol=${symbol}&regime=${regime}`,
              ),
            ] as const,
        ),
      );
      return Object.fromEntries(entries) as Record<PatternKind, ConditionalReliability>;
    },
  });

  return (
    <main className="p-8">
      <h1 className="text-2xl font-semibold">Patterns</h1>

      <div className="mt-4 flex gap-2 text-sm">
        <select
          value={symbol}
          onChange={(e) => setSymbol(e.target.value)}
          className="rounded border border-neutral-700 bg-transparent px-2 py-1"
        >
          {SYMBOLS.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        <select
          value={regime}
          onChange={(e) => setRegime(e.target.value as RegimeLabel)}
          className="rounded border border-neutral-700 bg-transparent px-2 py-1"
        >
          {REGIMES.map((r) => (
            <option key={r} value={r}>
              {r}
            </option>
          ))}
        </select>
      </div>

      {priors && (
        <div className="mt-4 flex flex-wrap gap-4 text-xs text-neutral-400">
          {KINDS.map((kind) => {
            const prior = priors[kind];
            return (
              <span key={kind}>
                {kind}, {symbol}, {regime}: n={prior.n}
                {prior.n > 0 && (
                  <>
                    , hit {(prior.hitRate * 100).toFixed(0)}%, avg {prior.avgR.toFixed(2)}R
                  </>
                )}
              </span>
            );
          })}
        </div>
      )}

      <div className="mt-6 grid grid-cols-1 gap-3 md:grid-cols-2 lg:grid-cols-3">
        {(patterns ?? []).map((pattern) => (
          <div key={pattern.id} className="rounded border border-neutral-800 p-4 text-sm">
            <div className="flex items-center justify-between">
              <span className="font-medium">{pattern.kind}</span>
              <span className="text-xs text-neutral-500">
                {pattern.symbol} · {pattern.regime}
              </span>
            </div>
            <p className="mt-1 text-xs text-neutral-500">
              confidence {(pattern.confidence * 100).toFixed(0)}% · target {pattern.targetPrice.toFixed(5)} ·
              invalidation {pattern.invalidationPrice.toFixed(5)}
            </p>
            <p className="mt-2 text-sm">
              {VERDICT_LABEL[pattern.resolution.verdict]}
              {pattern.resolution.verdict !== "timeout" && (
                <span className="ml-2 text-neutral-400">
                  {pattern.resolution.rMultiple >= 0 ? "+" : ""}
                  {pattern.resolution.rMultiple.toFixed(2)}R
                </span>
              )}
            </p>
          </div>
        ))}
      </div>

      {(patterns ?? []).length === 0 && <p className="mt-4 text-sm text-neutral-500">No patterns for this filter.</p>}
    </main>
  );
}
