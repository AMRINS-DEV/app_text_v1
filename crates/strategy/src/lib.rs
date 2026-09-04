//! Strategy VM. Config parsing (§5.5's YAML shape) is real and tested;
//! compiling that config to a <5µs decision tree and signal fusion (§8.4's
//! log-odds pooling) are Phase 3 stubs — they need the feature engine and
//! calibrated agent signals to be meaningful.

pub mod config;
pub mod fusion;

pub use config::StrategyConfig;
pub use fusion::{fuse, FusionInput};
