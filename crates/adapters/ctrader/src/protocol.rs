//! Wire protocol for `adapter-ctrader` (§17 Phase 8: "second broker
//! adapter... same strategy runs on 2 adapters with no core change").
//!
//! A real cTrader account speaks the Open API: Protobuf messages framed by
//! a 4-byte length prefix over a TLS-wrapped TCP socket, using a specific
//! schema (`ProtoOAApplicationAuthReq`, `ProtoOANewOrderReq`, ...) that this
//! sandbox has no way to test against — no network access, no cTrader
//! account, and reproducing the exact Protobuf field numbers from memory
//! with no way to verify them against a real server would be *pretending*
//! to be real rather than being real (the same standard applied when
//! Phase 7 rejected NautilusTrader and Phase 6 rejected a literal
//! FalkorDB). What's genuinely real here instead: length-prefixed framing
//! (matching the actual shape of Open API's own transport, including a
//! `req_id` correlation field mirroring Open API's real `clientMsgId`
//! mechanism for matching replies — and unsolicited server-initiated
//! frames, like a fill notification arriving after an accepted order — to
//! the request that caused them) carrying `serde_json` payloads of this
//! project's own domain types (`OrderIntent`, `ExecEvent`, `Tick`).
//!
//! What this crate exists to prove is §17's own exit criterion — the trait
//! boundary in `domain::ports` is enough to add a platform with zero core
//! changes — not byte-for-byte Protobuf fidelity to one specific broker's
//! real wire format. This is also why the transport stack deliberately
//! differs from `adapter-mt5` (blocking `std::net::TcpStream` + JSON here,
//! vs. async ZMQ + rkyv there): two adapters that happen to share
//! implementation code would prove nothing about the trait boundary being
//! the only real requirement.

use std::io::{self, Read, Write};

use domain::{ExecEvent, OrderIntent, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("json decode failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("frame exceeds max size ({0} bytes)")]
    FrameTooLarge(usize),
}

/// Generous ceiling against a corrupted/malicious length prefix; every
/// message this protocol actually carries is a few hundred bytes at most.
const MAX_FRAME_BYTES: usize = 1 << 20;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ClientMessage {
    Subscribe { symbols: Vec<u16> },
    NewOrder(OrderIntent),
    AmendOrder { broker_order_id: u64, sl: Option<i64>, tp: Option<i64> },
    ClosePosition { broker_order_id: u64, qty: Option<i64> },
    PositionsRequest,
    AccountRequest,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct WirePosition {
    pub broker_order_id: u64,
    pub symbol_id: u16,
    pub qty: i64,
    pub avg_price: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
pub struct WireAccount {
    pub equity: i64,
    pub balance: i64,
    pub free_margin: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ServerMessage {
    AuthAck,
    Spot { symbol_id: u16, tick: Tick },
    Execution(ExecEvent),
    OrderAccepted { broker_order_id: u64 },
    OrderRejected { reason: String },
    Amended { broker_order_id: u64 },
    Closed { broker_order_id: u64 },
    Positions(Vec<WirePosition>),
    Account(WireAccount),
}

/// `req_id = 0` is reserved for server-initiated frames with no
/// corresponding client request (a spot price tick, or a fill notification
/// arriving asynchronously after an already-acknowledged order) — the same
/// "0 means unsolicited" convention Open API's own `clientMsgId` uses.
pub const UNSOLICITED_REQ_ID: u64 = 0;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ClientFrame {
    pub req_id: u64,
    pub message: ClientMessage,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ServerFrame {
    pub req_id: u64,
    pub message: ServerMessage,
}

/// `[len: u32 BE][json payload...]`.
pub fn write_frame<W: Write, T: Serialize>(writer: &mut W, msg: &T) -> Result<(), ProtocolError> {
    let payload = serde_json::to_vec(msg)?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge(payload.len()));
    }
    writer.write_all(&(payload.len() as u32).to_be_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

pub fn read_frame<R: Read, T: for<'de> Deserialize<'de>>(reader: &mut R) -> Result<T, ProtocolError> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge(len));
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    Ok(serde_json::from_slice(&payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{OrderType, Side, TimeInForce, TradingMode};
    use smallvec::SmallVec;
    use std::io::Cursor;

    fn sample_intent() -> OrderIntent {
        OrderIntent {
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
        }
    }

    #[test]
    fn client_frame_roundtrips_through_a_byte_stream() {
        let frame = ClientFrame { req_id: 42, message: ClientMessage::NewOrder(sample_intent()) };
        let mut buf = Vec::new();
        write_frame(&mut buf, &frame).unwrap();
        let mut cursor = Cursor::new(buf);
        let decoded: ClientFrame = read_frame(&mut cursor).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn server_frame_roundtrips_through_a_byte_stream() {
        let frame = ServerFrame { req_id: 0, message: ServerMessage::Spot { symbol_id: 7, tick: sample_tick() } };
        let mut buf = Vec::new();
        write_frame(&mut buf, &frame).unwrap();
        let mut cursor = Cursor::new(buf);
        let decoded: ServerFrame = read_frame(&mut cursor).unwrap();
        assert_eq!(decoded, frame);
    }

    fn sample_tick() -> Tick {
        Tick { ts_ns: 1_000, recv_ns: 1_001, symbol_id: 7, bid: 100_000, ask: 100_010, bid_volume: 1, ask_volume: 2, flags: 0 }
    }

    #[test]
    fn multiple_frames_are_read_back_in_order_from_the_same_stream() {
        let mut buf = Vec::new();
        write_frame(&mut buf, &ServerFrame { req_id: 1, message: ServerMessage::AuthAck }).unwrap();
        write_frame(&mut buf, &ServerFrame { req_id: 0, message: ServerMessage::Spot { symbol_id: 1, tick: sample_tick() } }).unwrap();
        let mut cursor = Cursor::new(buf);
        let first: ServerFrame = read_frame(&mut cursor).unwrap();
        let second: ServerFrame = read_frame(&mut cursor).unwrap();
        assert_eq!(first.message, ServerMessage::AuthAck);
        assert!(matches!(second.message, ServerMessage::Spot { .. }));
    }

    #[test]
    fn truncated_frame_is_an_io_error_not_a_panic() {
        let mut buf = Vec::new();
        write_frame(&mut buf, &ClientFrame { req_id: 1, message: ClientMessage::PositionsRequest }).unwrap();
        buf.truncate(buf.len() - 1); // chop the last payload byte off
        let mut cursor = Cursor::new(buf);
        let result: Result<ClientFrame, _> = read_frame(&mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn a_length_prefix_over_the_ceiling_is_rejected_before_allocating() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(MAX_FRAME_BYTES as u32 + 1).to_be_bytes());
        let mut cursor = Cursor::new(buf);
        let result: Result<ClientFrame, _> = read_frame(&mut cursor);
        assert!(matches!(result, Err(ProtocolError::FrameTooLarge(_))));
    }
}
