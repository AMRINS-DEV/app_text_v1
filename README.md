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
| 4 · Dashboard v1 | In progress — see below |
| 5 · Agent layer | In progress — see below |
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

### Phase 4 — Dashboard v1 (in progress)

Scoped to §17's own Phase 4 exit list — "Auth, overview, charts workspace,
positions, kill switch, settings" — rather than the design doc's much
larger Prompt 8-10 wish list (patterns/news/signals/agents/backtest/journal
need the agent layer and graph store from later phases to be meaningful,
and stay Phase 0 stubs here).

**Real and verified** (`services/gateway`, `packages/chart-engine`,
`apps/dashboard`):

- Auth (`services/gateway/src/modules/auth`): real JWT (access/refresh,
  rotated on use) + a hand-rolled RFC 4226/6238 HOTP/TOTP implementation
  (`totp.ts`) — not `otplib`, whose default base32 plugin is ESM-only and
  broke this workspace's CommonJS Jest setup; verified against RFC 4226
  Appendix D's own published test vectors, not just round-tripped. Step-up
  auth (§13) mints a separate short-lived token after a second TOTP check,
  enforced by its own guard on `mode` and `kill-switch/reset` — the kill
  switch itself deliberately has no step-up (§11.2: "immediate
  flatten+halt"). Four seeded dev accounts (owner/trader/analyst/viewer),
  password `<username>-dev-password`; no real user database exists here.
- RBAC (`common/roles.guard.ts`, `step-up.guard.ts`): real per-route guards
  (`@Roles`, `@RequireStepUp`), plus a genuine NestJS DI gotcha found and
  fixed — a class passed to `@UseGuards()` is resolved against the
  *hosting controller's own module* injectables, not merely inherited from
  an imported module's exports, so each protected module re-declares the
  three guards as its own providers (see `trading.module.ts`'s doc
  comment).
- Realtime WS gateway (`modules/realtime`): topic subscribe/unsubscribe,
  MessagePack binary framing, 50-100ms coalescing, per-topic RBAC (one
  concrete example: `agent_status` excludes `viewer`), and
  backpressure-to-conflation keyed on `bufferedAmount` — verified with a
  real `ws` client against a booted Nest app (not just guard unit tests):
  auth-on-connect rejection, a real coalesced tick batch, unsubscribe
  actually stopping delivery, and RBAC denial. Ticks/bars are a
  deterministic synthetic feed (`MarketFeedService`) — this sandbox has no
  live MT5/core connection to draw from, the same split as Phase 1's mock
  bridge.
- Trading (`modules/trading`): `CoreClient` is the exact port a real gRPC
  client to `tradeos-core` would expose; `InMemoryCoreClient` implements it
  in-process, the same "real logic, mock infrastructure" split as
  `SimBroker`. Kill switch flattens and audit-logs atomically, measured at
  well under §17's 500ms budget in-process — that number can't stand in
  for real broker round-trip latency, only for "the code path itself does
  no unnecessary blocking work."
- Settings, stats, charts (`modules/settings|stats|charts`): real zod-
  validated CRUD; `/api/stats/overview`'s equity curve and Sharpe/drawdown
  come from a deterministic synthetic trade history (fixed reference
  instant, not `Date.now()` — a real bug caught here: the first version
  used `Date.now()` as a default and silently broke its own "deterministic"
  claim between two `StatsService` instances built a millisecond apart);
  `/api/charts/bars` generates deterministic historical OHLC keyed on
  `(symbol, tf, openTime)` and downsamples via a real LTTB implementation.
- `packages/chart-engine`: `lttbDownsample` (Largest-Triangle-Three-
  Buckets, a real bug fixed here too — the first version anchored its
  candidate-bucket and averaging-bucket ranges to the wrong offsets and
  duplicated the series' last point), `ChartHost` (TradingView Lightweight
  Charts v5 lifecycle), `DataProvider` (windowed fetch + live append + gap
  detection), `SyncBus` (crosshair/range sync). Built twice — `dist/esm`
  for bundler consumers and `dist/cjs` for the gateway's plain-Node
  `lttbDownsample` import — because `lightweight-charts` itself only
  exposes an ESM `import` condition; a single CJS build broke the
  dashboard's webpack bundling once `ChartHost` needed it.
- Dashboard (`apps/dashboard`): real two-step login (password → TOTP) →
  overview (equity curve + stats, live-updated over the `pnl` topic) →
  trading (live positions table, kill switch, step-up-gated mode/reset) →
  settings (risk profile, allowed pairs, mode) → charts (a 4-pane grid,
  each pane a real `ChartHost` fed by `/api/charts/bars` plus live
  `bars:{sym}:{tf}` WS updates). `TopicMultiplexer` carries the WS client's
  ref-counted subscribe/dispatch logic, deliberately split from the real
  `WebSocket`/`Worker` wiring so it's unit-tested without a browser — the
  same reason `chart-engine`'s `DataProvider`/`SyncBus` don't touch the
  DOM. A real end-to-end pass (Playwright against Chromium, driving actual
  login → TOTP → overview → trading → charts against a live gateway)
  confirmed the whole stack renders and streams correctly — that pass also
  caught a real, repo-wide bug: the dashboard's own `tsconfig.json` never
  included the `DOM` lib, so every DOM event-handler type in the app
  (`onChange`, etc.) silently degraded to a useless partial type. Fixed
  once, for every page.

**Still stubbed / honestly out of reach here:**

- No real gRPC client to a live `tradeos-core` process, and no real
  Postgres/Redis/QuestDB — every store above is in-memory, consistent with
  every prior phase's "mock infrastructure, real logic" split.
- §12.2's `dockview`-based multi-pane workspace with persisted layouts and
  chart pooling, the WASM indicator bindings, and the custom series
  primitives (PatternOverlay, SignalMarker, PredictionCone, etc.) — all
  depend on data (patterns, signals, agent output) that doesn't exist
  until Phase 5/6. The charts page here is a plain fixed grid of real,
  independently live panes, not a pooled/persisted workspace.
- §17's "8 live panes at ≥55 fps, verified with a Playwright performance
  trace" is not separately re-measured as a formal perf benchmark — the
  end-to-end pass above confirms the panes render and update live, but a
  dedicated frame-rate trace under sustained 200 tick/s load is unclaimed.
- `packages/ui` remains a stub; pages use plain Tailwind rather than a
  shared shadcn/ui component set.
- Envelope-encrypted secret storage (§11.1's `secrets` module) isn't in
  §17's Phase 4 exit list and stays a Phase 0 stub — provider API keys tie
  to the agent layer (Phase 5) anyway.

### Phase 5 — Agent layer (in progress)

Scoped to §17's own Phase 5 exit row — "LLM router (4 providers), news/
pattern/regime/critic agents, semantic cache, MCP servers | Signals
published with TTL; cache hit ≥40%; cost under cap" — rather than the design
doc's larger Prompt 6 wish list (LangGraph, `timeseries`/`graph` MCP
servers, vision-agent/flow-agent, research-agent all need infrastructure or
data this phase doesn't have and stay out of scope here).

**Real and verified** (`services/agents/packages/{llm,core,regime,pattern,
news,critic,mcp,orchestrator}`):

- LLM providers (`agents-llm`): `OpenAIProvider`, `AnthropicProvider`,
  `DeepSeekProvider`, `KimiProvider` build the exact real request shape and
  parse the exact real response shape for each provider's actual API
  (OpenAI Chat Completions, Anthropic Messages, OpenAI-compatible for
  DeepSeek/Kimi) — verified with `httpx.MockTransport`, never a live network
  call, API key, or real spend. Each provider carries a real circuit breaker
  (3 consecutive failures → unhealthy for a 60s cooldown), verified with an
  injected fake clock so the cooldown-then-recovery transition is actually
  exercised, not assumed.
- `LlmRouter`: capability-filtered fallback chain (vision/tools/ctx_len/
  json_mode), per-provider and per-agent spend caps, an `asyncio.wait_for`
  latency budget per candidate, and a full audit trail (prompt hash,
  provider, model, tokens, cost, latency, cache hit, agent, signal ID) —
  33 tests across fallback, capability filtering, circuit-breaker
  integration, spend caps, and the audit log.
- `SemanticCache` (§ L3a): bag-of-words cosine-similarity lookup ahead of
  any provider call; its ≥40% hit-rate target is met against a workload
  deliberately built from measured cosine similarities (near-duplicate
  paraphrases + genuinely novel queries), not asserted against a workload
  shaped to pass.
- `structured_output.parse_structured`: JSON-parse + Pydantic-validate with
  one retry-and-repair pass on malformed output.
- Guardrails (§10.4, `agents_core.guardrails`): `implausible_levels` cross-
  checks agent-claimed numeric levels against real OHLCV/ATR context
  (>0.1 ATR mismatch discards the signal), `wrap_untrusted_text` isolates
  news text as delimited data a prompt can never mistake for instructions,
  `calibration_key` folds model version into the calibration key, and
  `ModelVersionWeight` ramps a new model version's weight in over 30
  resolved predictions rather than trusting it immediately.
- `SignalBus` (§10.3): validates TTL, then `features_hash` against a
  registered snapshot, then the calibrated probability range, in that
  order, before a signal ever reaches fusion — agents hold zero
  order-placement authority by construction, not by convention.
- Agent roster (`AGENT_ROSTER`): `news-agent` (two-step triage → deep LLM
  call gated on `impact_score ≥ 0.7`, discards hallucinated numeric levels
  via the guardrail above), `pattern-agent` (real swing-point geometry
  detecting double top/bottom, optional LLM narrative), `regime-agent` (see
  the honest note below), `critic-agent` (approve/veto with an
  `outcome_tracker` recording every veto for §10.1's weight-adjustment
  measurement, whether or not anything gets published).
- `Orchestrator` (`agents-orchestrator`): wires regime → {pattern, news} →
  critic → `SignalBus` per bar; a `Flat`/zero-confidence proposal is each
  agent's own "nothing to report" convention and is never even sent to the
  critic — verified explicitly (`test_no_pattern_found_means_the_critic_is
  _never_even_consulted`), not just assumed from the code path.
- Two MCP tool servers (`agents-mcp`, the official `modelcontextprotocol`
  Python SDK's `FastMCP`, pinned to `mcp<2` after a suspicious `2.1.1`
  resolve pulled in forked sub-dependencies): `get_bars`, `get_trade_history`,
  both against synthetic-but-deterministic data (a fixed reference instant,
  never `time.time()`, following the same rule Phase 4's `StatsService` bug
  taught).
- Whole-workspace verification: 134 tests pass (`uv run pytest`), `ruff
  check .` and `mypy packages/*/src` both clean.

**Still stubbed / honestly out of reach here:**

- No real LLM API keys, no live network calls, no real spend anywhere —
  every provider is tested against its real request/response shape via
  `httpx.MockTransport`, nothing more.
- `regime-agent`'s classifier is k-means + a closed-form per-cluster
  Gaussian, not a learned-transition HMM: `hmmlearn.GaussianHMM`'s
  Baum-Welch EM reliably collapsed to a degenerate single dominant state on
  this data shape, regardless of k-means-seeded means, manually seeded
  covariances/transition matrix, or a tiny `min_covar` — verified via direct
  diagnostic scripts that the synthetic data itself was well-separated
  before concluding the fitting algorithm was the problem. Documented here
  rather than silently swapped.
- `vision-agent` and `flow-agent` remain `NotImplementedError` stubs (need a
  chart-rendering pipeline and a live DOM feed respectively) and are
  deliberately left out of `AGENT_ROSTER` rather than wired in to fail at
  call time; `research-agent` is out of Phase 5 scope entirely.
- No real NATS — agent-to-core signal delivery is in-process, the same
  "real logic, mock infrastructure" split as every prior phase's
  `SimBroker`/`InMemoryCoreClient`.
- No real Qdrant — the semantic cache is in-process bag-of-words cosine
  similarity, not a real embedding-based vector store.
- Not LangGraph — the orchestrator is a plain async pipeline; see its own
  module doc comment for why that's a deliberate scoping decision against
  §17's actual Phase 5 exit criteria, not a shortcut.
- The `timeseries` (QuestDB) and `graph` (FalkorDB, §7.2) MCP servers need
  infrastructure this sandbox doesn't have and are Phase 6+ scope, same as
  `crates/storage`.

### Everything else

- The Tauri desktop shell (`apps/desktop`) is a file-structure stub; building
  the actual GUI needs system webview dependencies not available here.
- No live broker connection is wired up anywhere in this repo — every
  provider/bridge above is real logic against mock infrastructure.
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
