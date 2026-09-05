//! Wire protocol (§5.4, `docs/protocol.md`). Frames are the `#[repr(C)]`
//! rkyv byte layout of the corresponding `domain` type, prefixed with a
//! monotonic sequence number for gap detection — no serde, no allocation
//! beyond the frame buffer itself, matching the hot-path serialization
//! choice in §3.2.
//!
//! This module is the single source of truth for the frame format; both
//! `Mt5MarketData`/`Mt5Broker` (the real adapter) and `mock-mt5-bridge`
//! (the test double standing in for the MQL5 EA) encode/decode through it,
//! so they can never drift from each other.

use domain::{BrokerOrderId, OrderIntent, Tick};
use rkyv::util::AlignedVec;
use rkyv::{Archive, Deserialize, Serialize};

/// rkyv requires its input buffer to satisfy the archived type's alignment
/// (typically 8 bytes for these types) — a raw slice off the wire (or a
/// sub-slice past a framing header) is never guaranteed to satisfy that, so
/// every decode path copies into a freshly allocated, correctly-aligned
/// buffer first. This is one extra copy per frame, not a hot-path concern
/// at Phase 1's scale; if it ever needs to be zero-copy, the fix is to keep
/// the payload as its own ZMQ multipart frame (so it's never sliced out of
/// a larger buffer) rather than removing this copy.
fn to_aligned(bytes: &[u8]) -> AlignedVec {
    let mut aligned = AlignedVec::new();
    aligned.extend_from_slice(bytes);
    aligned
}

pub const HEARTBEAT_INTERVAL_MS: u64 = 250;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("frame too short: {0} bytes")]
    TooShort(usize),
    #[error("unknown frame kind byte: {0}")]
    UnknownKind(u8),
    #[error("rkyv decode failed: {0}")]
    Rkyv(String),
}

// --- Market data frames (PUB/SUB) ---------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum MarketDataFrame {
    Tick { seq: u64, tick: Tick },
    Heartbeat { seq: u64 },
}

const KIND_TICK: u8 = 0;
const KIND_HEARTBEAT: u8 = 1;

/// `[seq: u64 LE][kind: u8][payload...]`
pub fn encode_tick_frame(seq: u64, tick: &Tick) -> Vec<u8> {
    let payload = rkyv::to_bytes::<rkyv::rancor::Error>(tick).expect("Tick archiving is infallible");
    let mut out = Vec::with_capacity(9 + payload.len());
    out.extend_from_slice(&seq.to_le_bytes());
    out.push(KIND_TICK);
    out.extend_from_slice(&payload);
    out
}

pub fn encode_heartbeat_frame(seq: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(9);
    out.extend_from_slice(&seq.to_le_bytes());
    out.push(KIND_HEARTBEAT);
    out
}

pub fn decode_market_data_frame(bytes: &[u8]) -> Result<MarketDataFrame, ProtocolError> {
    if bytes.len() < 9 {
        return Err(ProtocolError::TooShort(bytes.len()));
    }
    let seq = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
    match bytes[8] {
        KIND_TICK => {
            let aligned = to_aligned(&bytes[9..]);
            let archived = rkyv::access::<domain::ArchivedTick, rkyv::rancor::Error>(&aligned)
                .map_err(|e| ProtocolError::Rkyv(e.to_string()))?;
            let tick: Tick =
                rkyv::deserialize::<_, rkyv::rancor::Error>(archived).map_err(|e| ProtocolError::Rkyv(e.to_string()))?;
            Ok(MarketDataFrame::Tick { seq, tick })
        }
        KIND_HEARTBEAT => Ok(MarketDataFrame::Heartbeat { seq }),
        other => Err(ProtocolError::UnknownKind(other)),
    }
}

// --- Order frames (REQ/REP) ---------------------------------------------

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum OrderRequest {
    Submit(OrderIntent),
    Modify { broker_order_id: BrokerOrderId, sl: Option<i64>, tp: Option<i64> },
    Close { broker_order_id: BrokerOrderId, qty: Option<i64> },
}

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum OrderReply {
    Accepted { broker_order_id: BrokerOrderId },
    Modified,
    Closed,
    Rejected { reason: String },
}

