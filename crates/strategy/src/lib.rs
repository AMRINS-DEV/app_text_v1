//! Strategy VM. Config parsing (§5.5's YAML shape) is real and tested;
//! ONNX inference (§3.2, §17 Phase 3) is real and parity-tested against the
//! Python-trained reference model. Compiling a config into a <5µs decision
//! tree and signal fusion's real log-odds pooling (§8.4, beyond the
//! placeholder weighted average) remain later-phase scope — they need a
//! live feature engine and calibrated agent signals to be meaningful.

pub mod config;
pub mod fusion;
pub mod onnx_model;

pub use config::StrategyConfig;
pub use fusion::{fuse, FusionInput};
pub use onnx_model::{OnnxClassifier, OnnxError};
