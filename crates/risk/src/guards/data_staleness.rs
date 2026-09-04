use super::{Guard, GuardOutcome};

/// §9.5: "No tick for symbol > 5s in an open session -> Mark symbol
/// untradeable; flatten if position open." One instance per symbol — the
/// trait has no symbol parameter, so the caller owns the per-symbol
/// fan-out (a `HashMap<SymbolId, DataStalenessGuard>` at the call site).
pub struct DataStalenessGuard {
    max_gap_ns: u64,
    last_tick_ns: u64,
    session_open: bool,
    tripped: bool,
}

impl DataStalenessGuard {
    pub fn new(max_gap_ns: u64) -> Self {
        Self { max_gap_ns, last_tick_ns: 0, session_open: false, tripped: false }
    }

    pub fn record_tick(&mut self, ts_ns: u64) {
        self.last_tick_ns = ts_ns;
    }

    pub fn set_session_open(&mut self, open: bool) {
        self.session_open = open;
    }

    /// The check needs "now" from the caller — this guard has no clock of
    /// its own (P5: no hidden state, no wall-clock reads inside risk logic).
    pub fn check_at(&mut self, now_ns: u64) -> GuardOutcome {
        if self.tripped {
            return GuardOutcome::BlockEntries;
        }
        if self.session_open && now_ns.saturating_sub(self.last_tick_ns) > self.max_gap_ns {
            self.tripped = true;
            return GuardOutcome::BlockEntries;
        }
        GuardOutcome::Pass
    }
}

impl Guard for DataStalenessGuard {
    fn name(&self) -> &'static str {
        "data_staleness"
    }

    /// Re-checks against the last time recorded via `record_tick`/`check_at`
    /// — prefer `check_at` when a fresh "now" is available, since this
    /// path can only ever confirm or extend an existing trip, never detect
    /// a *new* gap without being told the current time.
    fn evaluate(&mut self) -> GuardOutcome {
        if self.tripped {
            GuardOutcome::BlockEntries
        } else {
            GuardOutcome::Pass
        }
    }

    fn reset(&mut self) {
        self.tripped = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEC: u64 = 1_000_000_000;

    #[test]
    fn passes_while_ticks_keep_arriving_within_the_gap() {
        let mut g = DataStalenessGuard::new(5 * SEC);
        g.set_session_open(true);
        g.record_tick(0);
        assert_eq!(g.check_at(4 * SEC), GuardOutcome::Pass);
    }

    #[test]
    fn trips_after_the_gap_elapses_in_an_open_session() {
        let mut g = DataStalenessGuard::new(5 * SEC);
        g.set_session_open(true);
        g.record_tick(0);
        assert_eq!(g.check_at(6 * SEC), GuardOutcome::BlockEntries);
    }

    #[test]
    fn does_not_trip_outside_an_open_session() {
        let mut g = DataStalenessGuard::new(5 * SEC);
        g.set_session_open(false);
        g.record_tick(0);
        assert_eq!(g.check_at(100 * SEC), GuardOutcome::Pass);
    }

    #[test]
    fn a_fresh_tick_after_tripping_does_not_auto_clear_it() {
        let mut g = DataStalenessGuard::new(5 * SEC);
        g.set_session_open(true);
        g.record_tick(0);
        g.check_at(6 * SEC);
        g.record_tick(6 * SEC);
        assert_eq!(g.check_at(6 * SEC), GuardOutcome::BlockEntries);
        g.reset();
        assert_eq!(g.check_at(6 * SEC), GuardOutcome::Pass);
    }
}
