//! MT5 adapter (§5.4). Implements both ports against the ZeroMQ protocol
//! spoken by `bridge/mt5`'s Expert Advisor: PUB socket for ticks, REQ/REP
//! for OrderSend/Modify/Close (`docs/protocol.md`, `protocol.rs`).
//!
//! The MQL5 EA side cannot be compiled or run in this environment (no MT5
//! terminal), so this adapter is developed and tested against
//! `crates/bin/mock-mt5-bridge` — a Rust test double speaking the exact
//! same wire protocol. That makes the **tick path** genuinely verified
//! end-to-end (real ZMQ sockets, real rkyv encode/decode, measured
//! latency — see `tests/bridge_integration.rs`).
//!
//! The **order path** is Rust-peer-only as implemented: `protocol::OrderRequest`/
//! `OrderReply` rely on rkyv's relative-pointer archive format for their
//! `Option`/`String`/`SmallVec` fields, which a hand-written MQL5 decoder
//! cannot feasibly reproduce (see `docs/protocol.md`'s "Important
//! correction" for how this was found). Connecting this adapter to a real
//! EA needs a flat, MQL5-friendly order encoding first — not a change to
//! this code path's logic, just its wire format.
//!
//! Async ZMQ I/O runs on a dedicated background OS thread (its own small
//! tokio runtime), decoupled from the hot path via a lock-free channel —
//! `poll_tick`/order calls never block on network I/O directly, matching
//! §5.1's "non-blocking hot path" rule. This is a pragmatic Phase 1 stand-in
//! for the `iceoryx2`/`rtrb` shared-memory rings §5.1 specifies for the
//! *in-process* pipeline stages; the bridge boundary itself talks ZMQ either
//! way (§5.4's own latency table lists both as options).

pub mod protocol;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use domain::ports::*;
use domain::{Bar, BrokerOrderId, ExecEvent, OrderIntent, SymbolId, Tick};
use zeromq::{Socket, SocketRecv, SocketSend};

use protocol::{
    decode_market_data_frame, decode_order_reply, encode_order_request, MarketDataFrame, OrderReply, OrderRequest,
};

const ORDER_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// `MarketDataSource` over ZMQ SUB. Symbol filtering happens client-side in
/// Phase 1 (the mock bridge and the real EA both publish everything on one
/// topic) — a per-symbol ZMQ topic prefix is a Phase 2 refinement, not a
/// wire-format change.
pub struct Mt5MarketData {
    endpoint: String,
    rx: Option<Receiver<Tick>>,
    // Retained (not joined) for graceful-shutdown wiring in Phase 2 — the
    // background thread self-terminates once its channel closes.
    #[allow(dead_code)]
    handle: Option<JoinHandle<()>>,
    last_seq: Arc<AtomicU64>,
    last_heartbeat_seq: Arc<AtomicU64>,
}

impl Mt5MarketData {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            rx: None,
            handle: None,
            last_seq: Arc::new(AtomicU64::new(0)),
            last_heartbeat_seq: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Ticks received since the last heartbeat/tick with a non-contiguous
    /// sequence number get their `GAP` flag set (§5.4 docs/protocol.md);
    /// this exposes the running count for telemetry (`feed_gap_seconds`,
    /// §14) without requiring a full guard wiring yet (Phase 2 scope).
    pub fn last_seq(&self) -> u64 {
        self.last_seq.load(Ordering::Relaxed)
    }
}

impl MarketDataSource for Mt5MarketData {
    fn subscribe(&mut self, _symbols: &[SymbolSpec]) -> Result<()> {
        let (tx, rx): (Sender<Tick>, Receiver<Tick>) = bounded(4096);
        let endpoint = self.endpoint.clone();
        let last_seq = self.last_seq.clone();
        let last_heartbeat_seq = self.last_heartbeat_seq.clone();

        let handle = std::thread::Builder::new()
            .name("mt5-md-ingest".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio runtime");
                rt.block_on(async move {
                    let mut sub = zeromq::SubSocket::new();
                    if sub.connect(&endpoint).await.is_err() {
                        return;
                    }
                    if sub.subscribe("").await.is_err() {
                        return;
                    }
                    let mut expected_seq: Option<u64> = None;
                    loop {
                        let Ok(msg) = sub.recv().await else { break };
                        let Some(bytes) = msg.get(0) else { continue };
                        match decode_market_data_frame(bytes) {
                            Ok(MarketDataFrame::Tick { seq, mut tick }) => {
                                if let Some(exp) = expected_seq {
                                    if seq != exp {
                                        tick.flags |= domain::TickFlags::GAP.bits();
                                    }
                                }
                                expected_seq = Some(seq + 1);
                                last_seq.store(seq, Ordering::Relaxed);
                                if tx.send(tick).is_err() {
                                    break; // receiver dropped
                                }
                            }
                            Ok(MarketDataFrame::Heartbeat { seq }) => {
                                last_heartbeat_seq.store(seq, Ordering::Relaxed);
                            }
                            Err(_) => continue, // malformed frame: drop and keep going, never panic on the hot path
                        }
                    }
                });
            })
            .expect("failed to spawn mt5-md-ingest thread");

