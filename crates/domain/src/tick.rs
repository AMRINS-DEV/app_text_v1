use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

bitflags::bitflags! {
    /// Tick condition flags, packed to avoid branching on the hot path.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct TickFlags: u16 {
        const GAP          = 0b0000_0001;
        const STALE        = 0b0000_0010;
        const SESSION_OPEN = 0b0000_0100;
        const WIDE_SPREAD  = 0b0000_1000;
    }
}

/// One market tick. `#[repr(C)]` + rkyv zero-copy (de)serialization because
/// this is the type that crosses the ingest -> feature SPSC ring buffer on
/// the hot path (§5.1-5.3): no serde overhead is acceptable here.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Tick {
    /// Broker timestamp, nanoseconds.
    pub ts_ns: u64,
    /// Local receive timestamp, for latency telemetry.
    pub recv_ns: u64,
    /// Interned symbol id — never a string on the hot path.
    pub symbol_id: crate::ids::SymbolId,
    /// Fixed-point price, scaled by 10^price_digits.
    pub bid: i64,
    pub ask: i64,
    pub bid_volume: u32,
    pub ask_volume: u32,
    pub flags: u16,
}

impl Tick {
    #[inline]
    pub fn spread(&self) -> i64 {
        self.ask - self.bid
    }

    #[inline]
    pub fn mid(&self) -> i64 {
        (self.bid + self.ask) / 2
    }

    #[inline]
    pub fn flags(&self) -> TickFlags {
        TickFlags::from_bits_truncate(self.flags)
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Bar {
    pub symbol_id: crate::ids::SymbolId,
    pub timeframe_seconds: u32,
    pub ts_open_ns: u64,
    pub open: i64,
    pub high: i64,
    pub low: i64,
    pub close: i64,
    pub volume: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_spread_and_mid() {
        let t = Tick { ts_ns: 0, recv_ns: 0, symbol_id: 1, bid: 100_000, ask: 100_010, bid_volume: 1, ask_volume: 1, flags: 0 };
        assert_eq!(t.spread(), 10);
        assert_eq!(t.mid(), 100_005);
    }

    #[test]
    fn tick_flags_roundtrip() {
        let flags = TickFlags::GAP | TickFlags::WIDE_SPREAD;
        let t = Tick { ts_ns: 0, recv_ns: 0, symbol_id: 1, bid: 1, ask: 2, bid_volume: 0, ask_volume: 0, flags: flags.bits() };
        assert!(t.flags().contains(TickFlags::GAP));
        assert!(t.flags().contains(TickFlags::WIDE_SPREAD));
        assert!(!t.flags().contains(TickFlags::STALE));
    }

    #[test]
    fn tick_rkyv_zero_copy_roundtrip() {
        let t = Tick { ts_ns: 42, recv_ns: 43, symbol_id: 7, bid: 100, ask: 101, bid_volume: 5, ask_volume: 6, flags: 0 };
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&t).unwrap();
        let archived = rkyv::access::<ArchivedTick, rkyv::rancor::Error>(&bytes).unwrap();
        assert_eq!(archived.ts_ns, 42);
        assert_eq!(archived.symbol_id, 7);
    }
}
