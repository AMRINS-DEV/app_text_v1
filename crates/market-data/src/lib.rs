//! Ingest-stage logic: bar aggregation is implemented for real (it's pure,
//! deterministic folding — the same code the ingest thread runs in
//! production). Book/DOM state and the symbol-interning table are stubs for
//! Phase 1, since they need a live feed to build meaningfully.

pub mod bar_aggregator;

pub use bar_aggregator::BarAggregator;
