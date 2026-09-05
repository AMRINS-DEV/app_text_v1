use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::enums::{OrderType, Side, TimeInForce, TradingMode};
use crate::ids::SymbolId;

/// The only message that authorizes an order. Produced exclusively by
/// `crates/risk` after the expectancy gate (§8.5) passes — agents and the
/// strategy VM cannot construct one that reaches the broker directly.
///
/// rkyv zero-copy in addition to serde: this crosses the MT5 bridge wire on
/// the hot path (§5.2 "Order encode + send to bridge: 5-20 µs"), same as `Tick`.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct OrderIntent {
    /// Idempotency key (ULID). The execution router must treat resubmission
    /// with the same `client_id` as a no-op, not a duplicate order.
    pub client_id: u128,
    pub symbol_id: SymbolId,
    pub side: Side,
    /// Fixed-point lots.
    pub qty: i64,
    pub order_type: OrderType,
    pub limit_px: Option<i64>,
    /// Set atomically with entry — never as a follow-up modify (§9.3).
    pub sl: Option<i64>,
    pub tp: Option<i64>,
    pub tif: TimeInForce,
    pub mode: TradingMode,
    pub max_slippage_pts: u32,
    /// Full attribution chain: every `Signal.id` that contributed to this
    /// order, for the trade replay bundle (§14).
    pub signal_ids: SmallVec<[u128; 4]>,
}

pub type BrokerOrderId = u64;

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ExecEvent {
    Fill { client_id: u128, broker_order_id: BrokerOrderId, fill_price: i64, qty: i64, ts_ns: u64 },
    Reject { client_id: u128, reason: String, ts_ns: u64 },
    Modify { broker_order_id: BrokerOrderId, sl: Option<i64>, tp: Option<i64>, ts_ns: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let o = OrderIntent {
            client_id: 1,
            symbol_id: 1,
            side: Side::Buy,
            qty: 100,
            order_type: OrderType::Market,
            limit_px: None,
            sl: Some(99_000),
            tp: Some(101_000),
            tif: TimeInForce::Gtc,
            mode: TradingMode::Normal,
            max_slippage_pts: 5,
            signal_ids: SmallVec::from_slice(&[7, 9]),
        };
        let json = serde_json::to_string(&o).unwrap();
        let back: OrderIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(o, back);
    }

    #[test]
    fn rkyv_zero_copy_roundtrip() {
        let o = OrderIntent {
            client_id: 42,
            symbol_id: 1,
            side: Side::Sell,
            qty: 250,
            order_type: OrderType::Limit,
            limit_px: Some(100_000),
            sl: Some(99_000),
            tp: Some(101_000),
            tif: TimeInForce::Ioc,
            mode: TradingMode::Aggressive,
            max_slippage_pts: 3,
            signal_ids: SmallVec::from_slice(&[7, 9]),
        };
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&o).unwrap();
        let archived = rkyv::access::<ArchivedOrderIntent, rkyv::rancor::Error>(&bytes).unwrap();
        assert_eq!(archived.client_id, 42);
        assert_eq!(archived.qty, 250);
        let side: Side = rkyv::deserialize::<_, rkyv::rancor::Error>(&archived.side).unwrap();
        assert_eq!(side, Side::Sell);
    }
}
