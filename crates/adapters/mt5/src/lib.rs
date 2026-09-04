//! MT5 adapter (§5.4). Implements both ports against the ZeroMQ protocol
//! spoken by `bridge/mt5`'s Expert Advisor: PUB socket for ticks/DOM, REQ/REP
//! for `OrderSend`/Modify/Close. The ZMQ client itself is Phase 1 scope
//! (`docs/protocol.md` defines the wire format); this crate currently wires
//! up the trait shape and capability flags so `crates/bin/tradeos-core` can
//! already select an adapter at startup.

use domain::ports::*;
use domain::{Bar, BrokerOrderId, ExecEvent, OrderIntent, SymbolId, Tick};

pub struct Mt5MarketData;
pub struct Mt5Broker;

impl MarketDataSource for Mt5MarketData {
    fn subscribe(&mut self, _symbols: &[SymbolSpec]) -> Result<()> {
        Err(PortError::Adapter("MT5 ZMQ bridge not implemented yet (Phase 1)".into()))
    }
    fn poll_tick(&mut self) -> Option<Tick> {
        None
    }
    fn history(&self, _sym: SymbolId, _tf: Timeframe, _from_ns: u64, _to_ns: u64) -> Result<Vec<Bar>> {
        Err(PortError::Adapter("MT5 ZMQ bridge not implemented yet (Phase 1)".into()))
    }
    fn capabilities(&self) -> FeedCaps {
        FeedCaps { depth: true, volume: true, ticks: true }
    }
}

impl Broker for Mt5Broker {
    fn submit(&mut self, _intent: &OrderIntent) -> Result<BrokerOrderId> {
        Err(PortError::Adapter("MT5 ZMQ bridge not implemented yet (Phase 1)".into()))
    }
    fn modify(&mut self, _id: BrokerOrderId, _sl: Option<i64>, _tp: Option<i64>) -> Result<()> {
        Err(PortError::NotConnected)
    }
    fn close(&mut self, _id: BrokerOrderId, _qty: Option<i64>) -> Result<()> {
        Err(PortError::NotConnected)
    }
    fn poll_event(&mut self) -> Option<ExecEvent> {
        None
    }
    fn account(&self) -> AccountSnapshot {
        AccountSnapshot::default()
    }
    fn constraints(&self, _sym: SymbolId) -> Result<SymbolConstraints> {
        Err(PortError::NotConnected)
    }
}
