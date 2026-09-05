"use client";

import { ChartPane } from "../../../components/chart-pane";

const SYMBOLS = ["EURUSD", "GBPUSD", "USDJPY", "XAUUSD"];

/**
 * Multi-chart workspace (§4, §12.2). A simple fixed grid of live panes —
 * the `dockview`-based workspace with persisted layouts, chart pooling,
 * and degradation to 1 Hz for inactive panes is later chart-engine work;
 * each pane here is still a real, independently live `ChartPane`.
 */
export default function ChartsPage() {
  return (
    <main className="p-8">
      <h1 className="text-2xl font-semibold">Charts</h1>
      <div className="mt-6 grid grid-cols-1 gap-4 sm:grid-cols-2">
        {SYMBOLS.map((symbol) => (
          <ChartPane key={symbol} symbol={symbol} timeframe="5s" />
        ))}
      </div>
    </main>
  );
}
