//! TradingView is a signal source and a charting UX reference, not an
//! execution venue (§5.4's adapter matrix: "TradingView does not provide a
//! general order-execution API for retail"). This crate therefore only
//! implements `MarketDataSource` (backed by the UDF datafeed / Pine
//! webhook), never `Broker`. Wiring the actual webhook receiver is Phase 8.

use domain::ports::*;
use domain::{Bar, SymbolId, Tick};

pub struct TradingViewMarketData;

impl MarketDataSource for TradingViewMarketData {
    fn subscribe(&mut self, _symbols: &[SymbolSpec]) -> Result<()> {
        Err(PortError::Adapter("TradingView webhook receiver not implemented yet (Phase 8)".into()))
    }
    fn poll_tick(&mut self) -> Option<Tick> {
        None
    }
    fn history(&self, _sym: SymbolId, _tf: Timeframe, _from_ns: u64, _to_ns: u64) -> Result<Vec<Bar>> {
        Err(PortError::Unsupported)
    }
    fn capabilities(&self) -> FeedCaps {
        FeedCaps { depth: false, volume: false, ticks: false }
    }
}
