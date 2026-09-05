//! Test double for a cTrader Open API server (§17 Phase 8). Speaks the
//! exact wire protocol `adapter_ctrader::protocol` defines, so
//! `adapter-ctrader` can be exercised end-to-end without a real cTrader
//! account. A real Open API TLS endpoint is a drop-in replacement: point
//! `adapter-ctrader` at its address instead of this binary's (modulo the
//! documented protocol substitution `adapter_ctrader::protocol` explains).
//!
//! Tick prices are a deterministic synthetic walk (no RNG), same
//! reproducibility discipline as `mock-mt5-bridge::synthetic_tick` — this is
//! a test double, not a market simulator.

use std::io;
use std::net::TcpListener;

use adapter_ctrader::protocol::{read_frame, write_frame, ClientFrame, ClientMessage, ServerFrame, ServerMessage, WireAccount, WirePosition, UNSOLICITED_REQ_ID};
use domain::{ExecEvent, Tick};

pub struct MarketDataConfig {
    pub bind_addr: String,
    pub symbol_id: u16,
    /// `None` streams forever; `Some(n)` stops after `n` ticks (test behavior).
    pub max_ticks: Option<u64>,
}

/// Deterministic synthetic tick for sequence number `seq` — a different
/// shape (sawtooth, not triangle) than `mock-mt5-bridge::synthetic_tick` on
/// purpose, so nothing here is a copy-paste of the MT5 test double under a
/// new name.
pub fn synthetic_tick(symbol_id: u16, seq: u64) -> Tick {
    const BASE_PRICE: i64 = 50_000;
    const AMPLITUDE: i64 = 300;
    const PERIOD: i64 = 30;
    let bid = BASE_PRICE + ((seq as i64) % PERIOD) * (AMPLITUDE / PERIOD);
    Tick {
        ts_ns: seq * 1_000_000,
        recv_ns: seq * 1_000_000,
        symbol_id,
        bid,
        ask: bid + 8,
        bid_volume: 1 + (seq % 4) as u32,
        ask_volume: 1 + (seq % 6) as u32,
        flags: 0,
    }
}

/// Accepts one connection, reads the client's initial `Subscribe` frame
/// (acknowledged with `AuthAck`), then streams unsolicited `Spot` frames
/// until `max_ticks` is reached.
pub fn run_market_data(cfg: MarketDataConfig) -> io::Result<()> {
    let listener = TcpListener::bind(&cfg.bind_addr)?;
    let (mut stream, _) = listener.accept()?;

    let sub_frame: ClientFrame = read_frame(&mut stream).map_err(io::Error::other)?;
    write_frame(&mut stream, &ServerFrame { req_id: sub_frame.req_id, message: ServerMessage::AuthAck }).map_err(io::Error::other)?;

    let mut seq: u64 = 0;
    loop {
        if let Some(max) = cfg.max_ticks {
            if seq >= max {
                return Ok(());
            }
        }
        let tick = synthetic_tick(cfg.symbol_id, seq);
        write_frame(&mut stream, &ServerFrame { req_id: UNSOLICITED_REQ_ID, message: ServerMessage::Spot { symbol_id: cfg.symbol_id, tick } })
            .map_err(io::Error::other)?;
        seq += 1;
    }
}

