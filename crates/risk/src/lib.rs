//! Risk engine. `sizing` is implemented for real (§9.2 — it's pure math);
//! stop/trailing/guards (§9.3, §9.5) are trait-shaped stubs for Phase 2.

pub mod guards;
pub mod sizing;

pub use guards::{Guard, GuardOutcome};
pub use sizing::{kelly_lots, KellyInputs, SizingError};
