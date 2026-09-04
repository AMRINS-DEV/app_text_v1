/**
 * Overview: P&L, equity curve, stats (§4). RSC + streaming, cached
 * materialized views (§12.1) — real data wiring is Phase 4 scope.
 */
export default function OverviewPage() {
  return (
    <main className="p-8">
      <h1 className="text-2xl font-semibold">Overview</h1>
      <p className="mt-2 text-sm text-neutral-500">
        Equity curve, P&amp;L and session stats render here once the gateway&apos;s
        StatsModule (§11.1) is implemented.
      </p>
    </main>
  );
}
