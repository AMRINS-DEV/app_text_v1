//! Incremental (amortized O(1)-per-update) indicators. §5.2's latency
//! budget assumes **no indicator is ever recomputed over a rolling window
//! from scratch** — every one of these carries just enough running state
//! (or a maintained ring buffer with running sums / monotonic deques) to
//! fold in the next price and emit an updated value without re-scanning
//! history.
//!
//! This crate has zero dependencies on purpose: it is compiled twice — once
//! into the Rust core (`cdylib`/`rlib`) and once to WASM for
//! `packages/chart-engine`, so the chart can never show a different value
//! than the engine traded on.
//!
//! Covers the volatility (ATR, Bollinger), trend (ADX, Efficiency Ratio),
//! momentum (RSI), and market-structure (Donchian, swing highs/lows)
//! families from §8.1's feature taxonomy. Not covered here: liquidity
//! levels, order-flow/microstructure, multi-timeframe alignment,
//! cross-asset, news, and positioning features — those need external data
//! (DOM, other instruments, news feeds, COT reports) this crate
//! deliberately has no access to; they're `crates/features`'/Phase 5-6
//! scope, not missing indicator math.

mod adx;
mod atr;
mod bollinger;
mod donchian;
mod efficiency_ratio;
mod ema;
mod ring;
mod rsi;
mod swing;
mod vwap;

pub use adx::Adx;
pub use atr::{Atr, OhlcInput};
pub use bollinger::{Bollinger, BollingerBands};
pub use donchian::{Donchian, DonchianChannel};
pub use efficiency_ratio::EfficiencyRatio;
pub use ema::Ema;
pub use rsi::Rsi;
pub use swing::{SwingDetector, SwingSignal};
pub use vwap::Vwap;

/// Common shape for every incremental indicator in this crate.
pub trait Incremental {
    /// The value type folded in on each update (a price, a bar, etc).
    type Input;
    /// The emitted reading, e.g. `Option<f64>` while warming up.
    type Output;

    fn update(&mut self, input: Self::Input) -> Self::Output;
    fn value(&self) -> Self::Output;
}
