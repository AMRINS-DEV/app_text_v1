# TradeOS

A multi-agent, multi-platform, latency-tiered algorithmic trading ecosystem.
Full design: see the system design doc this repo implements (Rust core ·
Python agent layer · NestJS gateway · Next.js 15 dashboard · Tauri 2 desktop shell).

## Status: Phase 0 — Foundation

This repository currently contains the **Phase 0 scaffold** per the design
doc's phased roadmap (§17): monorepo layout, cross-language protobuf
contracts with codegen, a Cargo workspace with domain types and crate
boundaries, a TS workspace (gateway + dashboard + shared packages), a Python
uv workspace for the agent layer, docker-compose infra, CI, and the justfile
dev loop.

**What is real vs. stubbed:**

- `crates/domain` — fully implemented data contracts (`Tick`, `Bar`,
  `Signal`, `OrderIntent`, `ExecEvent`) and the `MarketDataSource`/`Broker`
  port traits. No I/O, no unsafe, matches §5.3–5.4 exactly.
- `crates/indicators` — real O(1) incremental EMA/ATR/RSI implementations
  (the pattern every indicator in §8.1 follows), with unit tests.
- `crates/risk` — real fractional-Kelly sizing math (§9.2), unit tested.
- Every other crate/service/package is a compiling skeleton: correct module
  boundaries, correct trait/interface signatures, `todo!()`/`NotImplementedError`
  bodies. They exist so later phases (§17: Bridge+Core, Execution+Risk,
  Features+Model, Dashboard, Agent layer, Graph, Validation, Multi-platform)
  have the right seams to build into — not because the business logic is
  implemented yet.
- The MT5 bridge (`bridge/mt5`) is MQL5 source structure only — it cannot be
  compiled or tested outside a MetaTrader 5 terminal, which this environment
  does not have.
- The Tauri desktop shell (`apps/desktop`) is a file-structure stub; building
  the actual GUI needs system webview dependencies not available here.
- No live broker connection and no real LLM provider calls are wired up
  anywhere in this scaffold — `LlmProvider` implementations in
  `services/agents/packages/llm` are interface-complete stubs.

## Layout

See `docs/adr/0001-monorepo-layout.md` for the directory structure rationale,
and the design doc §4 for the full target tree.

## Getting started

```bash
just infra-up     # datastores + observability via docker compose
just build        # Rust workspace + TS workspace + Python packages
just test         # cargo nextest + pnpm test + pytest
just dev          # everything in dev mode
```

Requires: Rust (stable), Node 22+, pnpm 10+, Python 3.11+, `uv`, Docker.

## Build order

Per §20 of the design doc: **bridge → core → risk → validation → agents**.
Building the AI layer before the execution path is working produces an
impressive demo and no tradable system — resist that order.
