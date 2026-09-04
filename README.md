# TradeOS

A multi-agent, multi-platform, latency-tiered algorithmic trading ecosystem.
Full design: see the system design doc this repo implements (Rust core ·
Python agent layer · NestJS gateway · Next.js 15 dashboard · Tauri 2 desktop shell).

## Status

| Phase (§17) | Status |
|---|---|
| 0 · Foundation | Done |
| 1 · Bridge + Core | In progress — tick path done, order path partially blocked (see below) |
| 2 · Execution + Risk | In progress — see below |
| 3 · Features + Model | In progress — see below |
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
- `crates/storage` — trait-shaped stub; needs a real QuestDB/Postgres to be
  meaningful (Phase 3+).

### Phase 2 — Execution + Risk (in progress)

**Real and verified** (`crates/execution`, `crates/risk`):

- `SimBroker` (`crates/execution::sim_broker`) — a real `domain::Broker`
  implementation with configurable reject/requote/partial-fill probability
  (seedable for reproducible tests, plus a `force_next_rejects` hook for
  deterministic guard testing) — the doc's own Prompt 4 ask.
- `OrderRouter` (`crates/execution::router`) — enforces atomic SL/TP at
  submission (§9.3: never a follow-up modify) and idempotent resubmission
  correctly (a client_id that already *succeeded* is never resubmitted; one
  that only failed can still be retried — an easy trap this router's own
  tests catch).
- `domain::Broker` gained a `positions()` method and `PositionSnapshot`
  type — the §9.5 position-reconciliation guard has nothing to compare
  against without it, so this was a real trait gap, not a stub.
- All ten §9.5 safety guards (`crates/risk::guards`), each a small real
  state machine with its own unit tests: daily drawdown, max drawdown
  (manual re-arm only), consecutive losses (reduce size, 2 wins to
  restore), data staleness, clock skew, spread spike (rolling median),
  reject storm (time-boxed circuit breaker), agent unavailability (reduce
  size), kill switch, and position reconciliation.
- Exit management (`crates/risk::exits`, §9.3): stop-distance formula
  (widest of ATR/structure/broker-minimum), chandelier trailing that only
  ever ratchets tighter, breakeven that never undercuts real spread+
  commission cost, time stops.
- Quick-profit (`crates/risk::quick_profit`, §9.4): the gated partial
  scale-out rule plus the shadow A/B tracker the doc mandates —
  `should_recommend_disabling()` implements "if the delta is negative for
  100+ trades" verbatim.
- Soak test (`crates/execution/tests/soak.rs`): 20,000 randomized
  submit/modify/partial-close/full-close cycles through the real
  `OrderRouter`+`SimBroker`, with a local position book maintained purely
  from the `ExecEvent` stream (the way a real core would), reconciled
  every cycle — zero spurious divergence. A second test deliberately
  corrupts the local book and confirms the same guard reliably catches
  it — proving the guard can trip, not just proving it usually doesn't.
  This is the practical stand-in for "72h soak" in an environment with no
  72-hour live feed to run against.

**Still stubbed:** live P&L/margin accounting in `SimBroker` (equity is
static), the strategy VM's real decision tree (still §5.5 config parsing +
a placeholder EMA-cross rule from Phase 0/1), and everything that needs a
trained model or real feature vector (Phase 3).

### Phase 3 — Features + Model (in progress)

**Real and verified** (`crates/indicators`, `crates/features`,
`services/agents/packages/models`, `crates/strategy`):

- Six new O(1) incremental indicators beyond Phase 0's EMA/ATR/RSI:
  Bollinger Bands, Donchian channels, Efficiency Ratio, ADX, swing-point
  detection, and rolling VWAP — each backed by the new `SumRing`/
  `MinMaxRing` primitives (`crates/indicators::ring`, a running-sum and a
  monotonic-deque sliding window) so none of them are O(n) per update.
  Order-flow/liquidity/cross-asset/news/positioning indicators remain
  explicitly out of scope — they need external data feeds this environment
  doesn't have (documented in `crates/indicators::lib`, not silently
  dropped).
- `FeatureEngine` (`crates/features`) rewritten around the expanded
  indicator set into a single `FeatureSnapshot` per bar close, wired
  through `crates/replay::pipeline` and `tradeos-core`'s live path.
- Purged K-fold + embargo and walk-forward window splitting
  (`services/agents/packages/models::cross_validation`, §8.7's overfitting
  defenses) — pure functions over integer bar indices, unit-tested
  including a real embargo-vs-overlap boundary bug caught and fixed during
  development (a closed-interval label window touching a test fold's last
  bar must purge even with zero embargo; the fix corrected both the
  implementation's docstring and the test that had conflated the two
  effects).
- A real GBDT training pipeline (`agents_models.training`): LightGBM →
  isotonic calibration (`agents_models.calibration`, out-of-bounds clipped
  to `[eps, 1-eps]`) → Brier score / reliability diagram / Expected
  Calibration Error / max calibration gap, the exact metric set §8.3 asks
  for. Trained on a synthetic (not market) dataset — there is no real
  historical tick archive in this environment, called out here rather than
  implied otherwise.
- ONNX export (`onnxmltools.convert_lightgbm`) and real Rust inference
  (`crates/strategy::onnx_model`, via the `ort` crate) reading that exact
  file — the §17 Phase 3 exit criterion, "ONNX export with a parity test
  asserting Rust `ort` inference matches Python within 1e-6," passes for
  real: `crates/strategy/tests/onnx_parity.rs` checks 10 rows against a
  committed fixture (`crates/strategy/testdata/onnx_parity/`) generated and
  self-verified by
  `services/agents/packages/models/scripts/generate_onnx_fixture.py`, and
  matches to bit-for-bit float precision, well inside the 1e-6 budget.

**Still stubbed:**

- The training data is synthetic, not market data — there is no ingested
  tick/bar history to train against in this environment. The pipeline
  itself (splitting, training, calibration, export, inference) is real;
  what it's trained on is not.
- The strategy VM's real decision tree (compiling §5.5 YAML into a <5µs
  execution path) and real log-odds signal fusion (§8.4, beyond the
  existing placeholder weighted average) still need a live feature engine
  and calibrated multi-agent signals feeding them to be meaningful — Phase
  5 scope once the agent layer exists.
- `crates/storage` remains a trait-shaped stub pending a real QuestDB/
  Postgres.

### Everything else

- The Tauri desktop shell (`apps/desktop`) is a file-structure stub; building
  the actual GUI needs system webview dependencies not available here.
- No live broker connection and no real LLM provider calls are wired up
  anywhere in this repo — `LlmProvider` implementations in
  `services/agents/packages/llm` are interface-complete stubs.
- `crates/indicators` (O(1) EMA/ATR/RSI, §8.1) is real and unit-tested
  since Phase 0.

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
