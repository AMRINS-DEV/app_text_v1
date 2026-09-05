"use client";

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { useAuthStore } from "../../../stores/auth";

interface FixedHorizonMove {
  movePips: number;
  moveAtr: number;
  direction: "Long" | "Short" | "Flat";
  directionHit: boolean;
}

interface NewsEventRecord {
  id: string;
  ts: number;
  headline: string;
  eventType: string;
  symbol: string;
  impactTier: "low" | "medium" | "high";
  sentiment: number;
  expectedDirection: "Long" | "Short";
  horizonMin: number;
  move: FixedHorizonMove;
}

interface NewsImpactPeriod {
  quarter: string;
  n: number;
  avgImpact: number;
  directionHitRate: number;
}

const SYMBOLS = ["EURUSD", "GBPUSD", "USDJPY"];

/** §12.4's news page: impact timeline + a stability-over-time table backed
 * by §7.2 query #2 (via `GET /api/news/impact-stability`). The Cytoscape/
 * Sigma.js graph explorer from §12.4's full spec needs a graph API exposed
 * over HTTP, which this sandbox's gateway doesn't have yet (the graph
 * layer's own MCP tools are Python-side, §17 Phase 6) — see README's
 * Phase 6 section. */
export default function NewsPage() {
  const authorizedFetch = useAuthStore((state) => state.authorizedFetch);
  const [symbol, setSymbol] = useState<string>(SYMBOLS[0]);

  const { data: timeline } = useQuery({
    queryKey: ["news", symbol],
    queryFn: () => authorizedFetch<NewsEventRecord[]>(`/api/news?symbol=${symbol}`),
  });

  const firstEvent = timeline?.[0];
  const { data: stability } = useQuery({
    queryKey: ["news-impact-stability", firstEvent?.eventType, symbol, firstEvent?.horizonMin],
    queryFn: () =>
      authorizedFetch<NewsImpactPeriod[]>(
        `/api/news/impact-stability?event_type=${firstEvent?.eventType}&symbol=${symbol}&horizon_min=${firstEvent?.horizonMin}`,
      ),
    enabled: Boolean(firstEvent),
  });

  return (
    <main className="p-8">
      <h1 className="text-2xl font-semibold">News</h1>

      <select
        value={symbol}
        onChange={(e) => setSymbol(e.target.value)}
        className="mt-4 rounded border border-neutral-700 bg-transparent px-2 py-1 text-sm"
      >
        {SYMBOLS.map((s) => (
          <option key={s} value={s}>
            {s}
          </option>
        ))}
      </select>

      {firstEvent && (
        <section className="mt-6">
          <h2 className="text-sm font-medium text-neutral-300">
            Impact stability: {firstEvent.eventType} on {symbol} (+{firstEvent.horizonMin}m) — "does this always move
            this pair?"
          </h2>
          <table className="mt-2 w-full text-sm">
            <thead>
              <tr className="text-left text-neutral-500">
                <th className="pb-2">Quarter</th>
                <th className="pb-2">n</th>
                <th className="pb-2">Avg impact (ATR)</th>
                <th className="pb-2">Direction hit rate</th>
              </tr>
            </thead>
            <tbody>
              {(stability ?? []).map((period) => (
                <tr key={period.quarter} className="border-t border-neutral-800">
                  <td className="py-2">{period.quarter}</td>
                  <td className="py-2">{period.n}</td>
                  <td className="py-2">{period.avgImpact.toFixed(2)}</td>
                  <td className="py-2">{(period.directionHitRate * 100).toFixed(0)}%</td>
                </tr>
              ))}
            </tbody>
          </table>
          {(stability ?? []).length === 0 && <p className="mt-2 text-xs text-neutral-500">Not enough history yet.</p>}
        </section>
      )}

      <section className="mt-6">
        <h2 className="text-sm font-medium text-neutral-300">Timeline</h2>
        <table className="mt-2 w-full text-sm">
          <thead>
            <tr className="text-left text-neutral-500">
              <th className="pb-2">Event</th>
              <th className="pb-2">Impact</th>
              <th className="pb-2">Expected</th>
              <th className="pb-2">Realized</th>
              <th className="pb-2">Move (ATR)</th>
            </tr>
          </thead>
          <tbody>
            {(timeline ?? []).map((event) => (
              <tr key={event.id} className="border-t border-neutral-800">
                <td className="py-2">{event.headline}</td>
                <td className="py-2">{event.impactTier}</td>
                <td className="py-2">{event.expectedDirection}</td>
                <td className="py-2">
                  {event.move.direction} {event.move.directionHit ? "✅" : "❌"}
                </td>
                <td className="py-2">{event.move.moveAtr.toFixed(2)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {(timeline ?? []).length === 0 && <p className="mt-4 text-sm text-neutral-500">No news for this symbol.</p>}
      </section>
    </main>
  );
}
