/**
 * Built twice (see `package.json`'s `build` script): `dist/esm` (this
 * package's `main`, for bundler consumers like the dashboard — required
 * because `lightweight-charts` itself only exposes an ESM `import`
 * condition, so anything reaching `ChartHost` must stay ESM all the way
 * through) and `dist/cjs` (for a plain-Node CommonJS consumer like the
 * gateway, which only ever deep-imports `dist/cjs/lttb.js` directly —
 * see `services/gateway/src/modules/charts/charts.service.ts`'s import
 * comment for why that specific split exists).
 *
 * §12.2 chart engine. `lttbDownsample`, `ChartHost`, `DataProvider` and
 * `SyncBus` are real Phase 4 work (see each module's own doc comment for
 * what it covers and how it's tested). The series primitives
 * (PatternOverlay, LevelOverlay, SignalMarker, PredictionCone,
 * TradeOverlay, NewsMarker), the WASM indicator bindings, and the
 * `dockview`-based multi-chart workspace with persisted layouts and chart
 * pooling are not — they depend on data (patterns, signals, news, trade
 * history) that doesn't exist until later phases. `README.md`'s Phase 4
 * section has the full "real vs. scoped out" list.
 */
export { lttbDownsample } from "./lttb";
export { ChartHost } from "./chart-host";
export type { Bar, LinePoint, ChartHostOptions } from "./chart-host";
export { DataProvider } from "./data-provider";
export type { DataProviderOptions } from "./data-provider";
export { SyncBus } from "./sync-bus";
export type { CrosshairSync, RangeSync, Unsubscribe } from "./sync-bus";
