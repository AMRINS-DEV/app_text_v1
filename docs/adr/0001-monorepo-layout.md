# ADR 0001: Polyglot Monorepo Layout

## Status

Accepted (Phase 0).

## Context

TradeOS spans five language/runtime boundaries (Rust core, Python agents,
TypeScript gateway/dashboard, MQL5 bridge, plus infra-as-config) that must
share type contracts (`Tick`, `Signal`, `OrderIntent`, strategy configs)
without drifting. The design doc's §P3 (ports & adapters / hexagonal) and
§P4 (everything is an event, everything is replayable) both depend on those
contracts staying in lockstep across languages.

## Decision

One repository, structured per the design doc's §4:

- `crates/` — Cargo workspace. `domain` is the pure-type root every other
  Rust crate depends on; `adapters/*` implement the `MarketDataSource`/
  `Broker` traits so a new platform is a new crate, never a core change.
- `services/gateway` — NestJS BFF, one module per bounded concern (§11.1).
- `services/agents` — a separate `uv` workspace (its own `pyproject.toml`,
  not the repo root's) so Python dependency resolution doesn't bleed into
  the TS/Rust build graphs. Each agent is its own installable package
  (`agents-core`, `agents-llm`, ...) so Phase 5 can add heavy ML
  dependencies to `agents-models` without forcing every other package to
  resolve them.
- `apps/dashboard`, `apps/desktop` + `packages/*` — pnpm/Turborepo workspace.
- `packages/proto` — the cross-language source of truth. Rust gets its
  bindings via `prost-build` at compile time; TS/Python regenerate via
  `packages/proto/generate.sh` (`just proto`).
- `bridge/mt5` — MQL5, deliberately outside every workspace above since it
  can only be built inside a MetaTrader 5 terminal.

## Consequences

- Adding a broker/platform adapter never touches `crates/domain` or
  `crates/strategy` — enforced by the `MarketDataSource`/`Broker` trait
  boundary, proven in Phase 0 by four adapter crates (`mt5`, `tradingview`,
  `ctrader`, `binance`) that all compile against the same traits.
- `cargo build --workspace`, `pnpm -r build`, and `uv sync --all-packages`
  (run from `services/agents`) are the three independent build entry
  points; `just build` runs all three.
- `apps/desktop`'s Tauri build is excluded from `pnpm -r build`'s real work
  (see its `package.json`) because this repository's CI/dev environment
  lacks the system webview dependencies (webkit2gtk/gdk) Tauri needs —
  building it for real requires a machine with those installed.
