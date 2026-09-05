//+------------------------------------------------------------------+
//| Wire protocol structs mirroring crates/domain/src/tick.rs::Tick, |
//| byte-for-byte, per docs/protocol.md. The exact layout below was  |
//| verified empirically from the Rust side (see                     |
//| crates/domain/examples/dump_tick_layout.rs), not assumed:         |
//|                                                                    |
//|   ts_ns       @  0..8   (u64 LE)                                  |
//|   recv_ns     @  8..16  (u64 LE)                                  |
//|   symbol_id   @ 16..18  (u16 LE)                                  |
//|   (padding)   @ 18..24  -- bid needs 8-byte alignment              |
//|   bid         @ 24..32  (i64 LE)                                  |
//|   ask         @ 32..40  (i64 LE)                                  |
//|   bid_volume  @ 40..44  (u32 LE)                                  |
//|   ask_volume  @ 44..48  (u32 LE)                                  |
//|   flags       @ 48..50  (u16 LE)                                  |
//|   (padding)   @ 50..56  -- struct rounds up to 8-byte alignment    |
//|                                                                    |
//| The full market-data PUB frame is: [seq: u64 LE][kind: u8][the    |
//| 56 bytes above] for a tick frame, or just [seq][kind] (9 bytes,    |
//| kind=1) for a heartbeat.                                          |
//|                                                                    |
//| IMPORTANT — order path is NOT implemented here. The Rust side's   |
//| OrderRequest/OrderReply (adapter-mt5/src/protocol.rs) are rkyv-   |
//| archived enums whose Option/String/SmallVec fields use relative   |
//| pointers into the archive buffer — a format only a Rust peer can  |
//| decode. Wiring a real EA to the order path needs a *separate*,    |
//| flat fixed-offset encoding (see docs/protocol.md's "Important     |
//| correction"), which is unimplemented on both sides. This file     |
//| covers ticks only; a WireOrderRequest/WireOrderReply pair is      |
//| future work, not a placeholder that's merely unfilled here.       |
//+------------------------------------------------------------------+
#property strict

struct WireTick
{
   ulong  ts_ns;       // @ 0
   ulong  recv_ns;     // @ 8
   ushort symbol_id;   // @ 16
   // 6 bytes padding here to reach the 24-byte offset -- do not add a
   // field for it; MQL5 struct layout follows the same natural-alignment
   // rule as C, so this struct's own memory layout already matches, IF
   // (and only if) the compiler lays out these fields with no packing
   // pragma in effect. This has not been verified against a real MQL5
   // compiler in this environment -- treat as unverified (see README.md).
   long   bid;         // @ 24
   long   ask;         // @ 32
   uint   bid_volume;  // @ 40
   uint   ask_volume;  // @ 44
   ushort flags;       // @ 48
   // 6 bytes trailing padding to 56 bytes total.
};

// Header for every PUB frame: 8-byte LE sequence number + 1-byte kind
// (0 = Tick, 1 = Heartbeat). Phase 1 scope, unimplemented in this file --
// see serializer.mqh for where the encode/decode functions belong.
#define FRAME_KIND_TICK      0
#define FRAME_KIND_HEARTBEAT 1
