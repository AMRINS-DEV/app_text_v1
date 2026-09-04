//! Binance adapter stub — see `adapter-ctrader` for the rationale (§17 Phase 8).

use domain::ports::*;
use domain::{Bar, BrokerOrderId, ExecEvent, OrderIntent, SymbolId, Tick};

pub struct BinanceMarketData;
pub struct BinanceBroker;

impl MarketDataSource for BinanceMarketData {
    fn subscribe(&mut self, _symbols: &[SymbolSpec]) -> Result<()> {
        Err(PortError::Adapter("Binance adapter not implemented yet (Phase 8)".into()))
    }
    fn poll_tick(&mut self) -> Option<Tick> {
        None
    }
    fn history(&self, _sym: SymbolId, _tf: Timeframe, _from_ns: u64, _to_ns: u64) -> Result<Vec<Bar>> {
        Err(PortError::Adapter("Binance adapter not implemented yet (Phase 8)".into()))
    }
    fn capabilities(&self) -> FeedCaps {
        FeedCaps { depth: true, volume: true, ticks: true }
    }
}

impl Broker for BinanceBroker {
    fn submit(&mut self, _intent: &OrderIntent) -> Result<BrokerOrderId> {
        Err(PortError::Adapter("Binance adapter not implemented yet (Phase 8)".into()))
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
