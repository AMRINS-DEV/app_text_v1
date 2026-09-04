use domain::{Bar, SymbolId, Tick};

/// Folds ticks into fixed-timeframe OHLCV bars, one aggregator per symbol.
/// O(1) per tick, no allocation after the first bar (matches the §5.1 "no
/// allocation in T0 threads after warm-up" rule).
pub struct BarAggregator {
    symbol_id: SymbolId,
    timeframe_seconds: u32,
    current: Option<Bar>,
}

impl BarAggregator {
    pub fn new(symbol_id: SymbolId, timeframe_seconds: u32) -> Self {
        assert!(timeframe_seconds > 0);
        Self { symbol_id, timeframe_seconds, current: None }
    }

    fn bucket_start_ns(&self, ts_ns: u64) -> u64 {
        let tf_ns = self.timeframe_seconds as u64 * 1_000_000_000;
        (ts_ns / tf_ns) * tf_ns
    }

    /// Feeds one tick in. Returns the just-closed bar when `tick` belongs to
    /// a new bucket, so the caller can push it downstream (§6: `bar.closed`
    /// is the event that drives cache invalidation).
    pub fn on_tick(&mut self, tick: &Tick) -> Option<Bar> {
        let bucket = self.bucket_start_ns(tick.ts_ns);
        let mid = tick.mid();

        match &mut self.current {
            Some(bar) if bar.ts_open_ns == bucket => {
                bar.high = bar.high.max(mid);
                bar.low = bar.low.min(mid);
                bar.close = mid;
                bar.volume += (tick.bid_volume + tick.ask_volume) as u64;
                None
            }
            Some(_) => {
                let closed = self.current.take();
                self.current = Some(Bar {
                    symbol_id: self.symbol_id,
                    timeframe_seconds: self.timeframe_seconds,
                    ts_open_ns: bucket,
                    open: mid,
                    high: mid,
                    low: mid,
                    close: mid,
                    volume: (tick.bid_volume + tick.ask_volume) as u64,
                });
                closed
            }
            None => {
                self.current = Some(Bar {
                    symbol_id: self.symbol_id,
                    timeframe_seconds: self.timeframe_seconds,
                    ts_open_ns: bucket,
                    open: mid,
                    high: mid,
                    low: mid,
                    close: mid,
                    volume: (tick.bid_volume + tick.ask_volume) as u64,
                });
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(ts_ns: u64, bid: i64, ask: i64) -> Tick {
        Tick { ts_ns, recv_ns: ts_ns, symbol_id: 1, bid, ask, bid_volume: 1, ask_volume: 1, flags: 0 }
    }

    #[test]
    fn same_bucket_updates_ohlc_without_closing() {
        let mut agg = BarAggregator::new(1, 60);
        assert!(agg.on_tick(&tick(0, 100, 102)).is_none());
        assert!(agg.on_tick(&tick(30_000_000_000, 105, 107)).is_none());
        let bar = agg.current.unwrap();
        assert_eq!(bar.open, 101);
        assert_eq!(bar.high, 106);
        assert_eq!(bar.close, 106);
    }

    #[test]
    fn crossing_a_bucket_boundary_emits_the_closed_bar() {
        let mut agg = BarAggregator::new(1, 60);
        agg.on_tick(&tick(0, 100, 100));
        let closed = agg.on_tick(&tick(61_000_000_000, 200, 200));
        assert!(closed.is_some());
        assert_eq!(closed.unwrap().close, 100);
    }
}
