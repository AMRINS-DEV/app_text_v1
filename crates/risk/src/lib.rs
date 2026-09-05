//! Risk engine. `sizing` (§9.2) and `exits` (§9.3) are pure math, real and
//! unit-tested. `guards` (§9.5) is real as of Phase 2. `quick_profit` (§9.4)
//! is real, including its mandated shadow A/B accounting.

pub mod exits;
pub mod guards;
pub mod quick_profit;
pub mod sizing;

pub use exits::{breakeven_stop, chandelier_stop, is_time_stop_triggered, ratchet_stop, stop_distance, stop_price, target_price};
pub use guards::{
    AgentUnavailabilityGuard, ClockSkewGuard, ConsecutiveLossesGuard, DailyDrawdownGuard, DataStalenessGuard, Guard,
    GuardOutcome, KillSwitchGuard, MaxDrawdownGuard, PositionReconciliationGuard, RejectStormGuard, SpreadSpikeGuard,
};
pub use quick_profit::{QuickProfitConfig, QuickProfitTracker, ShadowOutcome};
pub use sizing::{kelly_lots, KellyInputs, SizingError};
