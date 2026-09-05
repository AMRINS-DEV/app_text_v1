# Bridge Wire Protocol (§5.4)

Status: **implemented and tested** on the Rust side
(`crates/adapters/mt5/src/protocol.rs`, exercised end-to-end in
`crates/adapters/mt5/tests/bridge_integration.rs` against
`crates/bin/mock-mt5-bridge`). The MQL5 EA side (`bridge/mt5/Experts/TradeOSBridge.mq5`)
implements the same format but **cannot be compiled or tested** outside a
real MT5 terminal, which this environment does not have — treat it as
unverified until it's run against a live terminal.

## Transport

- **Ticks**: MT5 EA -> Rust core, ZeroMQ **PUB/SUB**, TCP (loopback in
  practice; the wire format doesn't care).
- **Orders**: Rust core -> MT5 EA, ZeroMQ **REQ/REP**. `OrderSend`/Modify/
  Close each get a request/response round trip.

The Rust side uses the pure-Rust [`zeromq`](https://docs.rs/zeromq) crate
(ZMTP 3.0-compatible), not a `libzmq` binding — this matters not at all to
the EA side, which only ever sees ZMTP-conformant peers on the wire.

## Market data frame (PUB/SUB)

```
[ seq: u64 LE ][ kind: u8 ][ payload ]
```

- `seq`: monotonic per-stream sequence number. The receiver tracks the
  next-expected value; a gap sets the tick's `TickFlags::GAP` bit
  (`crates/domain/src/tick.rs`) rather than raising an error — a missed
  tick degrades a feature snapshot, it doesn't halt the feed (see the
  hot-path "no `Result` unwinding" rule, §5.1).
- `kind`: `0` = Tick, `1` = Heartbeat.
- `payload` (kind `0` only): the rkyv-archived byte layout of
  `domain::Tick` — zero-copy on the receive side once decoded into an
  aligned buffer (rkyv requires the archive's own alignment, which a raw
  sub-slice of a framed buffer does not satisfy — see
  `protocol::to_aligned`'s doc comment for why every decode path copies
  into a fresh aligned buffer before `rkyv::access`; this was found by the
  bridge integration tests, not assumed).

  The archived layout is **56 bytes**, matching `Tick`'s plain
  `#[repr(C)]` layout with natural alignment (verified empirically —
  `cargo run -p domain --example dump_tick_layout` — not just read off
  rkyv's docs, since archived-format guarantees can be subtle):

  ```
  ts_ns       @  0..8    (u64 LE)
  recv_ns     @  8..16   (u64 LE)
  symbol_id   @ 16..18   (u16 LE)
  (padding)   @ 18..24   -- bid needs 8-byte alignment
  bid         @ 24..32   (i64 LE)
  ask         @ 32..40   (i64 LE)
  bid_volume  @ 40..44   (u32 LE)
  ask_volume  @ 44..48   (u32 LE)
  flags       @ 48..50   (u16 LE)
  (padding)   @ 50..56   -- struct size rounds up to 8-byte alignment
  ```

  A from-scratch MQL5 encoder must reproduce this exact byte pattern,
  padding included — this is precisely the kind of detail `bridge/mt5`
  cannot get right by inspection alone without a real MT5 terminal to test
  against (see that directory's own caveat).
- Heartbeat frames (kind `1`) carry no payload — just the 9-byte header.
  Sent every `HEARTBEAT_INTERVAL_MS` (250ms) worth of ticks by convention;
  the mock bridge's `ticks_per_heartbeat` config controls the actual cadence.

## Order frames (REQ/REP)

Both request and reply are rkyv-archived Rust enums
(`adapter_mt5::protocol::{OrderRequest, OrderReply}`), sent as a single
ZMQ message part each.

```rust
enum OrderRequest {
    Submit(OrderIntent),               // full domain::OrderIntent
    Modify { broker_order_id: u64, sl: Option<i64>, tp: Option<i64> },
    Close { broker_order_id: u64, qty: Option<i64> },
}

enum OrderReply {
    Accepted { broker_order_id: u64 },
    Modified,
    Closed,
    Rejected { reason: String },
}
```

**Important correction, found while writing `dump_order_layout.rs`** (the
`OrderRequest`/`OrderReply` analogue of `dump_tick_layout.rs`): unlike
`Tick`, these types are *not* plain fixed-layout structs once archived.
`OrderIntent` contains `Option<i64>`, a `SmallVec`, and (transitively via
`OrderReply::Rejected`) a `String` — rkyv represents all of these with
**relative pointers** into the same buffer (an `Option`'s discriminant is
encoded as a sentinel/niche value, and the `SmallVec`/`String` payloads
live at buffer offsets computed at serialization time and referenced by
relative offset, not inline). Dumping `OrderRequest::Submit(...)` for a
144-byte archive makes this checkable: the bytes are not the field values
in declaration order the way `Tick`'s are.

**Consequence:** this rkyv encoding is only practical between two Rust
peers (`adapter-mt5` <-> `mock-mt5-bridge`, which is exactly what
`bridge_integration.rs` tests — genuinely, not by assumption). A **real
MQL5 EA cannot feasibly decode it** — reimplementing rkyv's relative-
pointer archive format by hand in MQL5 is not a reasonable ask. Bridging
to an actual MT5 terminal needs one of:

1. A flat, fixed-offset struct for the order path specifically (mirroring
   the Tick frame's approach: primitives only, no `Option`/`String`/
   `SmallVec` — e.g. a sentinel value in place of `Option`, a fixed-size
   byte array in place of `String`), with the Rust adapter translating
   to/from `OrderIntent` at the boundary, or
2. A thin non-Rust rkyv-compatible decoder (impractical — out of scope).

Option 1 is real, scoped work for whoever picks up a live MT5 connection —
it is **not implemented here**. The tick path needs no such change; it
already produces a flat layout for free.

## Versioning

Not yet implemented. A `protocol_version` byte belongs in the header once
there's a second version to distinguish — premature now with exactly one
implementation of each side.

## Heartbeats and reconnect

Heartbeat frames update `Mt5MarketData::last_heartbeat_seq` (Rust side).
Turning a stale heartbeat into the §14 `feed_gap_seconds` SLO / a §9.5
"data staleness" guard trip is Phase 2 scope — it needs the guard
framework in `crates/risk::guards` to be wired up, not just the raw
counter this crate exposes today.

Reconnect-with-resync is not yet implemented: `Mt5MarketData::subscribe`
connects once: on error, the read is inert. This is a real gap. Phase 2 must
implement retry, but is only meaningful against a *real* EA that can
actually disconnect/reconnect - the mock bridge's own process lifetime is
the test's, so there is nothing to reconnect to in this environment.

## Order path latency

Measured (see `crates/adapters/mt5/tests/bridge_integration.rs`, pure-Rust
ZMQ over TCP loopback, mock bridge as the peer, this environment):

| Percentile | Measured | §5.2 budget |
|---|---:|---:|
| p50 | ~36 µs | — |
| p99 | ~113 µs | < 50 µs |
| p999 | ~189 µs | — |

p99 is above the design doc's budget. This is expected, not a regression
to chase down: §5.2's number assumes native `libzmq` or a shared-memory
transport (`iceoryx2`/`rtrb`), neither of which this crate uses yet — the
pure-Rust `zeromq` crate was chosen for Phase 1 specifically because it
needs no system `libzmq` dependency to build in CI/sandboxes. Closing this
gap for real is a `libzmq`-backed adapter or the shared-memory path, not a
tuning exercise on the current one.
