//! Feature engine (§8.1, §5.1 "feature" stage). Assembles the full feature
//! taxonomy (volatility, trend, structure, liquidity, order flow, momentum,
//! multi-timeframe, session, cross-asset, news, positioning, cost) into the
//! fixed-layout vector the strategy VM's ONNX model consumes.
//!
//! This crate is a placeholder for Phase 3: it defines the shape so
//! `crates/strategy` has something concrete to depend on, but the SIMD
//! batch computation (`wide`/`polars`) and the full feature list are not
//! implemented yet — only a couple of indicators from `crates/indicators`
//! are wired in as a proof of the pipeline shape.

use domain::Tick;
use indicators::{Ema, Incremental};

/// A minimal slice of the §8.1 feature vector — enough to prove the
/// ingest -> feature wiring, not the full taxonomy.
#[derive(Debug, Clone, Copy, Default)]
pub struct FeatureSnapshot {
    pub ema_fast: Option<f64>,
    pub ema_slow: Option<f64>,
}

pub struct FeatureEngine {
    ema_fast: Ema,
    ema_slow: Ema,
}

impl FeatureEngine {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self { ema_fast: Ema::new(fast_period), ema_slow: Ema::new(slow_period) }
    }

    pub fn on_tick(&mut self, tick: &Tick) -> FeatureSnapshot {
        let mid = tick.mid() as f64;
        FeatureSnapshot { ema_fast: self.ema_fast.update(mid), ema_slow: self.ema_slow.update(mid) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_a_snapshot_per_tick() {
        let mut engine = FeatureEngine::new(3, 10);
        let t = Tick { ts_ns: 0, recv_ns: 0, symbol_id: 1, bid: 100, ask: 100, bid_volume: 1, ask_volume: 1, flags: 0 };
        let snap = engine.on_tick(&t);
        assert_eq!(snap.ema_fast, Some(100.0));
    }
}