pub fn encode_order_request(req: &OrderRequest) -> Vec<u8> {
    rkyv::to_bytes::<rkyv::rancor::Error>(req).expect("OrderRequest archiving is infallible").into_vec()
}

pub fn decode_order_request(bytes: &[u8]) -> Result<OrderRequest, ProtocolError> {
    let aligned = to_aligned(bytes);
    let archived = rkyv::access::<ArchivedOrderRequest, rkyv::rancor::Error>(&aligned)
        .map_err(|e| ProtocolError::Rkyv(e.to_string()))?;
    rkyv::deserialize::<_, rkyv::rancor::Error>(archived).map_err(|e| ProtocolError::Rkyv(e.to_string()))
}

pub fn encode_order_reply(reply: &OrderReply) -> Vec<u8> {
    rkyv::to_bytes::<rkyv::rancor::Error>(reply).expect("OrderReply archiving is infallible").into_vec()
}

pub fn decode_order_reply(bytes: &[u8]) -> Result<OrderReply, ProtocolError> {
    let aligned = to_aligned(bytes);
    let archived = rkyv::access::<ArchivedOrderReply, rkyv::rancor::Error>(&aligned)
        .map_err(|e| ProtocolError::Rkyv(e.to_string()))?;
    rkyv::deserialize::<_, rkyv::rancor::Error>(archived).map_err(|e| ProtocolError::Rkyv(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{OrderType, Side, TimeInForce, TradingMode};
    use smallvec::SmallVec;

    fn sample_tick(seq_hint: i64) -> Tick {
        Tick { ts_ns: 1_000 + seq_hint as u64, recv_ns: 1_001, symbol_id: 7, bid: 100_000, ask: 100_010, bid_volume: 1, ask_volume: 2, flags: 0 }
    }

    #[test]
    fn tick_frame_roundtrip() {
        let tick = sample_tick(5);
        let frame = encode_tick_frame(42, &tick);
        match decode_market_data_frame(&frame).unwrap() {
            MarketDataFrame::Tick { seq, tick: decoded } => {
                assert_eq!(seq, 42);
                assert_eq!(decoded, tick);
            }
            other => panic!("expected Tick frame, got {other:?}"),
        }
    }

    #[test]
    fn heartbeat_frame_roundtrip() {
        let frame = encode_heartbeat_frame(7);
        match decode_market_data_frame(&frame).unwrap() {
            MarketDataFrame::Heartbeat { seq } => assert_eq!(seq, 7),
            other => panic!("expected Heartbeat frame, got {other:?}"),
        }
    }

    #[test]
    fn too_short_frame_is_rejected() {
        assert!(matches!(decode_market_data_frame(&[1, 2, 3]), Err(ProtocolError::TooShort(3))));
    }

    #[test]
    fn unknown_kind_byte_is_rejected() {
        let mut bytes = 0u64.to_le_bytes().to_vec();
        bytes.push(0xFF);
        assert!(matches!(decode_market_data_frame(&bytes), Err(ProtocolError::UnknownKind(0xFF))));
    }

    #[test]
    fn order_request_roundtrip() {
        let req = OrderRequest::Submit(OrderIntent {
            client_id: 1,
            symbol_id: 7,
            side: Side::Buy,
            qty: 100,
            order_type: OrderType::Market,
            limit_px: None,
            sl: Some(99_000),
            tp: Some(101_000),
            tif: TimeInForce::Gtc,
            mode: TradingMode::Normal,
            max_slippage_pts: 5,
            signal_ids: SmallVec::from_slice(&[9]),
        });
        let bytes = encode_order_request(&req);
        let decoded = decode_order_request(&bytes).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn order_reply_roundtrip() {
        let reply = OrderReply::Rejected { reason: "spread too wide".into() };
        let bytes = encode_order_reply(&reply);
        assert_eq!(decode_order_reply(&bytes).unwrap(), reply);
    }
}
