use super::{Guard, GuardOutcome};

/// §9.5: "Realized+unrealized loss > mode limit -> Flatten all, halt new
/// entries until next session."
pub struct DailyDrawdownGuard {
    day_start_equity: i64,
    limit_pct: f64,
    current_equity: i64,
    tripped: bool,
}

impl DailyDrawdownGuard {
    pub fn new(day_start_equity: i64, limit_pct: f64) -> Self {
        Self { day_start_equity, limit_pct, current_equity: day_start_equity, tripped: false }
    }

    /// Realized + unrealized equity right now.
    pub fn record_equity(&mut self, equity: i64) {
        self.current_equity = equity;
    }

    /// A new trading session (§9.1's session boundary) resets the
    /// reference point and clears the latch — this is the "until next
    /// session" half of the rule, distinct from a manual re-arm.
    pub fn start_new_session(&mut self, day_start_equity: i64) {
        self.day_start_equity = day_start_equity;
        self.current_equity = day_start_equity;
        self.tripped = false;
    }
}

impl Guard for DailyDrawdownGuard {
    fn name(&self) -> &'static str {
        "daily_drawdown"
    }

    fn evaluate(&mut self) -> GuardOutcome {
        if self.tripped {
            return GuardOutcome::HaltAndFlatten;
        }
        let loss_pct = 1.0 - (self.current_equity as f64 / self.day_start_equity as f64);
        if loss_pct > self.limit_pct {
            self.tripped = true;
            return GuardOutcome::HaltAndFlatten;
        }
        GuardOutcome::Pass
    }

    fn reset(&mut self) {
        self.tripped = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_under_the_limit() {
        let mut g = DailyDrawdownGuard::new(10_000, 0.02);
        g.record_equity(9_900); // 1% loss
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
    }

    #[test]
    fn trips_over_the_limit_and_latches() {
        let mut g = DailyDrawdownGuard::new(10_000, 0.02);
        g.record_equity(9_700); // 3% loss > 2% limit
        assert_eq!(g.evaluate(), GuardOutcome::HaltAndFlatten);
        // Recovering equity doesn't un-trip it — only a new session/reset does.
        g.record_equity(10_500);
        assert_eq!(g.evaluate(), GuardOutcome::HaltAndFlatten);
    }

    #[test]
    fn new_session_resets_the_reference_and_the_latch() {
        let mut g = DailyDrawdownGuard::new(10_000, 0.02);
        g.record_equity(9_700);
        assert_eq!(g.evaluate(), GuardOutcome::HaltAndFlatten);
        g.start_new_session(9_700);
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
    }
}
