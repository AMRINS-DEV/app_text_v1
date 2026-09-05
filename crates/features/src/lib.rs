//! Feature engine (§8.1, §5.1 "feature" stage). Assembles a real slice of
//! the §8.1 feature taxonomy from `crates/indicators` into one fixed-layout
//! snapshot: volatility (ATR, Bollinger), trend (EMA cross, ADX,
//! Efficiency Ratio), momentum (RSI), market structure (Donchian, swing
//! highs/lows), session (VWAP), and cost (spread) families.
//!
//! **Not implemented here** (§8.1 families needing data this crate has no
//! access to, not missing math): liquidity levels beyond Donchian
//! (volume profile POC/VAH/VAL needs a full order book), order-flow/
//! microstructure (needs DOM), multi-timeframe alignment (needs multiple
//! `FeatureEngine`s wired to different bar timeframes — an orchestration
//! concern, Phase 4+), cross-asset (needs other instruments' feeds), news
//! (needs the agent layer, §10), positioning (needs COT/broker-flow data).
//! SIMD batch computation (`wide`/`polars`) for backtesting many symbols at
//! once is also Phase 3+ scope beyond this single-symbol incremental path.
//!
//! Two update paths, matching how each underlying indicator actually
//! wants its input: `on_tick` for tick-driven features (EMA cross, spread,
//! VWAP), `on_bar_close` for bar-driven ones (everything else). Both
//! return the *same* snapshot type — the latest known value of every
//! feature, tick-driven or not.

