//! Safety guards (§9.5). Trait-shaped stub for Phase 2 — every guard listed
//! in the design doc's table must become a variant here and be unit-tested
//! against a simulated broker before this crate leaves "Phase 0" status.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardOutcome {
    Pass,
    /// Fail closed (P6): flatten open positions and halt new entries.
    HaltAndFlatten,
    /// Fail closed, softer: block new entries but leave existing positions.
    BlockEntries,
}

/// One safety guard from §9.5's table (daily drawdown, max drawdown,
/// consecutive losses, data staleness, clock skew, spread spike, reject
/// storm, agent unavailability, kill switch, position reconciliation).
pub trait Guard: Send {
    fn name(&self) -> &'static str;
    fn evaluate(&mut self) -> GuardOutcome;
}
