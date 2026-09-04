//! Hexagonal ports (§P3, §5.4). Adding a platform means implementing these
//! two traits in a new `crates/adapters/*` crate — zero changes here or in
//! `crates/strategy`/`crates/risk`.

use crate::ids::SymbolId;
use crate::order::{BrokerOrderId, ExecEvent, OrderIntent};
use crate::tick::{Bar, Tick};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeframe {
    M1,
    M5,
    M15,
    H1,
    H4,
    D1,
}

#[derive(Debug, Clone, Copy)]
pub struct SymbolSpec {
    pub symbol_id: SymbolId,
    pub price_digits: u8,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FeedCaps {
    pub depth: bool,
    pub volume: bool,
    pub ticks: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SymbolConstraints {
    pub min_lot: i64,
    pub lot_step: i64,
    pub stop_level_points: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AccountSnapshot {
    pub equity: i64,
    pub balance: i64,
    pub free_margin: i64,
}

/// One open position as the broker itself reports it — the "ground truth"
/// side of position reconciliation (§9.5: "Compare local position book vs
/// broker; any divergence → halt + alert"). Without this, reconciliation
/// has nothing to compare the local book *against*.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionSnapshot {
    pub broker_order_id: BrokerOrderId,
    pub symbol_id: SymbolId,
    pub qty: i64,
    pub avg_price: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("not connected")]
    NotConnected,
    #[error("unsupported by this adapter")]
    Unsupported,
    #[error("adapter error: {0}")]
    Adapter(String),
}

pub type Result<T> = std::result::Result<T, PortError>;

/// Market data source adapter. `poll_tick` is the hot-path method: it must
/// be non-blocking and allocation-free (§5.1) — implementations poll an
/// already-filled ring buffer, they do not perform I/O inline.
pub trait MarketDataSource: Send {
    fn subscribe(&mut self, symbols: &[SymbolSpec]) -> Result<()>;
    fn poll_tick(&mut self) -> Option<Tick>;
    fn history(&self, sym: SymbolId, tf: Timeframe, from_ns: u64, to_ns: u64) -> Result<Vec<Bar>>;
    fn capabilities(&self) -> FeedCaps;
}

/// Broker adapter. This is the only trait whose implementations may
/// actually transmit an order — agents and the strategy VM never see it.
pub trait Broker: Send {
    fn submit(&mut self, intent: &OrderIntent) -> Result<BrokerOrderId>;
    fn modify(&mut self, id: BrokerOrderId, sl: Option<i64>, tp: Option<i64>) -> Result<()>;
    fn close(&mut self, id: BrokerOrderId, qty: Option<i64>) -> Result<()>;
    fn poll_event(&mut self) -> Option<ExecEvent>;
    fn account(&self) -> AccountSnapshot;
    fn constraints(&self, sym: SymbolId) -> Result<SymbolConstraints>;
    /// Every open position, as the broker reports it right now. The §9.5
    /// position-reconciliation guard polls this every 5s and compares it
    /// against the core's own local book — this method existing is what
    /// makes that comparison possible at all, not an optional extra.
    fn positions(&self) -> Result<Vec<PositionSnapshot>>;
}
