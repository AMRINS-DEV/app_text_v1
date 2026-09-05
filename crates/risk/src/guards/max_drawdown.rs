use super::{Guard, GuardOutcome};

/// §9.5: "Equity < peak × (1 − 10%) -> Halt, require manual re-arm from
/// dashboard." Unlike the daily guard, this one is deliberately *not*
/// cleared by a new session — a 10% peak-to-trough drawdown is a
/// standing-capital event, not a daily one.
pub struct MaxDrawdownGuard {
    peak_equity: i64,
    limit_pct: f64,
    tripped: bool,
}

impl MaxDrawdownGuard {
    pub fn new(initial_equity: i64, limit_pct: f64) -> Self {
        Self { peak_equity: initial_equity, limit_pct, tripped: false }
    }

    pub fn record_equity(&mut self, equity: i64) {
        self.peak_equity = self.peak_equity.max(equity);
        if !self.tripped {
            let drawdown_pct = 1.0 - (equity as f64 / self.peak_equity as f64);
            if drawdown_pct > self.limit_pct {
                self.tripped = true;
            }
        }
    }
}

impl Guard for MaxDrawdownGuard {
    fn name(&self) -> &'static str {
        "max_drawdown"
    }

    fn evaluate(&mut self) -> GuardOutcome {
        if self.tripped {
            GuardOutcome::HaltAndFlatten
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

    #[test]
    fn tracks_a_rising_peak_and_measures_drawdown_from_it() {
        let mut g = MaxDrawdownGuard::new(10_000, 0.10);
        g.record_equity(12_000); // new peak
        g.record_equity(11_000); // ~8.3% off peak, under 10%
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
        g.record_equity(10_700); // ~10.8% off the 12,000 peak
        assert_eq!(g.evaluate(), GuardOutcome::HaltAndFlatten);
    }

    #[test]
    fn only_explicit_reset_clears_it_not_equity_recovery() {
        let mut g = MaxDrawdownGuard::new(10_000, 0.10);
        g.record_equity(8_000);
        assert_eq!(g.evaluate(), GuardOutcome::HaltAndFlatten);
        g.record_equity(20_000); // fully recovered and then some
        assert_eq!(g.evaluate(), GuardOutcome::HaltAndFlatten, "requires manual re-arm (§9.5)");
        g.reset();
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
    }
}