        self.rx = Some(rx);
        self.handle = Some(handle);
        Ok(())
    }

    fn poll_tick(&mut self) -> Option<Tick> {
        self.rx.as_ref()?.try_recv().ok()
    }

    fn history(&self, _sym: SymbolId, _tf: Timeframe, _from_ns: u64, _to_ns: u64) -> Result<Vec<Bar>> {
        // MT5 history export runs offline via bridge/mt5/Scripts/ExportHistory.mq5
        // (§4) into storage, not over this live socket — Phase 1 scope.
        Err(PortError::Unsupported)
    }

    fn capabilities(&self) -> FeedCaps {
        FeedCaps { depth: true, volume: true, ticks: true }
    }
}

/// `Broker` over ZMQ REQ/REP. One background thread owns the REQ socket
/// (REQ/REP is strictly ordered request-then-reply, so it cannot be shared
/// across concurrent callers without serializing through something) —
/// `submit`/`modify`/`close` each hand a request across a channel and block
/// on a per-call reply channel with a timeout.
pub struct Mt5Broker {
    tx: Option<Sender<(OrderRequest, std_mpsc::Sender<OrderReply>)>>,
    // Retained (not joined) for graceful-shutdown wiring in Phase 2 — the
    // background thread self-terminates once its channel closes.
    #[allow(dead_code)]
    handle: Option<JoinHandle<()>>,
}

impl Mt5Broker {
    pub fn new(endpoint: impl Into<String>) -> Self {
        let endpoint = endpoint.into();
        let (tx, rx): (Sender<(OrderRequest, std_mpsc::Sender<OrderReply>)>, _) = bounded(256);

        let handle = std::thread::Builder::new()
            .name("mt5-broker-req".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio runtime");
                rt.block_on(async move {
                    let mut req_sock = zeromq::ReqSocket::new();
                    if req_sock.connect(&endpoint).await.is_err() {
                        return;
                    }
                    while let Ok((request, reply_tx)) = rx.recv() {
                        let bytes = encode_order_request(&request);
                        let send_ok = req_sock.send(bytes.into()).await.is_ok();
                        if !send_ok {
                            let _ = reply_tx.send(OrderReply::Rejected { reason: "bridge send failed".into() });
                            continue;
                        }
                        match req_sock.recv().await {
                            Ok(msg) => {
                                let reply = msg
                                    .get(0)
                                    .and_then(|b| decode_order_reply(b).ok())
                                    .unwrap_or(OrderReply::Rejected { reason: "malformed reply".into() });
                                let _ = reply_tx.send(reply);
                            }
                            Err(_) => {
                                let _ = reply_tx.send(OrderReply::Rejected { reason: "bridge recv failed".into() });
                            }
                        }
                    }
                });
            })
            .expect("failed to spawn mt5-broker-req thread");

        Self { tx: Some(tx), handle: Some(handle) }
    }

    fn call(&self, request: OrderRequest) -> Result<OrderReply> {
        let tx = self.tx.as_ref().ok_or(PortError::NotConnected)?;
        let (reply_tx, reply_rx) = std_mpsc::channel();
        tx.send((request, reply_tx)).map_err(|_| PortError::NotConnected)?;
        reply_rx.recv_timeout(ORDER_REQUEST_TIMEOUT).map_err(|_| PortError::Adapter("order reply timed out".into()))
    }
}

impl Broker for Mt5Broker {
    fn submit(&mut self, intent: &OrderIntent) -> Result<BrokerOrderId> {
        match self.call(OrderRequest::Submit(intent.clone()))? {
            OrderReply::Accepted { broker_order_id } => Ok(broker_order_id),
            OrderReply::Rejected { reason } => Err(PortError::Adapter(reason)),
            other => Err(PortError::Adapter(format!("unexpected reply to submit: {other:?}"))),
        }
    }

    fn modify(&mut self, id: BrokerOrderId, sl: Option<i64>, tp: Option<i64>) -> Result<()> {
        match self.call(OrderRequest::Modify { broker_order_id: id, sl, tp })? {
            OrderReply::Modified => Ok(()),
            OrderReply::Rejected { reason } => Err(PortError::Adapter(reason)),
            other => Err(PortError::Adapter(format!("unexpected reply to modify: {other:?}"))),
        }
    }

    fn close(&mut self, id: BrokerOrderId, qty: Option<i64>) -> Result<()> {
        match self.call(OrderRequest::Close { broker_order_id: id, qty })? {
            OrderReply::Closed => Ok(()),
            OrderReply::Rejected { reason } => Err(PortError::Adapter(reason)),
            other => Err(PortError::Adapter(format!("unexpected reply to close: {other:?}"))),
        }
    }

    fn poll_event(&mut self) -> Option<ExecEvent> {
        // Fills/rejects/modifies pushed asynchronously by the EA need a
        // second PUB/SUB channel symmetric to market data — not yet in
        // docs/protocol.md. Phase 2 scope.
        None
    }

    fn account(&self) -> AccountSnapshot {
        // Needs an AccountInfo query message added to protocol.rs — Phase 2 scope.
        AccountSnapshot::default()
    }

    fn constraints(&self, _sym: SymbolId) -> Result<SymbolConstraints> {
        // Needs a SymbolInfo query message added to protocol.rs — Phase 2 scope.
        Err(PortError::Unsupported)
    }

    fn positions(&self) -> Result<Vec<domain::PositionSnapshot>> {
        // Needs a PositionInfo query message added to protocol.rs — Phase 2 scope.
        Err(PortError::Unsupported)
    }
}
