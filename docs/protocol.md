# Bridge Wire Protocol (§5.4)

Status: **Phase 1 scope, not yet implemented.** This document fixes the
contract now so `bridge/mt5` and `crates/adapters/mt5` can be built against
the same spec independently.

## Transport

- **Ticks/DOM**: MT5 EA -> Rust core, ZeroMQ **PUB/SUB**, loopback TCP (or a
  shared-memory transport later if ZMQ loopback's ~20-30µs isn't tight
  enough against the §5.2 budget).
- **Orders**: Rust core -> MT5 EA, ZeroMQ **REQ/REP**. `OrderSend`/Modify/
  Close each get a request/response round trip carrying an idempotency key
  (`OrderIntent.client_id`).

## Frame layout

Every frame is the `#[repr(C)]` byte layout of the corresponding
`crates/domain` type (`Tick`, `OrderIntent`) — see `crates/domain/src/tick.rs`
and `order.rs` for the authoritative field list. `bridge/mt5/Include/protocol.mqh`
mirrors these structs on the MQL5 side; the two must be verified
byte-identical with a captured-frame integration test before Phase 1 exits
(§17: "Bridge + Core... 24h replay is bit-identical").

## Versioning

Reserved: a `protocol_version` byte in the frame header, incremented on any
field change. Not yet implemented — Phase 1.

## Heartbeats and reconnect

The EA must emit a heartbeat frame at a fixed interval; the Rust adapter
treats a missed heartbeat as `feed_gap_seconds` (§14 SLO, target p99 < 1s)
and, on reconnect, must resynchronize sequence numbers rather than silently
resuming (P6: fail closed, never "assume last known good"). Not yet
implemented — Phase 1.

## Sequence numbers and gap detection

Every tick carries a monotonic per-symbol sequence number. A gap sets
`Tick.flags`'s `GAP` bit (`crates/domain/src/tick.rs::TickFlags`) so the
feature engine can decide whether to trust the bar it's building. Not yet
implemented — Phase 1.
