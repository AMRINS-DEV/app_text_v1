//! Test double for the MQL5 bridge EA (§5.4). Speaks the exact wire
//! protocol `adapter-mt5::protocol` defines, so `adapter-mt5` — and the
//! §5.2 bridge latency budget — can be exercised end-to-end without a real
//! MT5 terminal. A real EA is a drop-in replacement: point `adapter-mt5` at
//! its endpoint instead of this binary's.
//!
//! Tick prices are a deterministic synthetic walk (no RNG) so runs are
//! reproducible — this is a test double, not a market simulator; realistic
//! price dynamics belong to NautilusTrader backtests (§3.2), not here.

use domain::Tick;
use zeromq::{Socket, SocketRecv, SocketSend};

use adapter_mt5::protocol::{
    decode_order_request, encode_heartbeat_frame, encode_order_reply, encode_tick_frame, OrderReply, OrderRequest,
};

/// Nanoseconds since the Unix epoch — used to stamp the real send time on
/// each tick so a receiver can measure actual bridge -> core latency (§5.2),
/// distinct from `synthetic_tick`'s deterministic 1ms-spacing timestamp.
pub fn now_ns() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64
}

pub struct MarketDataConfig {
    pub bind_addr: String,
    pub symbol_id: u16,
    /// `None` streams forever (real EA behavior); `Some(n)` stops after `n`
    /// ticks (test/benchmark behavior).
    pub max_ticks: Option<u64>,
    /// Ticks between each heartbeat frame.
    pub ticks_per_heartbeat: u64,
}

/// Deterministic synthetic tick for sequence number `seq`: a small
/// triangle-wave oscillation around a fixed-point base price so bar
/// aggregation and indicators downstream see plausible OHLC movement.
pub fn synthetic_tick(symbol_id: u16, seq: u64) -> Tick {
    const BASE_PRICE: i64 = 100_000; // fixed-point, matches domain::Tick's scaling convention
    const AMPLITUDE: i64 = 200;
    const PERIOD: i64 = 40;
    let phase = (seq as i64) % PERIOD;
    let triangle = if phase < PERIOD / 2 { phase } else { PERIOD - phase };
    let bid = BASE_PRICE + (triangle * AMPLITUDE) / (PERIOD / 2);
    Tick {
        ts_ns: seq * 1_000_000, // 1ms synthetic spacing
        recv_ns: seq * 1_000_000,
        symbol_id,
        bid,
        ask: bid + 10,
        bid_volume: 1 + (seq % 5) as u32,
        ask_volume: 1 + (seq % 3) as u32,
        flags: 0,
    }
}

/// Binds a PUB socket and streams synthetic ticks + periodic heartbeats.
/// Returns once `max_ticks` is reached (or never, if `None`).
///
/// Waits `startup_grace` after bind before sending the first tick — ZMQ
/// PUB/SUB is fire-and-forget for subscribers that haven't connected yet
/// (the classic "slow joiner" problem), and unlike a real trading session
/// (where the core connects once and stays connected for hours), this
/// bridge's test callers connect only after seeing it start, so without a
/// grace period a short, un-throttled `max_ticks` burst can finish and
/// even exit before the subscriber's connect/subscribe completes.
pub async fn run_market_data(cfg: MarketDataConfig) -> Result<(), zeromq::ZmqError> {
    let mut pub_sock = zeromq::PubSocket::new();
    pub_sock.bind(&cfg.bind_addr).await?;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let mut seq: u64 = 0;
    loop {
        if let Some(max) = cfg.max_ticks {
            if seq >= max {
                return Ok(());
            }
        }
        let mut tick = synthetic_tick(cfg.symbol_id, seq);
        // Overwrite with the real send-time wall clock so callers can measure
        // actual bridge -> core latency (§5.2) rather than the synthetic
        // 1ms-spacing timestamp `synthetic_tick` uses for price-walk determinism.
        tick.ts_ns = now_ns();
        pub_sock.send(encode_tick_frame(seq, &tick).into()).await?;
        seq += 1;
        if cfg.ticks_per_heartbeat > 0 && seq.is_multiple_of(cfg.ticks_per_heartbeat) {
            pub_sock.send(encode_heartbeat_frame(seq).into()).await?;
        }
    }
}

/// Binds a REP socket and answers order requests deterministically:
/// every `Submit` is accepted with an incrementing broker order id, every
/// `Modify`/`Close` succeeds. Returns after `max_requests` (or never, if `None`).
pub async fn run_order_responder(bind_addr: String, max_requests: Option<u64>) -> Result<(), zeromq::ZmqError> {
    let mut rep_sock = zeromq::RepSocket::new();
    rep_sock.bind(&bind_addr).await?;

    let mut next_broker_order_id: u64 = 1;
    let mut served: u64 = 0;
    loop {
        if let Some(max) = max_requests {
            if served >= max {
                return Ok(());
            }
        }
        let msg = rep_sock.recv().await?;
        let reply = match msg.get(0).and_then(|b| decode_order_request(b).ok()) {
            Some(OrderRequest::Submit(_intent)) => {
                let id = next_broker_order_id;
                next_broker_order_id += 1;
                OrderReply::Accepted { broker_order_id: id }
            }
            Some(OrderRequest::Modify { .. }) => OrderReply::Modified,
            Some(OrderRequest::Close { .. }) => OrderReply::Closed,
            None => OrderReply::Rejected { reason: "malformed request".into() },
        };
        rep_sock.send(encode_order_reply(&reply).into()).await?;
        served += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_walk_is_deterministic_and_bounded() {
        let a = synthetic_tick(1, 100);
        let b = synthetic_tick(1, 100);
        assert_eq!(a, b);
        for seq in 0..200 {
            let t = synthetic_tick(1, seq);
            assert!(t.bid >= 100_000 && t.bid <= 100_200);
            assert!(t.ask > t.bid);
        }
    }
}