/// Accepts one connection and answers order/position/account requests
/// deterministically: every `NewOrder` is accepted with an incrementing
/// broker order id, immediately followed by an unsolicited `Execution` fill
/// event (the same submit-then-async-fill lifecycle a real broker has, per
/// `adapter_ctrader`'s own doc comment) — `poll_event` is what actually
/// observes it, not the `submit` reply itself. Tracks a minimal in-memory
/// position book so `AmendOrder`/`ClosePosition`/`PositionsRequest` behave
/// consistently with prior `NewOrder`s in the same run. Returns after
/// `max_requests` (or never, if `None`).
pub fn run_order_responder(bind_addr: String, max_requests: Option<u64>) -> io::Result<()> {
    let listener = TcpListener::bind(&bind_addr)?;
    let (stream, _) = listener.accept()?;
    let mut reader = stream.try_clone()?;
    let mut writer = stream;

    let mut next_broker_order_id: u64 = 1;
    let mut served: u64 = 0;
    let mut positions: Vec<WirePosition> = Vec::new();

    loop {
        if let Some(max) = max_requests {
            if served >= max {
                return Ok(());
            }
        }
        let frame: ClientFrame = match read_frame(&mut reader) {
            Ok(f) => f,
            Err(_) => return Ok(()), // connection closed by the client: a clean end, not an error
        };

        match frame.message {
            ClientMessage::NewOrder(intent) => {
                let broker_order_id = next_broker_order_id;
                next_broker_order_id += 1;
                let fill_price = intent.limit_px.unwrap_or(100_000);
                positions.push(WirePosition { broker_order_id, symbol_id: intent.symbol_id, qty: intent.qty, avg_price: fill_price });

                write_frame(&mut writer, &ServerFrame { req_id: frame.req_id, message: ServerMessage::OrderAccepted { broker_order_id } })
                    .map_err(io::Error::other)?;
                write_frame(
                    &mut writer,
                    &ServerFrame {
                        req_id: UNSOLICITED_REQ_ID,
                        message: ServerMessage::Execution(ExecEvent::Fill {
                            client_id: intent.client_id,
                            broker_order_id,
                            fill_price,
                            qty: intent.qty,
                            ts_ns: 0,
                        }),
                    },
                )
                .map_err(io::Error::other)?;
            }
            ClientMessage::AmendOrder { broker_order_id, .. } => {
                let reply = if positions.iter().any(|p| p.broker_order_id == broker_order_id) {
                    ServerMessage::Amended { broker_order_id }
                } else {
                    ServerMessage::OrderRejected { reason: "unknown order".into() }
                };
                write_frame(&mut writer, &ServerFrame { req_id: frame.req_id, message: reply }).map_err(io::Error::other)?;
            }
            ClientMessage::ClosePosition { broker_order_id, qty } => {
                let reply = match positions.iter().position(|p| p.broker_order_id == broker_order_id) {
                    Some(idx) => {
                        match qty {
                            Some(q) if q < positions[idx].qty => positions[idx].qty -= q,
                            _ => {
                                positions.remove(idx);
                            }
                        }
                        ServerMessage::Closed { broker_order_id }
                    }
                    None => ServerMessage::OrderRejected { reason: "unknown order".into() },
                };
                write_frame(&mut writer, &ServerFrame { req_id: frame.req_id, message: reply }).map_err(io::Error::other)?;
            }
            ClientMessage::PositionsRequest => {
                write_frame(&mut writer, &ServerFrame { req_id: frame.req_id, message: ServerMessage::Positions(positions.clone()) })
                    .map_err(io::Error::other)?;
            }
            ClientMessage::AccountRequest => {
                let account = WireAccount { equity: 1_000_000, balance: 1_000_000, free_margin: 1_000_000 };
                write_frame(&mut writer, &ServerFrame { req_id: frame.req_id, message: ServerMessage::Account(account) })
                    .map_err(io::Error::other)?;
            }
            ClientMessage::Subscribe { .. } => {
                // Not expected on the order connection in this adapter's
                // two-connection split; ignore rather than error, matching
                // `mock-mt5-bridge`'s "malformed frame: drop and keep going"
                // hot-path discipline.
            }
        }
        served += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_walk_is_deterministic_and_bounded() {
        let a = synthetic_tick(1, 17);
        let b = synthetic_tick(1, 17);
        assert_eq!(a, b);
        for seq in 0..200 {
            let t = synthetic_tick(1, seq);
            assert!(t.bid >= 50_000 && t.bid < 50_300);
            assert!(t.ask > t.bid);
        }
    }
}
