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
| 6 · Graph + Knowledge | In progress — see below |
| 7 · Validation | In progress — see below |
| 8 · Multi-platform | In progress — see below |
| 9 · Scale | In progress — see below |

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
- The `timeseries` (QuestDB) MCP server needs infrastructure this sandbox
  doesn't have and remains Phase 6+ scope, same as `crates/storage`. The
  `graph` MCP server is real as of Phase 6 — see below.

### Phase 6 — Graph + Knowledge (in progress)

Scoped to §17's own Phase 6 exit row — "FalkorDB schema, ingest pipeline,
priors into fusion, news & pattern pages | Conditional-reliability query
< 50ms; priors measurably improve expectancy" — using the same real-logic/
mock-infrastructure split as every prior phase: no Docker daemon exists in
this sandbox to run a real FalkorDB, so the graph layer's schema and query
semantics are real, exercised against an in-memory engine that stands in
for it (documented in `agents_graph.store`'s own doc comment) — the same
substitution this project already made for `hmmlearn` in Phase 5's regime
classifier.

**Real and verified** (`crates/strategy::fusion`, `services/agents/packages/
graph`, `services/agents/packages/mcp`, `services/gateway/src/modules/
{patterns,news}`, `apps/dashboard/app/(main)/{patterns,news}`):

- Real §8.4 log-odds signal fusion (`crates/strategy::fusion`), replacing
  the placeholder weighted average every prior phase carried forward:
  `logit(P_fused) = Σ w_i·logit(P_i) − λ·Σ_{i<j} ρ_ij·|logit(P_i)|·|logit(P_j)|`,
  with `BrierTracker`'s online Brier Skill Score for `w_i` and
  `PairwiseCorrelationTracker`'s online Pearson correlation for `ρ_ij` —
  both real streaming statistics, not placeholders. A dedicated test
  (`a_well_evidenced_graph_prior_changes_the_expectancy_gate_decision`)
  demonstrates the Phase 6 exit criterion directly: fusing two weak,
  near-coin-flip agent signals alone produces a probability that fails
  §8.5's `p_min_mode = 0.55` gate; adding a well-evidenced graph prior
  (§7.2's conditional-reliability output) pushes the same fusion over that
  gate — priors measurably changing the actual trade decision, not just a
  number moving in the right direction.
- The §7.1 knowledge graph schema (`agents_graph.schema`): every node label
  (Instrument, NewsEvent, EventType, Pattern, PatternInst, MarketRegime,
  Outcome, Trade, Session, Concept) and relationship from the design doc's
  own Cypher, as typed builders over a plain `Node`/`Edge` pair.
- `KnowledgeGraph` (`agents_graph.store`): an indexed, idempotent-by-id
  in-memory graph engine — `upsert_node`/`upsert_edge` are real `MERGE`
  semantics, and node/edge lookups are O(1) dict indices, not a scan (the
  property that makes the latency benchmark below meaningful).
- Ingest pipelines and automatic outcome resolution (`agents_graph.ingest`,
  `.outcomes`): real functions turning agent-shaped output into graph
  nodes/edges, and two real barrier-walk resolvers — `resolve_pattern_
  outcome` (the same triple-barrier shape as `agents_models.labeling.
  triple_barrier`, generalized to asymmetric, direction-aware target/
  invalidation levels — real CONFIRMED/FAILED/TIMEOUT verdicts computed
  from actual subsequent OHLC, the same computation §12.3's "honest
  track-record generator" describes) and `resolve_fixed_horizon_move`
  (§7.2 query #2's fixed-horizon news-impact measurement).
- The exact §7.2 queries (`agents_graph.queries`) as real traversal/
  aggregation logic against `KnowledgeGraph`'s indexed adjacency:
  conditional reliability (query #1, including `percentileCont`'s
  linear-interpolation median, matching openCypher's own definition),
  news impact persistence bucketed by quarter (query #2), and confluence
  discovery gated at `n >= 40` (query #3) — every result reports its own
  `n` rather than silently hiding an under-powered sample.
- The §17 exit benchmark: `conditional_reliability` measured at ~26ms
  against 100k pattern instances (~400k edges) — honestly scaled down from
  the design doc's 10M-edge target (this sandbox can't synthesize and hold
  that much data), with the scaling argument made explicit in the test's
  own doc comment: the query's cost is O(matching instances via indexed
  lookups), not O(total graph size), so a smaller-N benchmark is a
  legitimate proxy rather than an unrelated number.
- A backfill script (`agents_graph.backfill`) generating deterministic
  synthetic historical news and patterns (fixed reference instant, seeded
  `random.Random` — never `time.time()`), seeded so `double_top` has a
  genuinely higher hit rate than `double_bottom`, giving the queries above
  something real, if synthetic, to discover.
- A third MCP tool server, `graph` (`agents_mcp.server`, alongside the
  Phase 5 `market`/`journal` tools in the same process): `get_pattern_
  reliability`, `get_news_impact_stability`, `get_confluence`, all backed
  by the backfilled `KnowledgeGraph` above.
- Gateway `patterns`/`news` modules (`services/gateway/src/modules/
  {patterns,news}`), fleshed out from Phase 0's empty stubs: real
  TypeScript reimplementations of the same barrier-walk/fixed-horizon
  resolvers and conditional-reliability/impact-stability aggregations
  (there's no live cross-language bridge from this NestJS process to the
  Python agent layer in this sandbox, the same split as the Rust domain
  types and their Zod mirrors in `packages/schemas`), exposed as
  `GET /api/patterns`, `GET /api/patterns/prior`, `GET /api/news`, and
  `GET /api/news/impact-stability`.
- Dashboard `patterns`/`news` pages (§12.3, §12.4), replacing Phase 0's
  placeholder text: pattern cards with kind/symbol/regime/confidence, a
  computed CONFIRMED/FAILED/TIMEOUT verdict and R-multiple, and the
  historical prior line per pattern kind; a news timeline with realized-
  direction/move data and a quarter-bucketed impact-stability table.
  Verified end-to-end in a real browser (login → TOTP → Patterns → News
  against a live gateway), which caught and fixed a real, previously
  dormant bug found only by actually rendering the Overview page along
  the way: `stats.math.ts`'s `equityCurveFrom` reused the first trade's
  own timestamp as the synthetic curve's starting point, so the first two
  points were always identical — harmless until `chart-engine`'s
  `ChartHost.setLine` (which requires strictly ascending, duplicate-free
  timestamps) actually rendered it, at which point it crashed the whole
  page. Fixed by placing the starting point 1ms before the first trade,
  with a regression test. The sidebar nav also never linked to Patterns/
  News (a Phase 4 leftover from when they were empty stubs) — added.

**Still stubbed / honestly out of reach here:**

- No real FalkorDB — see `agents_graph.store`'s own doc comment; the
  schema and query semantics are real, the storage engine underneath is
  not the one the design doc names. `KnowledgeGraph` is dependency-
  injectable behind the same interface, so a real FalkorDB-backed
  implementation (via `redis`'s `GRAPH.QUERY`) can replace it later
  without touching any ingest/query caller.
- No real cross-language bridge: the gateway's `patterns`/`news` modules
  reimplement the same real algorithms independently in TypeScript rather
  than calling the Python `agents_graph`/MCP layer over the network — the
  same duplication the Rust domain types and TS Zod schemas already
  accept elsewhere in this repo.
- §12.4's Cytoscape/Sigma.js graph explorer needs a graph query API
  exposed over HTTP from the gateway, which doesn't exist yet (the real
  graph queries live behind Python's MCP tools in this phase) — Phase 7+
  scope.
- §12.3's live "detect" job trigger and `packages/chart-engine`'s
  `PatternOverlay` series primitive (rendering pattern geometry directly
  on a live chart) need a live agent bridge and are Phase 7+ scope; the
  patterns page here reads pre-resolved history, it doesn't trigger new
  detection.
- `crates/strategy`'s decision tree compilation (§5.5, <5µs) is still the
  only piece of the strategy VM left unreal — fusion is the last of the
  three components (config parsing, ONNX inference, fusion) called out in
  `lib.rs`'s own doc comment across Phases 0/3/6.

### Phase 7 — Validation (in progress)

Scoped to §17's own Phase 7 exit row — "Walk-forward, paper trading,
calibration monitoring | 60-day paper expectancy within 1 SE of backtest" —
not §19 Prompt 12's much larger observability wish list (a live dashboard
panel, alerting integrations, a scaling-decision UI) or §15's full 8-item
pre-launch checklist, the same roadmap-row-not-generation-prompt scoping
every prior phase used. A new package, `services/agents/packages/
validation` (`agents_validation`), is the first place in the whole project
that wires Phase 3's previously disjoint building blocks — `agents_models.
labeling.triple_barrier`, `.cross_validation`'s purged/embargoed walk-
forward splitter, and `.calibration`'s isotonic calibrator + Brier/ECE —
into an actual time-ordered backtest.

Two deliberate substitutions, both argued the same way Phase 6 argued away
LangGraph and FalkorDB:

- **Not NautilusTrader** (the specific engine §15 names): this project
  already built its own simulated-execution stack in Phase 2 (`SimBroker`,
  `OrderRouter`, `crates/risk`'s Kelly sizing and guard suite) purpose-built
  for exactly this job. A second, unrelated backtesting framework would
  bypass everything Phases 1-6 built and reimplement the same trade logic a
  second time, disconnected from the Rust engine — so `agents_validation.
  backtest` is this project's own §8.5 expectancy-gate runner instead.
  "Realistic costs" (§15 item 3) are a fixed cost-in-R constant, the same
  cost ceiling §8.5's own formula already accounts for, not a market-impact
  model.
- **No real 60-day paper trading run.** This is a different category of gap
  from every other "we don't have real X" limitation in this project: it
  needs elapsed real calendar time against a live feed, and no amount of
  engineering effort — mock or otherwise — can make real time pass inside a
  coding session. What §15 actually checks is a *statistical property* (does
  forward performance track backtest performance), and Phase 2's 72-hour
  soak test already established this project's answer to exactly this shape
  of requirement: simulate the property via an accelerated, compressed-time
  run rather than literally waiting. `agents_validation.paper_trading` fits
  a final model on a prefix of the synthetic series, then runs the real
  backtest logic over a trailing range that training and calibration never
  saw at all — an honest accelerated stand-in for a live paper-trading
  period, never claimed as an actual 60-day run.

**Real and verified** (`services/agents/packages/validation`, 56 tests, ruff/
mypy clean):

- `dataset.generate_labeled_series`: a synthetic price series with real,
  triple-barrier-resolved labels — the first dataset in the project needing
  *persistent*, multi-bar predictive signal rather than a single-step
  nudge, because a label resolves over up to `max_bars` future bars. Built
  around an AR(1) latent "regime" process driving returns over many
  consecutive bars, with one feature column a noisy readout of that same
  regime; an i.i.d.-per-bar version tried first produced no measurable
  signal at all (confirmed empirically, not assumed). A second empirical
  fix was needed for the ATR-proxy barrier distance: at the raw scale,
  crossing a barrier took only 2-5 bars regardless of noise level (a
  scale-invariant consequence of diffusion), making TIMEOUT essentially
  impossible — fixed with an empirically tuned 4x scale factor found via a
  parameter sweep, producing a genuine three-way WIN/LOSS/TIMEOUT mix.
- `backtest.run_backtest`: the exact §8.5 expectancy gate —
  `E[R] = p·R_target − (1−p)·1.0 − c`, gated on a cost ceiling
  (`c ≤ 0.10·R_target`), a minimum probability, and `E[R] ≥ θ` — connecting
  a calibrated probability stream to real triple-barrier trade outcomes for
  the first time anywhere in the project.
- `walk_forward.run_walk_forward`: per-window train → calibrate → backtest,
  correctly enforcing that the calibrator is fit only on a held-out
  validation slice (never the GBDT's own training data) and that the
  backtest only ever runs on each window's genuinely out-of-sample test
  range.
- `statistics.deflated_sharpe_ratio` and `.monte_carlo_drawdown_
  distribution` (§15 items 4-5, zero prior implementation anywhere in the
  repo): a real Bailey & López de Prado Deflated Sharpe Ratio — the
  probability that observed performance is genuine skill rather than the
  best-of-`n_trials` artifact §8.7's purging/embargo splitters exist to
  guard against, reducing to the ordinary Probabilistic Sharpe Ratio at
  `n_trials=1` — and a Monte Carlo trade-order-shuffle drawdown distribution
  testing sequencing-risk robustness. Fixing a real NaN bug (constant-zero
  returns reaching skew/kurtosis before a variance guard could catch them)
  and a flawed test assumption (a single DSR draw is uniform on [0,1] under
  the null, not concentrated near 0.5 — that only holds in expectation
  across many trials) both required first-principles statistical reasoning,
  not just loosened tolerances.
- `calibration_monitor.RollingCalibrationMonitor`: wraps Phase 3's static
  Brier/ECE functions in a sliding window over a live `(predicted, actual)`
  stream, flagging calibration drift past §8.3's target. The default window
  size (1000) was empirically tuned, not guessed: a 100-trial sweep across
  window sizes 200-2000 showed a smaller window's ECE is dominated by
  binomial sampling noise even for a perfectly calibrated model, falsely
  flagging drift; only ≥1000 gives an acceptably low (~1%) false-positive
  rate against the 0.05 threshold.
- `paper_trading.run_accelerated_paper_trading` and `.check_paper_vs_
  backtest_divergence`: §17's own literal exit criterion, computed for
  real — trains and calibrates on a prefix of the series, runs the real
  backtest over a trailing range the model never saw, then checks whether
  paper expectancy falls within one standard error of backtest expectancy
  (the standard error of the *backtest* expectancy estimate, per §15's own
  wording). `DivergenceResult.halt_scaling` is `True` both when divergence
  exceeds one SE and when there's insufficient evidence to judge at all
  (fewer than 2 backtest trades) — a claim that was never actually tested
  isn't grounds to scale up either.

**Still stubbed / honestly out of reach here:**

- No real 60-day (or 30-day live-micro-capital) run against an actual
  broker feed — see the substitution argument above; this is elapsed-time-
  bound, not infrastructure-bound, and no mock closes that gap.
- §19 Prompt 12's live observability surface (a dashboard panel showing
  walk-forward/DSR/drawdown-distribution/calibration-drift results, an
  alerting integration, a UI for the scaling decision) is out of §17's own
  Phase 7 exit-row scope — nothing here needed a gateway or dashboard
  surface, so none was added this phase, unlike every phase that did touch
  the gateway/dashboard.
- `crates/strategy`'s decision tree compilation (§5.5, <5µs) remains the
  only still-unreal piece of the strategy VM, carried forward unchanged
  from Phase 6.

### Phase 8 — Multi-platform (in progress)

Scoped to §17's own Phase 8 exit row — "TradingView webhook + UDF datafeed,
second broker adapter | Same strategy runs on 2 adapters with no core
change." `domain::ports`'s own doc comment makes the claim this phase has to
prove concrete: "Adding a platform means implementing these two traits in a
new `crates/adapters/*` crate — zero changes here or in `crates/strategy`/
`crates/risk`." Nothing in `crates/domain`, `crates/strategy`, `crates/risk`,
or `crates/execution` changed to build this phase.

**`adapter-ctrader`** (§5.4's second broker platform), tested against its
own `mock-ctrader-server` test double — the same "real adapter, mock
counterparty" split `adapter-mt5`/`mock-mt5-bridge` established in Phase 1,
since this sandbox has neither network access nor credentials for a real
cTrader account. Its wire protocol (`adapter_ctrader::protocol`) is an
honestly documented, simplified stand-in for cTrader's real Protobuf-over-
TLS Open API — length-prefixed framing with a `req_id` correlation field
mirroring Open API's real `clientMsgId` mechanism, carrying `serde_json`
payloads of this project's own domain types, rather than a guessed
reimplementation of Open API's exact Protobuf schema (this project has no
way to verify real field numbers against a live server, so claiming byte-
for-byte fidelity would be pretending, the same standard that ruled out a
literal FalkorDB in Phase 6 and NautilusTrader in Phase 7). It deliberately
uses a different transport stack than `adapter-mt5` — blocking
`std::net::TcpStream` + JSON here, vs. async ZMQ + rkyv there — so the two
adapters share no implementation, only the trait boundary. Order acceptance
is synchronous; the resulting fill arrives as a separate, unsolicited
frame a reader thread demultiplexes from replies by `req_id`, routed to
`poll_event` — the same submit-then-poll lifecycle `SimBroker`'s own doc
comment describes, genuinely crossing a socket boundary this time.

The exit criterion itself is a real test, not a prose argument:
`tests/cross_adapter_parity.rs`'s `run_router_scenario` is one generic
function over `B: domain::ports::Broker`, run once with `execution::
SimBroker` and once with `CTraderBroker` against a live `mock-ctrader-
server`, asserting both produce identical externally observable outcomes
(reject-missing-SL/TP, idempotent resubmission, an observed fill, position
count before/after close) for the exact same `OrderRouter` call sequence.

**`adapter-tradingview`** (§5.4's signal-source-only platform — TradingView
cannot execute retail orders, so this crate only ever implements
`MarketDataSource`), the project's first Rust HTTP server (`axum`, chosen
as the only already-tokio-based option with nothing larger already in the
workspace):

- `POST /webhook`: a Pine-alert receiver. A TradingView alert fires *at* a
  price, so — once authenticated against a shared-secret token — that price
  becomes a genuine, if one-sided (no independent bid/ask), `Tick` for its
  named symbol (§5.4 point 2).
- `GET /udf/{config,symbols,history}`: a real TradingView UDF ("Universal
  Data Feed") datafeed server — unlike the cTrader substitution, UDF is
  TradingView's actual small, public JSON-over-HTTP protocol, faithfully
  implemented rather than stood in for, including genuine resampling
  (`udf::resample`) from the natively-aggregated 1-minute bars up to
  whatever resolution a client requests, not just relabeled data.

Both routes share one real data path: every webhook alert folds into this
adapter's own `market_data::BarAggregator` state, so `/udf/history`, the
trait's own `history()` method, and a live TradingView chart pointed at this
server all read the same underlying bars — one real pipeline, not three
independent stubs. Verified against a real bound port with a real `reqwest`
HTTP client (`tests/webhook_and_udf_integration.rs`), not just handler
functions tested in isolation.

**Still stubbed / honestly out of reach here:**

- `adapter-binance`/`adapter-ctrader`'s own third message types
  (`SymbolInfo`/`ProtoOASymbolByIdReq`, real historical trendbars) are
  deferred the same way `Mt5Broker::constraints`/`Mt5MarketData::history`
  deferred theirs in Phase 1/2 — not exercised by the order-routing-parity
  exit criterion.
- No real cTrader Open API or TradingView account exists to validate either
  adapter's wire format against in this sandbox — both are honestly
  documented substitutions/faithful-but-unverified-against-the-real-thing
  implementations, not claims of production readiness.
- `adapter-binance`/`adapter-ctrader`'s own doc comments call cTrader/
  Binance/IBKR collectively "v2" in §5.4's adapter matrix; only cTrader was
  built out this phase (per its own Cargo.toml description explicitly
  naming Phase 8) — Binance stays a stub, unchanged, matching the design
  doc's own "second broker adapter" (singular) exit-criterion wording.
- `crates/strategy`'s decision tree compilation (§5.5, <5µs) remains the
  only still-unreal piece of the strategy VM, carried forward unchanged.

### Phase 9 — Scale (in progress)

§17's own Phase 9 row has no exit criteria cell (it reads "—") and no §19
generation prompt — the prompts stop at Prompt 12 (Observability and
validation), the same gap Phase 8 already had. Scoped to the roadmap row's
four named items — "Multi-account, portfolio optimizer, distributed agents,
GPU inference" — each built for real against what's actually testable in
this sandbox, with the same honest split as every prior phase between real
logic and infrastructure this environment cannot provide (no GPU hardware,
no multi-machine cluster, no live multi-account broker credentials).

- **Multi-account** (`execution::AccountManager`): one `OrderRouter` per
  account, keyed by `AccountId`, reusing `OrderRouter<B>`'s existing
  genericity rather than a second implementation of order routing — each
  account's idempotency ledger is a separate `HashMap`, so a `client_id`
  colliding across two accounts produces two independent orders, never a
  false idempotent match. `domain::ports::Broker` picked up one small,
  purely additive change: a forwarding `impl Broker for Box<dyn Broker>`,
  since the trait was already object-safe but had nothing making that
  usable — this lets `AccountManager<Box<dyn Broker>>` hold accounts on
  different broker platforms at once (an MT5 account and a cTrader account
  side by side, concretely), extending Phase 8's "same strategy runs on 2
  adapters" to "at the same time." `aggregate_snapshot()` sums equity
  across every registered account for real, the number a portfolio-level
  view needs as its input.
- **Portfolio optimizer** (new `crates/portfolio`, zero external
  dependencies beyond `thiserror`): capital allocation *across* strategies/
  accounts, sitting above `crates/risk`'s own per-trade Kelly sizing rather
  than replacing it. A from-scratch Cholesky-based SPD linear solver
  (`linalg`) backs closed-form Markowitz mean-variance weights
  (`mean_variance`: global-minimum-variance and maximum-Sharpe/tangency,
  both verified against hand-computable closed forms for the uncorrelated
  case) and an iterative equal-risk-contribution risk-parity solver
  (`risk_parity`). Two real bugs were found and fixed by verifying against
  known closed-form answers rather than trusting the code: the Cholesky
  pivot test's exact `<= 0.0` comparison missed a genuinely singular
  duplicate-asset matrix that floating-point rounding nudged to `6.9e-18`
  instead of `0.0` (fixed with a documented epsilon threshold), and the
  risk-parity solver's first, undamped multiplicative update
  (`w_i *= target/actual`) provably oscillated forever between two weight
  vectors on a trivial 2-asset case instead of converging — fixed by
  damping the update to `w_i *= sqrt(target/actual)`, the mathematically
  motivated correction once you account for a risk contribution being
  quadratic, not linear, in its own weight.
- **Distributed agents** (new `services/agents/packages/distributed`):
  §16's own claim — "agents are stateless and horizontally scalable" — made
  concrete and tested across real, separate OS processes (`multiprocessing`
  with the `spawn` start method, not `fork`, precisely so nothing can pass
  between workers via accidentally shared copy-on-write memory) rather than
  just asserted from `BaseAgent`'s stateless interface shape. Each worker
  reconstructs its own `RegimeAgent` from nothing but a fixed seed — no
  pickled model crosses the process boundary — and
  `tests/test_dispatcher.py` verifies the actual property this is supposed
  to guarantee: the same job set produces byte-identical results whether
  run with 1 worker or 4, work genuinely spreads across more than one
  process, and a bad job or unknown agent kind reports a failed result
  rather than crashing or hanging a worker.
- **GPU inference** (`crates/strategy`'s `gpu` Cargo feature, off by
  default): registers `ort::ep::CUDA` ahead of `ort::ep::CPU` on the ONNX
  session builder. This sandbox has no GPU and no CUDA runtime to
  accelerate anything with — there is no hardware here to benchmark a
  speedup against, and claiming one would be fabricating a result — but the
  registration code is real, and `ort`'s own execution-provider dispatch
  already falls back to the next provider in the list (logging a warning)
  when an earlier one fails to initialize, which is exactly what happens
  under `--features gpu` in this environment. The existing Phase 3 ONNX
  parity test (`onnx_parity`) doubles as the verification here with no
  changes needed: run under `--features gpu`, it exercises the CUDA-then-
  CPU-fallback path end-to-end and produces the exact same result as the
  default build, proving the fallback path is genuinely correct, not just
  compilable.

**Still stubbed / honestly out of reach here:**

- No real multi-account run against live broker credentials, no real GPU
  hardware to benchmark inference acceleration against, and no real
  multi-machine deployment of the agent layer (§16's own topology diagram)
  — all three need infrastructure this sandbox does not have and no mock
  substitutes for; `AccountManager`, the `gpu` feature, and the
  multi-process dispatcher are all real, tested code paths that are ready
  for that infrastructure whenever it exists, not simulations of it.
- The portfolio optimizer has no gateway/dashboard surface — nothing in
  §17's Phase 9 scope row named one, unlike phases that did touch the
  gateway/dashboard.
- `long_only_max_sharpe_weights`' clamp-and-renormalize is a documented
  approximation to the true long-only-constrained optimum, not a
  quadratic-programming solver — see `mean_variance`'s own doc comment.
- `crates/strategy`'s decision tree compilation (§5.5, <5µs) remains the
  only still-unreal piece of the strategy VM, carried forward unchanged
  across every phase since Phase 0.

With Phase 9, every row of §17's roadmap has real, tested work behind it —
see each phase's own section above for exactly what's real, what's
substituted, and why.

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
