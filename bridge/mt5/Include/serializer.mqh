//+------------------------------------------------------------------+
//| Encodes WireTick (protocol.mqh) to the wire format documented in |
//| docs/protocol.md. Phase 1 scope, unverified (no MT5 terminal in  |
//| this environment to compile/test against).                       |
//|                                                                    |
//| MQL5 has no built-in little-endian byte-array packer for mixed    |
//| struct fields, so this cannot be a `memcpy`-equivalent even if     |
//| the struct's in-memory layout happens to match WireTick's          |
//| documented offsets -- it must be written field-by-field into a     |
//| uchar[] using explicit shifts (e.g. a `PackUInt64LE(buf, offset,   |
//| value)` helper per field), the same way `dump_tick_layout.rs`      |
//| shows the Rust side actually laying bytes out. That helper set is  |
//| not written here; writing it without a compiler to check against  |
//| would just be guessing at MQL5 syntax, which is worse than an      |
//| honest gap.                                                        |
//+------------------------------------------------------------------+
#property strict
