/// Interned symbol identifier. Strings never appear on the hot path (§5.1) —
/// the interning table (name <-> id) lives in the L0 in-process cache
/// (`crates/market-data`), not here, since this crate has no I/O.
pub type SymbolId = u16;
