//+------------------------------------------------------------------+
//| Wire protocol structs mirroring crates/domain/src/tick.rs::Tick  |
//| and order.rs::OrderIntent byte-for-byte (§5.3-5.4). See          |
//| docs/protocol.md for the authoritative field layout, versioning  |
//| and heartbeat scheme. Phase 1 scope: fixed-point prices,         |
//| monotonic sequence numbers with gap detection.                   |
//+------------------------------------------------------------------+
#property strict

// #pragma pack semantics differ in MQL5 — Phase 1 implementation must
// verify each struct's byte layout against crates/domain's #[repr(C)]
// types with an integration test that decodes a captured frame in Rust.
struct WireTick
{
   ulong  ts_ns;
   ulong  recv_ns;
   ushort symbol_id;
   long   bid;
   long   ask;
   uint   bid_volume;
   uint   ask_volume;
   ushort flags;
};
