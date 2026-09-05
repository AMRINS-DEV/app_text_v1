//! Strategy VM. Config parsing (§5.5's YAML shape) is real and tested;
//! ONNX inference (§3.2, §17 Phase 3) is real and parity-tested against the
//! Python-trained reference model; signal fusion (§8.4, §17 Phase 6) is
//! real log-odds pooling with online Brier-score weights and an empirical
//! correlation penalty. Compiling a config into a <5µs decision tree
//! remains later-phase scope — it needs a live feature engine to be
//! meaningful.

pub mod config;
pub mod fusion;
pub mod onnx_model;

pub use config::StrategyConfig;
pub use fusion::{
    fuse, BrierTracker, Correlations, FusionInput, PairwiseCorrelationTracker, DEFAULT_LAMBDA,
    SAMPLE_SIZE_GATE,
};
pub use onnx_model::{OnnxClassifier, OnnxError};
