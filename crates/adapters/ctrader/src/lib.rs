//! cTrader adapter (§5.4, §17 Phase 8: "second broker adapter... same
//! strategy runs on 2 adapters with no core change"). This crate exists to
//! prove the `domain::ports` trait boundary is the only thing a new
//! platform needs to satisfy — see `protocol`'s own doc comment for why its
//! wire format is a documented, honest stand-in for cTrader's real Open API
//! rather than a byte-for-byte Protobuf reimplementation, and why it
//! deliberately uses a different transport stack than `adapter-mt5`
//! (blocking `std::net::TcpStream` + JSON here, vs. async ZMQ + rkyv
//! there).
//!
//! Developed and tested against `mock-ctrader-server`, this crate's own
//! test double, the same "real adapter logic, mock counterparty" split as
//! `adapter-mt5`/`mock-mt5-bridge`. `crates/execution::OrderRouter<B>` is
//! generic over `B: Broker`; `tests/cross_adapter_parity.rs` runs the exact
//! same `OrderRouter` call sequence against both `SimBroker` and this
//! crate's `CTraderBroker`, which is the literal exit criterion made
//! concrete rather than just argued in prose.
//!
//! Order acceptance is synchronous (an `OrderAccepted`/`OrderRejected`
//! reply to the `submit` call itself), but the resulting fill arrives as a
//! separate, unsolicited `Execution` frame the reader thread routes to
//! `poll_event` — the same submit-then-poll lifecycle `SimBroker`'s own doc
//! comment describes, but here genuinely crossing a socket boundary rather
//! than an in-process queue.

pub mod protocol;

use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use domain::ports::*;
use domain::{Bar, BrokerOrderId, ExecEvent, OrderIntent, SymbolId, Tick};

use protocol::{read_frame, write_frame, ClientFrame, ClientMessage, ServerFrame, ServerMessage, UNSOLICITED_REQ_ID};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// `MarketDataSource` over a dedicated TCP connection carrying unsolicited
/// `Spot` frames — cTrader's Open API multiplexes spot prices and trading
/// on one connection; this adapter splits them into two (mirroring
/// `adapter-mt5`'s PUB/SUB-vs-REQ/REP split) as a documented simplification,
/// not a protocol-fidelity claim.
pub struct CTraderMarketData {
    endpoint: String,
    rx: Option<Receiver<Tick>>,
    #[allow(dead_code)]
    handle: Option<JoinHandle<()>>,
}

impl CTraderMarketData {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into(), rx: None, handle: None }
    }
}

impl MarketDataSource for CTraderMarketData {
    fn subscribe(&mut self, symbols: &[SymbolSpec]) -> Result<()> {
        let mut stream = TcpStream::connect(&self.endpoint)
            .map_err(|e| PortError::Adapter(format!("connect failed: {e}")))?;
        let symbol_ids: Vec<u16> = symbols.iter().map(|s| s.symbol_id).collect();
        write_frame(&mut stream, &ClientFrame { req_id: 1, message: ClientMessage::Subscribe { symbols: symbol_ids } })
            .map_err(|e| PortError::Adapter(format!("subscribe failed: {e}")))?;

        let (tx, rx): (Sender<Tick>, Receiver<Tick>) = bounded(4096);
        let handle = std::thread::Builder::new()
            .name("ctrader-md-ingest".into())
            .spawn(move || {
                let mut reader = stream;
                loop {
                    match read_frame::<_, ServerFrame>(&mut reader) {
                        Ok(ServerFrame { message: ServerMessage::Spot { tick, .. }, .. }) => {
                            if tx.send(tick).is_err() {
                                break; // receiver dropped
                            }
                        }
                        Ok(_) => continue,
                        Err(_) => break, // connection closed or malformed frame: stop, never panic on the hot path
                    }
                }
            })
            .expect("failed to spawn ctrader-md-ingest thread");

        self.rx = Some(rx);
        self.handle = Some(handle);
        Ok(())
    }

