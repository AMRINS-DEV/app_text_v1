//! Incremental (O(1)-per-update) indicators. §5.2's latency budget assumes
//! **no indicator is ever recomputed over a rolling window** — every one of
//! these carries just enough state to fold in the next price and emit an
//! updated value in constant time.
//!
//! This crate has zero dependencies on purpose: it is compiled twice — once
//! into the Rust core (`cdylib`/`rlib`) and once to WASM for
//! `packages/chart-engine`, so the chart can never show a different value
//! than the engine traded on.

mod atr;
mod ema;
mod rsi;

pub use atr::{Atr, OhlcInput};
pub use ema::Ema;
pub use rsi::Rsi;

/// Common shape for every incremental indicator in this crate.
pub trait Incremental {
    /// The value type folded in on each update (a price, a bar, etc).
    type Input;
    /// The emitted reading, e.g. `Option<f64>` while warming up.
    type Output;

    fn update(&mut self, input: Self::Input) -> Self::Output;
    fn value(&self) -> Self::Output;
}
