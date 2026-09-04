# TradeOS

A multi-agent, multi-platform, latency-tiered algorithmic trading ecosystem.
Full design: see the system design doc this repo implements (Rust core ·
Python agent layer · NestJS gateway · Next.js 15 dashboard · Tauri 2 desktop shell).

## Status

| Phase (§17) | Status |
|---|---|
| 0 · Foundation | Done |
| 1 · Bridge + Core | In progress — tick path done, order path partially blocked (see below) |
| 2 · Execution + Risk | Not started |
| 3 · Features + Model | Not started |
| 4 · Dashboard v1 | Not started |
| 5 · Agent layer | Not started |
| 6 · Graph + Knowledge | Not started |
| 7 · Validation | Not started |
| 8 · Multi-platform | Not started |
| 9 · Scale | Not started |

### Phase 0 — Foundation

Monorepo layout, cross-language protobuf contracts with codegen, a Cargo
workspace with domain types and crate boundaries, a TS workspace (gateway +
dashboard + shared packages), a Python uv workspace for the agent layer,
docker-compose infra, CI, and the justfile dev loop.

### Phase 1 — Bridge + Core (in progress)

**Real and verified** (`crates/adapters/mt5`, `crates/bin/mock-mt5-bridge`,
`crates/replay`):

- The MT5 bridge wire protocol (`docs/protocol.md`) is implemented and
  tested end-to-end over real ZMQ sockets: `Mt5MarketData`/`Mt5Broker`
  (`crates/adapters/mt5`) against `mock-mt5-bridge` — a Rust test double
  standing in for the MQL5 EA, which this environment cannot compile or run.
- Tick frames are rkyv zero-copy, byte-for-byte verified against the real
  archived layout (`crates/domain/examples/dump_tick_layout.rs`), including
  a real alignment bug (`rkyv::access` on an unaligned sub-slice) found and
  fixed by the integration tests, not assumed away.
- Measured bridge→core latency (pure-Rust ZMQ over TCP loopback, this
  sandbox): p50 ≈ 36µs, p99 ≈ 113µs, p999 ≈ 189µs — honestly reported
  against the design doc's p99 < 50µs budget, which assumes native `libzmq`
  or a shared-memory transport, neither of which this crate uses (see
  `docs/protocol.md`'s "Order path latency" section for why).
- Deterministic replay-through-pipeline (`crates/replay::pipeline`): feeds
  a recorded tick log through the real bar-aggregator/feature-engine/risk-
  sizing crates and asserts two runs are byte-identical — the actual
  property behind "24h replay is bit-identical" (§17), proven at whatever
  scale a `.jsonl` file covers. `tradeos-replay --generate-sample` /
  `--input` exercises this from the CLI.

**Found, not yet fixed:** the order path (`OrderRequest`/`OrderReply`) is
rkyv-encoded and only decodable by a Rust peer — `Option`/`String`/
`SmallVec` fields use relative pointers into the archive buffer that a
hand-written MQL5 decoder cannot reproduce. A real MT5 connection needs a
separate flat encoding for orders first (see `docs/protocol.md`'s
"Important correction"); this is scoped, not silently skipped.

**Still stubbed:**

- The MQL5 EA (`bridge/mt5`) — cannot be compiled or tested outside a real
  MT5 terminal, which this environment does not have. Its tick-publishing
  half is concrete pseudocode against the verified wire format; its
  order-accepting half is intentionally unwritten (see above).
- Reconnect-with-resync on the adapter side — Phase 2 scope, and only
  testable against a real EA that can actually disconnect.
- `crates/risk::guards`, `crates/execution`, `crates/storage` — trait-shaped
  stubs for Phase 2.

### Everything else

- The Tauri desktop shell (`apps/desktop`) is a file-structure stub; building
  the actual GUI needs system webview dependencies not available here.
- No live broker connection and no real LLM provider calls are wired up
  anywhere in this repo — `LlmProvider` implementations in
  `services/agents/packages/llm` are interface-complete stubs.
- `crates/indicators` (O(1) EMA/ATR/RSI, §8.1) and `crates/risk::sizing`
  (fractional-Kelly, §9.2) are real and unit-tested since Phase 0.

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