    fn poll_tick(&mut self) -> Option<Tick> {
        self.rx.as_ref()?.try_recv().ok()
    }

    fn history(&self, _sym: SymbolId, _tf: Timeframe, _from_ns: u64, _to_ns: u64) -> Result<Vec<Bar>> {
        // Open API does expose historical trendbars (ProtoOAGetTrendbarsReq);
        // wiring a third message type for it isn't exercised by the Phase 8
        // exit criterion (order-routing parity across adapters) — same
        // "Phase 2 scope" deferral `Mt5MarketData::history` already made.
        Err(PortError::Unsupported)
    }

    fn capabilities(&self) -> FeedCaps {
        FeedCaps { depth: false, volume: true, ticks: true }
    }
}

struct PendingRequest {
    message: ClientMessage,
    reply_tx: std_mpsc::Sender<ServerMessage>,
}

/// `Broker` over a second TCP connection. One thread owns the write half
/// and assigns each outbound request a fresh `req_id`; a second thread owns
/// the read half and demultiplexes replies (by `req_id`) from unsolicited
/// `Execution` pushes (`req_id == 0`) against a shared pending-request map —
/// the mechanism a single connection needs once request/reply and
/// server-initiated pushes share one byte stream.
pub struct CTraderBroker {
    request_tx: Option<Sender<PendingRequest>>,
    event_rx: Option<Receiver<ExecEvent>>,
    #[allow(dead_code)]
    write_handle: Option<JoinHandle<()>>,
    #[allow(dead_code)]
    read_handle: Option<JoinHandle<()>>,
}

impl CTraderBroker {
    pub fn new(endpoint: impl Into<String>) -> Self {
        let endpoint = endpoint.into();
        match TcpStream::connect(&endpoint) {
            Ok(stream) => Self::from_stream(stream),
            Err(_) => Self { request_tx: None, event_rx: None, write_handle: None, read_handle: None },
        }
    }

    fn from_stream(stream: TcpStream) -> Self {
        let write_stream = stream.try_clone().expect("tcp stream clone for ctrader broker write half");
        let read_stream = stream;

        let (request_tx, request_rx): (Sender<PendingRequest>, Receiver<PendingRequest>) = bounded(256);
        let (event_tx, event_rx): (Sender<ExecEvent>, Receiver<ExecEvent>) = bounded(1024);
        let pending: Arc<Mutex<HashMap<u64, std_mpsc::Sender<ServerMessage>>>> = Arc::new(Mutex::new(HashMap::new()));
        let next_req_id = Arc::new(AtomicU64::new(1));

        let pending_writer = pending.clone();
        let next_req_id_writer = next_req_id.clone();
        let write_handle = std::thread::Builder::new()
            .name("ctrader-broker-write".into())
            .spawn(move || {
                let mut writer = write_stream;
                while let Ok(PendingRequest { message, reply_tx }) = request_rx.recv() {
                    let req_id = next_req_id_writer.fetch_add(1, Ordering::Relaxed);
                    pending_writer.lock().unwrap().insert(req_id, reply_tx.clone());
                    if write_frame(&mut writer, &ClientFrame { req_id, message }).is_err() {
                        pending_writer.lock().unwrap().remove(&req_id);
                        let _ = reply_tx.send(ServerMessage::OrderRejected { reason: "write failed".into() });
                    }
                }
            })
            .expect("failed to spawn ctrader-broker-write thread");

        let read_handle = std::thread::Builder::new()
            .name("ctrader-broker-read".into())
            .spawn(move || {
                let mut reader = read_stream;
                // connection closed: stop, outstanding calls time out naturally
                while let Ok(ServerFrame { req_id, message }) = read_frame::<_, ServerFrame>(&mut reader) {
                    if req_id == UNSOLICITED_REQ_ID {
                        if let ServerMessage::Execution(event) = message {
                            let _ = event_tx.send(event);
                        }
                    } else if let Some(reply_tx) = pending.lock().unwrap().remove(&req_id) {
                        let _ = reply_tx.send(message);
                    }
                }
            })
            .expect("failed to spawn ctrader-broker-read thread");

        Self {
            request_tx: Some(request_tx),
            event_rx: Some(event_rx),
            write_handle: Some(write_handle),
            read_handle: Some(read_handle),
        }
    }

