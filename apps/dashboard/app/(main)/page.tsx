"use client";

import { ChartHost } from "@tradeos/chart-engine";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef } from "react";

import { useWsClient } from "../../lib/use-ws-client";
import { useAuthStore } from "../../stores/auth";

interface StatsOverview {
  startingEquity: number;
  equityCurve: Array<{ t: number; equity: number }>;
  totalTrades: number;
  winRate: number;
  expectancy: number;
  maxDrawdownPct: number;
  sharpeApprox: number;
}

interface PnlFrame {
  equity: number;
  ts: number;
}

/** Equity curve, P&L and session stats (§4). `GET /api/stats/overview`
 * seeds the chart; the `pnl` WS topic appends live points on top of it. */
export default function OverviewPage() {
  const authorizedFetch = useAuthStore((state) => state.authorizedFetch);
  const containerRef = useRef<HTMLDivElement>(null);
  const hostRef = useRef<ChartHost | null>(null);
  const ws = useWsClient();

  const { data, isLoading, error } = useQuery({
    queryKey: ["stats-overview"],
    queryFn: () => authorizedFetch<StatsOverview>("/api/stats/overview"),
  });

  useEffect(() => {
    if (!containerRef.current) return;
    const host = new ChartHost({ container: containerRef.current });
    hostRef.current = host;
    return () => host.dispose();
  }, []);

  useEffect(() => {
    if (data) hostRef.current?.setLine(data.equityCurve.map((p) => ({ time: p.t, value: p.equity })));
  }, [data]);

  useEffect(() => {
    if (!ws) return;
    return ws.subscribe<PnlFrame>("pnl", (frame) => {
      hostRef.current?.updateLine({ time: frame.payload.ts, value: frame.payload.equity });
    });
  }, [ws]);

  return (
    <main className="p-8">
      <h1 className="text-2xl font-semibold">Overview</h1>
      {isLoading && <p className="mt-2 text-sm text-neutral-500">Loading…</p>}
      {error && <p className="mt-2 text-sm text-red-400">Failed to load stats.</p>}

      {data && (
        <div className="mt-4 grid grid-cols-2 gap-4 sm:grid-cols-4">
          <StatCard label="Win rate" value={`${(data.winRate * 100).toFixed(1)}%`} />
          <StatCard label="Expectancy" value={data.expectancy.toFixed(2)} />
          <StatCard label="Max drawdown" value={`${data.maxDrawdownPct.toFixed(2)}%`} />
          <StatCard label="Sharpe (approx.)" value={data.sharpeApprox.toFixed(2)} />
        </div>
      )}

      <div ref={containerRef} className="mt-6 h-96 w-full rounded border border-neutral-800" />
    </main>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded border border-neutral-800 p-4">
      <div className="text-xs text-neutral-500">{label}</div>
      <div className="mt-1 text-lg font-semibold">{value}</div>
    </div>
  );
}