use domain::{Bar, Tick};
use indicators::Atr as Atr_;
use indicators::{
    Adx, Bollinger, BollingerBands, Donchian, DonchianChannel, EfficiencyRatio, Ema, Incremental, OhlcInput, Rsi,
    SwingDetector, SwingSignal, Vwap,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct FeatureSnapshot {
    // Trend / momentum (tick-driven)
    pub ema_fast: Option<f64>,
    pub ema_slow: Option<f64>,
    // Cost (tick-driven)
    pub spread: i64,
    // Session (tick-driven, resets on `reset_session`)
    pub vwap: Option<f64>,
    // Volatility (bar-driven)
    pub atr: Option<f64>,
    pub bollinger: Option<BollingerBands>,
    // Trend (bar-driven)
    pub adx: Option<f64>,
    pub efficiency_ratio: Option<f64>,
    // Momentum (bar-driven)
    pub rsi: Option<f64>,
    // Market structure (bar-driven)
    pub donchian: Option<DonchianChannel>,
    pub swing: SwingSignal,
}

impl FeatureSnapshot {
    /// True once the EMA-cross features this crate's Phase 0/1 placeholder
    /// signal rule (`crates/replay::pipeline`) depends on are both warm.
    pub fn ema_cross_ready(&self) -> bool {
        self.ema_fast.is_some() && self.ema_slow.is_some()
    }
}

pub struct FeatureEngineConfig {
    pub ema_fast_period: usize,
    pub ema_slow_period: usize,
    pub atr_period: usize,
    pub bollinger_period: usize,
    pub bollinger_k: f64,
    pub adx_period: usize,
    pub efficiency_ratio_period: usize,
    pub rsi_period: usize,
    pub donchian_period: usize,
    pub swing_k: usize,
}

impl Default for FeatureEngineConfig {
    fn default() -> Self {
        Self {
            ema_fast_period: 3,
            ema_slow_period: 10,
            atr_period: 14,
            bollinger_period: 20,
            bollinger_k: 2.0,
            adx_period: 14,
            efficiency_ratio_period: 10,
            rsi_period: 14,
            donchian_period: 20,
            swing_k: 2,
        }
    }
}

pub struct FeatureEngine {
    ema_fast: Ema,
    ema_slow: Ema,
    atr: Atr_,
    bollinger: Bollinger,
    adx: Adx,
    efficiency_ratio: EfficiencyRatio,
    rsi: Rsi,
    donchian: Donchian,
    swing: SwingDetector,
    vwap: Vwap,
    snapshot: FeatureSnapshot,
}

impl FeatureEngine {
    pub fn new(config: FeatureEngineConfig) -> Self {
        Self {
            ema_fast: Ema::new(config.ema_fast_period),
            ema_slow: Ema::new(config.ema_slow_period),
            atr: Atr_::new(config.atr_period),
            bollinger: Bollinger::new(config.bollinger_period, config.bollinger_k),
            adx: Adx::new(config.adx_period),
            efficiency_ratio: EfficiencyRatio::new(config.efficiency_ratio_period),
            rsi: Rsi::new(config.rsi_period),
            donchian: Donchian::new(config.donchian_period),
            swing: SwingDetector::new(config.swing_k),
            vwap: Vwap::new(),
            snapshot: FeatureSnapshot::default(),
        }
    }

    /// Convenience constructor matching Phase 0/1's two-argument call
    /// sites (`tradeos-core`, `crates/replay::pipeline`) — same EMA
    /// periods, defaults for everything new.
    pub fn with_ema_periods(fast_period: usize, slow_period: usize) -> Self {
        Self::new(FeatureEngineConfig { ema_fast_period: fast_period, ema_slow_period: slow_period, ..Default::default() })
    }

    pub fn on_tick(&mut self, tick: &Tick) -> FeatureSnapshot {
        let mid = tick.mid() as f64;
        self.snapshot.ema_fast = self.ema_fast.update(mid);
        self.snapshot.ema_slow = self.ema_slow.update(mid);
        self.snapshot.spread = tick.spread();
        self.snapshot.vwap = self.vwap.update((mid, (tick.bid_volume + tick.ask_volume) as f64));
        self.snapshot
    }

    pub fn on_bar_close(&mut self, bar: &Bar) -> FeatureSnapshot {
        let ohlc = OhlcInput { high: bar.high as f64, low: bar.low as f64, close: bar.close as f64 };
        self.snapshot.atr = self.atr.update(ohlc);
        self.snapshot.bollinger = self.bollinger.update(bar.close as f64);
        self.snapshot.adx = self.adx.update(ohlc);
        self.snapshot.efficiency_ratio = self.efficiency_ratio.update(bar.close as f64);
        self.snapshot.rsi = self.rsi.update(bar.close as f64);
        self.snapshot.donchian = self.donchian.update((bar.high as f64, bar.low as f64));
        self.snapshot.swing = self.swing.update(ohlc);
        self.snapshot
    }

    /// New trading session (§8.1 "Session & time") — VWAP resets, nothing
    /// else does (ATR/ADX/etc. are multi-session by convention).
    pub fn reset_session(&mut self) {
        self.vwap.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(bid: i64, volume: u32) -> Tick {
        Tick { ts_ns: 0, recv_ns: 0, symbol_id: 1, bid, ask: bid + 10, bid_volume: volume, ask_volume: volume, flags: 0 }
    }

    fn bar(high: i64, low: i64, close: i64) -> Bar {
        Bar { symbol_id: 1, timeframe_seconds: 60, ts_open_ns: 0, open: (high + low) / 2, high, low, close, volume: 10 }
    }

    #[test]
    fn on_tick_emits_ema_spread_and_vwap() {
        let mut engine = FeatureEngine::with_ema_periods(3, 10);
        let snap = engine.on_tick(&tick(100_000, 5));
        assert_eq!(snap.ema_fast, Some(100_005.0));
        assert_eq!(snap.spread, 10);
        assert!(snap.vwap.is_some());
    }

    #[test]
    fn on_bar_close_eventually_emits_the_bar_driven_features() {
        let mut engine = FeatureEngine::new(FeatureEngineConfig {
            atr_period: 3,
            bollinger_period: 3,
            adx_period: 3,
            efficiency_ratio_period: 3,
            rsi_period: 3,
            donchian_period: 3,
            swing_k: 1,
            ..Default::default()
        });
        let mut last = FeatureSnapshot::default();
        for i in 0..10 {
            let base = 100_000 + i * 100;
            last = engine.on_bar_close(&bar(base + 50, base - 50, base));
        }
        assert!(last.atr.is_some());
        assert!(last.bollinger.is_some());
        assert!(last.donchian.is_some());
        assert!(last.rsi.is_some());
    }

    #[test]
    fn reset_session_clears_vwap_but_nothing_else() {
        let mut engine = FeatureEngine::with_ema_periods(3, 10);
        engine.on_tick(&tick(100_000, 5));
        engine.reset_session();
        let snap = engine.on_tick(&tick(50_000, 1));
        // VWAP restarted from this tick alone -> equals this tick's mid price.
        assert!((snap.vwap.unwrap() - 50_005.0).abs() < 1e-6);
        // EMA state (not session-scoped) is untouched by the reset.
        assert!(snap.ema_fast.is_some());
    }

    #[test]
    fn ema_cross_ready_reflects_warmup_state() {
        let mut engine = FeatureEngine::with_ema_periods(3, 10);
        let snap = engine.on_tick(&tick(100_000, 1));
        assert!(snap.ema_cross_ready(), "both EMAs seed on the first tick");
        assert!(!FeatureSnapshot::default().ema_cross_ready());
    }
}
