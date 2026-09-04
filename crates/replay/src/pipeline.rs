//! Deterministic replay-through-pipeline (§P4, §15 item 2). Feeds a
//! recorded tick sequence through the same market-data/feature/risk crates
//! the live core uses and produces a decision stream; running the same
//! input twice must produce a byte-for-byte identical stream — that
//! property, not any particular trading logic, is what this module proves.
//! The "signal" here is a placeholder EMA-cross rule, not the real
//! strategy VM (Phase 3 scope) — determinism is the point, not edge.

use domain::{SymbolId, Tick};
use features::FeatureEngine;
use market_data::BarAggregator;
use risk::sizing::{kelly_lots, KellyInputs};

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayDecision {
    pub bar_ts_open_ns: u64,
    pub bar_close: i64,
    pub ema_fast: Option<f64>,
    pub ema_slow: Option<f64>,
    /// `Some(lots)` when the placeholder EMA-cross rule fires; sizing uses
    /// a fixed calibrated probability so the run has no external inputs
    /// beyond the tick stream itself (P5: deterministic core).
    pub lots: Option<f64>,
}

pub fn run_deterministic_pipeline(ticks: &[Tick], symbol_id: SymbolId, timeframe_seconds: u32) -> Vec<ReplayDecision> {
    let mut bars = BarAggregator::new(symbol_id, timeframe_seconds);
    let mut feature_engine = FeatureEngine::new(3, 10);
    let mut decisions = Vec::new();

    for tick in ticks {
        let snapshot = feature_engine.on_tick(tick);
        if let Some(closed) = bars.on_tick(tick) {
            let lots = match (snapshot.ema_fast, snapshot.ema_slow) {
                (Some(fast), Some(slow)) if fast > slow => kelly_lots(KellyInputs {
                    probability: 0.58,
                    r_target: 2.2,
                    kappa: 0.25,
                    f_max: 0.02,
                    equity: 10_000.0,
                    risk_per_trade_pct: 0.005,
                    stop_distance: 30.0,
                    pip_value: 1.0,
                    contract_size: 1.0,
                })
                .ok(),
                _ => None,
            };
            decisions.push(ReplayDecision {
                bar_ts_open_ns: closed.ts_open_ns,
                bar_close: closed.close,
                ema_fast: snapshot.ema_fast,
                ema_slow: snapshot.ema_slow,
                lots,
            });
        }
    }
    decisions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(ts_ns: u64, bid: i64) -> Tick {
        Tick { ts_ns, recv_ns: ts_ns, symbol_id: 1, bid, ask: bid + 10, bid_volume: 1, ask_volume: 1, flags: 0 }
    }

    fn sample_ticks() -> Vec<Tick> {
        // Enough bars, with enough of a trend, to exercise both the "no
        // signal yet" (EMA warm-up) and "signal fires" branches.
        (0..300).map(|i| tick(i * 200_000_000, 100_000 + i as i64 * 5)).collect()
    }

    #[test]
    fn same_input_twice_produces_bit_identical_output() {
        let ticks = sample_ticks();
        let run1 = run_deterministic_pipeline(&ticks, 1, 1);
        let run2 = run_deterministic_pipeline(&ticks, 1, 1);
        assert_eq!(run1, run2);
        assert!(!run1.is_empty(), "sanity: the pipeline should close at least one bar");
    }

    #[test]
    fn a_rising_price_series_eventually_fires_the_placeholder_signal() {
        let decisions = run_deterministic_pipeline(&sample_ticks(), 1, 1);
        assert!(decisions.iter().any(|d| d.lots.is_some()), "fast EMA should cross above slow EMA on a steady uptrend");
    }
}
