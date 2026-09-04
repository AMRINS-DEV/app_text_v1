//! TradeOS core engine entry point.
//!
//! Phase 0 status: this drives simulated ticks through the real
//! ingest -> feature -> sizing crates to prove the pipeline's boundaries
//! compile and compose. It intentionally does **not** yet implement:
//! - the thread-per-core pinned pipeline with `rtrb` SPSC rings (§5.1),
//! - a live `MarketDataSource`/`Broker` (needs the MT5 bridge, Phase 1),
//! - the strategy VM's decision tree or ONNX inference (Phase 3).
//!
//! Those land in the phases the design doc's own roadmap (§17) lays out:
//! bridge -> core -> risk -> validation -> agents.

use domain::Tick;
use features::FeatureEngine;
use market_data::BarAggregator;
use risk::sizing::{kelly_lots, KellyInputs};
use strategy::StrategyConfig;

const DEMO_STRATEGY_YAML: &str = include_str!("../../../../packages/schemas/strategies/london_breakout_v3.yaml");

fn simulated_ticks() -> Vec<Tick> {
    // A short, deterministic synthetic tick sequence — replaced by the MT5
    // ZMQ feed once `adapter-mt5` (Phase 1) is implemented.
    (0..20)
        .map(|i| {
            let drift = i as i64 * 15;
            Tick {
                ts_ns: i as u64 * 200_000_000,
                recv_ns: i as u64 * 200_000_000 + 5_000,
                symbol_id: 1,
                bid: 100_000 + drift,
                ask: 100_010 + drift,
                bid_volume: 3,
                ask_volume: 2,
                flags: 0,
            }
        })
        .collect()
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("tradeos-core starting (Phase 0 scaffold — see README.md for scope)");

    let cfg = StrategyConfig::from_yaml(DEMO_STRATEGY_YAML).expect("demo strategy config must parse");
    tracing::info!(strategy_id = %cfg.id, "loaded strategy config");

    let mut bars = BarAggregator::new(1, 1); // 1-second bars for the demo
    let mut features = FeatureEngine::new(3, 10);

    for tick in simulated_ticks() {
        if let Some(closed) = bars.on_tick(&tick) {
            tracing::info!(?closed, "bar closed");
        }
        let snapshot = features.on_tick(&tick);
        tracing::debug!(?snapshot, "feature snapshot");
    }

    // Proves risk sizing wiring against a plausible calibrated signal — not
    // a real fused probability (Phase 3/5 scope).
    let lots = kelly_lots(KellyInputs {
        probability: 0.58,
        r_target: 2.2,
        kappa: cfg.sizing.kelly_fraction,
        f_max: 0.02,
        equity: 10_000.0,
        risk_per_trade_pct: cfg.sizing.risk_per_trade_pct / 100.0,
        stop_distance: 30.0,
        pip_value: 1.0,
        contract_size: 1.0,
    })
    .expect("demo sizing inputs are valid");
    tracing::info!(lots, "demo quarter-Kelly sizing result");

    tracing::info!("tradeos-core demo run complete");
}
