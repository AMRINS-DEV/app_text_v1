"use client";

import { ChartHost, DataProvider, type Bar } from "@tradeos/chart-engine";
import { useEffect, useRef } from "react";

import { useWsClient } from "../lib/use-ws-client";
import { useAuthStore } from "../stores/auth";

interface BarMessage {
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

function toChartBar(bar: BarMessage): Bar {
  return { time: bar.openTime, open: bar.open, high: bar.high, low: bar.low, close: bar.close };
}

const PERIOD_MS: Record<string, number> = { "5s": 5_000, "1m": 60_000, "5m": 300_000, "1h": 3_600_000 };

/** One live candlestick pane in the charts workspace (§4, §12.2): windowed
 * historical fetch via `DataProvider`, then live bar-close updates from
 * the `bars:{sym}:{tf}` WS topic. */
export function ChartPane({ symbol, timeframe }: { symbol: string; timeframe: string }) {
  const authorizedFetch = useAuthStore((state) => state.authorizedFetch);
  const containerRef = useRef<HTMLDivElement>(null);
  const providerRef = useRef<DataProvider<BarMessage> | null>(null);
  const ws = useWsClient();

  useEffect(() => {
    if (!containerRef.current) return;
    const host = new ChartHost({ container: containerRef.current });
    const provider = new DataProvider<BarMessage>({
      fetchWindow: async (fromMs, toMs) => {
        const params = new URLSearchParams({
          sym: symbol,
          tf: timeframe,
          from: String(fromMs),
          to: String(toMs),
          max_points: "300",
        });
        return authorizedFetch<BarMessage[]>(`/api/charts/bars?${params}`);
      },
      timeOf: (bar) => bar.openTime,
      periodMs: PERIOD_MS[timeframe] ?? 60_000,
      onData: (bars) => host.setCandlesticks(bars.map(toChartBar)),
      onAppend: (bar) => host.upsertCandlestick(toChartBar(bar)),
    });
    providerRef.current = provider;

    const now = Date.now();
    void provider.loadWindow(now - 30 * 60_000, now);

    return () => host.dispose();
  }, [symbol, timeframe, authorizedFetch]);

  useEffect(() => {
    if (!ws) return;
    return ws.subscribe<BarMessage[] | BarMessage>(`bars:${symbol}:${timeframe}`, (frame) => {
      const bars = Array.isArray(frame.payload) ? frame.payload : [frame.payload];
      for (const bar of bars) providerRef.current?.appendLive(bar);
    });
  }, [ws, symbol, timeframe]);

  return (
    <div className="flex flex-col">
      <div className="px-1 pb-1 text-xs text-neutral-400">
        {symbol} · {timeframe}
      </div>
      <div ref={containerRef} className="h-64 w-full rounded border border-neutral-800" />
    </div>
  );
}
