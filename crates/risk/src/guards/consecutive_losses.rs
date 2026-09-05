use super::{Guard, GuardOutcome};

/// §9.5: "N losses in a row (default 5) -> Reduce size 50%, require 2 wins
/// to restore." Unlike the halt-style guards, this one auto-recovers on
/// its own defined condition (2 wins) rather than needing a manual reset —
/// that recovery path *is* the reset, so `reset()` is just the manual
/// override for e.g. an operator clearing it early.
pub struct ConsecutiveLossesGuard {
    threshold: u32,
    wins_required_to_restore: u32,
    consecutive_losses: u32,
    wins_since_reduction: u32,
    reduced: bool,
}

impl ConsecutiveLossesGuard {
    pub fn new(threshold: u32, wins_required_to_restore: u32) -> Self {
        Self { threshold, wins_required_to_restore, consecutive_losses: 0, wins_since_reduction: 0, reduced: false }
    }

    pub fn record_trade_result(&mut self, was_win: bool) {
        if was_win {
            self.consecutive_losses = 0;
            if self.reduced {
                self.wins_since_reduction += 1;
                if self.wins_since_reduction >= self.wins_required_to_restore {
                    self.reduced = false;
                    self.wins_since_reduction = 0;
                }
            }
        } else {
            self.consecutive_losses += 1;
            self.wins_since_reduction = 0;
            if self.consecutive_losses >= self.threshold {
                self.reduced = true;
            }
        }
    }
}

impl Guard for ConsecutiveLossesGuard {
    fn name(&self) -> &'static str {
        "consecutive_losses"
    }

    fn evaluate(&mut self) -> GuardOutcome {
        if self.reduced {
            GuardOutcome::ReduceSize { multiplier_pct: 50 }
        } else {
            GuardOutcome::Pass
        }
    }

    fn reset(&mut self) {
        self.reduced = false;
        self.consecutive_losses = 0;
        self.wins_since_reduction = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_below_the_loss_threshold() {
        let mut g = ConsecutiveLossesGuard::new(5, 2);
        for _ in 0..4 {
            g.record_trade_result(false);
        }
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
    }

    #[test]
    fn reduces_size_at_the_threshold() {
        let mut g = ConsecutiveLossesGuard::new(5, 2);
        for _ in 0..5 {
            g.record_trade_result(false);
        }
        assert_eq!(g.evaluate(), GuardOutcome::ReduceSize { multiplier_pct: 50 });
    }

    #[test]
    fn a_single_win_mid_streak_resets_the_loss_counter() {
        let mut g = ConsecutiveLossesGuard::new(5, 2);
        for _ in 0..4 {
            g.record_trade_result(false);
        }
        g.record_trade_result(true);
        for _ in 0..4 {
            g.record_trade_result(false);
        }
        assert_eq!(g.evaluate(), GuardOutcome::Pass, "streak was broken, never reached 5 in a row");
    }

    #[test]
    fn requires_exactly_two_wins_in_a_row_to_restore_full_size() {
        let mut g = ConsecutiveLossesGuard::new(5, 2);
        for _ in 0..5 {
            g.record_trade_result(false);
        }
        assert_eq!(g.evaluate(), GuardOutcome::ReduceSize { multiplier_pct: 50 });
        g.record_trade_result(true);
        assert_eq!(g.evaluate(), GuardOutcome::ReduceSize { multiplier_pct: 50 }, "only one win so far");
        g.record_trade_result(true);
        assert_eq!(g.evaluate(), GuardOutcome::Pass, "two wins in a row restores full size");
    }

    #[test]
    fn a_loss_between_the_two_restoring_wins_resets_the_win_counter() {
        let mut g = ConsecutiveLossesGuard::new(5, 2);
        for _ in 0..5 {
            g.record_trade_result(false);
        }
        g.record_trade_result(true);
        g.record_trade_result(false); // breaks the restore streak, but is itself only 1 consecutive loss
        g.record_trade_result(true);
        assert_eq!(g.evaluate(), GuardOutcome::ReduceSize { multiplier_pct: 50 }, "restore streak was broken");
    }
}
