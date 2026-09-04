//! Safety guards (§9.5). Every guard latches once tripped — `evaluate()`
//! keeps returning the tripped outcome until `reset()` is called explicitly,
//! regardless of whether the underlying condition has since cleared. This
//! is deliberate (P6: fail closed): a spread spike that subsides a second
//! later shouldn't silently un-halt a system that already decided to stop.
//!
//! Each guard owns whatever state it needs and exposes its own `record_*`
//! setter(s) — those aren't part of the shared trait because the guards
//! genuinely consume different inputs (equity, ticks, broker events...);
//! `Guard::evaluate` is the one thing they all have in common: "given what
//! you've been told, is it safe to keep trading?"

mod agent_unavailability;
mod clock_skew;
mod consecutive_losses;
mod daily_drawdown;
mod data_staleness;
mod kill_switch;
mod max_drawdown;
mod position_reconciliation;
mod reject_storm;
mod spread_spike;

pub use agent_unavailability::AgentUnavailabilityGuard;
pub use clock_skew::ClockSkewGuard;
pub use consecutive_losses::ConsecutiveLossesGuard;
pub use daily_drawdown::DailyDrawdownGuard;
pub use data_staleness::DataStalenessGuard;
pub use kill_switch::KillSwitchGuard;
pub use max_drawdown::MaxDrawdownGuard;
pub use position_reconciliation::PositionReconciliationGuard;
pub use reject_storm::RejectStormGuard;
pub use spread_spike::SpreadSpikeGuard;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuardOutcome {
    Pass,
    /// Fail closed (P6): flatten open positions and halt new entries.
    HaltAndFlatten,
    /// Fail closed, softer: block new entries but leave existing positions.
    BlockEntries,
    /// §9.5 "consecutive losses"/"agent unavailability": keep trading, but
    /// at reduced size.
    ReduceSize { multiplier_pct: u8 },
}

/// One safety guard from §9.5's table.
pub trait Guard: Send {
    fn name(&self) -> &'static str;
    fn evaluate(&mut self) -> GuardOutcome;
    /// Manual re-arm. A latched guard never clears itself.
    fn reset(&mut self);
}