    fn call(&self, message: ClientMessage) -> Result<ServerMessage> {
        let tx = self.request_tx.as_ref().ok_or(PortError::NotConnected)?;
        let (reply_tx, reply_rx) = std_mpsc::channel();
        tx.send(PendingRequest { message, reply_tx }).map_err(|_| PortError::NotConnected)?;
        reply_rx.recv_timeout(REQUEST_TIMEOUT).map_err(|_| PortError::Adapter("request timed out".into()))
    }
}

impl Broker for CTraderBroker {
    fn submit(&mut self, intent: &OrderIntent) -> Result<BrokerOrderId> {
        match self.call(ClientMessage::NewOrder(intent.clone()))? {
            ServerMessage::OrderAccepted { broker_order_id } => Ok(broker_order_id),
            ServerMessage::OrderRejected { reason } => Err(PortError::Adapter(reason)),
            other => Err(PortError::Adapter(format!("unexpected reply to submit: {other:?}"))),
        }
    }

    fn modify(&mut self, id: BrokerOrderId, sl: Option<i64>, tp: Option<i64>) -> Result<()> {
        match self.call(ClientMessage::AmendOrder { broker_order_id: id, sl, tp })? {
            ServerMessage::Amended { .. } => Ok(()),
            ServerMessage::OrderRejected { reason } => Err(PortError::Adapter(reason)),
            other => Err(PortError::Adapter(format!("unexpected reply to modify: {other:?}"))),
        }
    }

    fn close(&mut self, id: BrokerOrderId, qty: Option<i64>) -> Result<()> {
        match self.call(ClientMessage::ClosePosition { broker_order_id: id, qty })? {
            ServerMessage::Closed { .. } => Ok(()),
            ServerMessage::OrderRejected { reason } => Err(PortError::Adapter(reason)),
            other => Err(PortError::Adapter(format!("unexpected reply to close: {other:?}"))),
        }
    }

    fn poll_event(&mut self) -> Option<ExecEvent> {
        self.event_rx.as_ref()?.try_recv().ok()
    }

    fn account(&self) -> AccountSnapshot {
        match self.call(ClientMessage::AccountRequest) {
            Ok(ServerMessage::Account(acc)) => AccountSnapshot { equity: acc.equity, balance: acc.balance, free_margin: acc.free_margin },
            _ => AccountSnapshot::default(),
        }
    }

    fn constraints(&self, _sym: SymbolId) -> Result<SymbolConstraints> {
        // Open API exposes real symbol constraints (ProtoOASymbolByIdReq);
        // wiring a third request/reply message type isn't needed to
        // demonstrate order-routing parity across adapters — same
        // deferral `Mt5Broker::constraints` already made for Phase 1/2.
        Ok(SymbolConstraints { min_lot: 1, lot_step: 1, stop_level_points: 10 })
    }

    fn positions(&self) -> Result<Vec<domain::PositionSnapshot>> {
        match self.call(ClientMessage::PositionsRequest)? {
            ServerMessage::Positions(list) => Ok(list
                .into_iter()
                .map(|p| domain::PositionSnapshot {
                    broker_order_id: p.broker_order_id,
                    symbol_id: p.symbol_id,
                    qty: p.qty,
                    avg_price: p.avg_price,
                })
                .collect()),
            other => Err(PortError::Adapter(format!("unexpected reply to positions: {other:?}"))),
        }
    }
}
