# Signal Contract (§8, §10.3)

The `Signal` type is the only channel through which an LLM agent, a trained
model, or a deterministic rule can influence a trade — and even then, only
as advisory input to fusion (§8.4), never as an order. Three representations
of the same contract must stay in sync:

| Representation | File | Boundary |
|---|---|---|
| Rust (in-process) | `crates/domain/src/signal.rs` | strategy VM, fusion, core |
| Protobuf (wire) | `packages/proto/signal.proto` | NATS JetStream, gRPC |
| Zod (TS) | `packages/schemas/src/signal.ts` | gateway, dashboard |
| Pydantic (Python) | `services/agents/packages/core/src/agents_core/agent.py::AgentOutput` | agent layer |

## Fields (see `crates/domain/src/signal.rs` for authoritative doc comments)

`id`, `source`, `symbol_id`, `direction`, `probability` (calibrated,
**never** a raw model score — §8.3), `confidence`, `expected_r`,
`horizon_ms`, `ttl_ns`, `regime`, `features_hash`, `evidence_ref`.

## Validation the core performs before fusion (§10.3)

1. Schema + TTL validation — an expired signal is discarded unconditionally
   (P6: fail closed).
2. `features_hash` must match a known feature snapshot — prevents
   stale/hallucinated context from a delayed or replaying agent.
3. Probability must be inside the emitting agent's calibrated range.
4. Feeds into fusion (§8.4) — **never** treated as a decision by itself.

## What's implemented vs. not (Phase 0 status)

- The Rust, protobuf and zod representations exist and match field-for-field.
- The sample-size gate and naive weighted-average fusion in
  `crates/strategy/src/fusion.rs` are real but a placeholder for the actual
  log-odds pooling formula (§8.4) — the online Brier-score weight tracker
  and empirical correlation matrix (`rho_ij`) are Phase 5/6 scope.
- Validation steps 1-4 above are not wired into any running process yet —
  they need the NATS bus (`crates/bus`, Phase 1) and a live feature snapshot
  source (`crates/features`, Phase 3) to be meaningful.
