use std::collections::VecDeque;

use super::{Guard, GuardOutcome};

/// §9.5: ">3 rejects in 60s -> Circuit-break that symbol for 5 min." One
/// instance per symbol, same as `DataStalenessGuard`. Unlike the halt-style
/// guards, this one auto-clears after its cooldown — a circuit breaker,
/// not a latch — so `check_at` (which knows "now") is the accurate check;
/// `evaluate` is a conservative fallback for callers with no fresh
/// timestamp to hand.
pub struct RejectStormGuard {
    window_ns: u64,
    threshold: usize,
    cooldown_ns: u64,
    reject_timestamps: VecDeque<u64>,
    blocked_until_ns: Option<u64>,
}

impl RejectStormGuard {
    pub fn new(window_ns: u64, threshold: usize, cooldown_ns: u64) -> Self {
        Self { window_ns, threshold, cooldown_ns, reject_timestamps: VecDeque::new(), blocked_until_ns: None }
    }

    pub fn record_reject(&mut self, now_ns: u64) {
        self.reject_timestamps.push_back(now_ns);
        while let Some(&front) = self.reject_timestamps.front() {
            if now_ns.saturating_sub(front) > self.window_ns {
                self.reject_timestamps.pop_front();
            } else {
                break;
            }
        }
        if self.reject_timestamps.len() > self.threshold {
            self.blocked_until_ns = Some(now_ns + self.cooldown_ns);
        }
    }

    pub fn check_at(&mut self, now_ns: u64) -> GuardOutcome {
        match self.blocked_until_ns {
            Some(until) if now_ns < until => GuardOutcome::BlockEntries,
            Some(_) => {
                self.blocked_until_ns = None;
                GuardOutcome::Pass
            }
            None => GuardOutcome::Pass,
        }
    }
}

impl Guard for RejectStormGuard {
    fn name(&self) -> &'static str {
        "reject_storm"
    }

    fn evaluate(&mut self) -> GuardOutcome {
        if self.blocked_until_ns.is_some() {
            GuardOutcome::BlockEntries
        } else {
            GuardOutcome::Pass
        }
    }

    fn reset(&mut self) {
        self.blocked_until_ns = None;
        self.reject_timestamps.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEC: u64 = 1_000_000_000;

    #[test]
    fn passes_at_or_below_the_threshold() {
        let mut g = RejectStormGuard::new(60 * SEC, 3, 5 * 60 * SEC);
        for t in [0, 10, 20] {
            g.record_reject(t * SEC);
        }
        assert_eq!(g.check_at(21 * SEC), GuardOutcome::Pass);
    }

    #[test]
    fn trips_above_the_threshold_within_the_window() {
        let mut g = RejectStormGuard::new(60 * SEC, 3, 5 * 60 * SEC);
        for t in [0, 10, 20, 30] {
            g.record_reject(t * SEC);
        }
        assert_eq!(g.check_at(31 * SEC), GuardOutcome::BlockEntries);
    }

    #[test]
    fn old_rejects_outside_the_window_do_not_count() {
        let mut g = RejectStormGuard::new(60 * SEC, 3, 5 * 60 * SEC);
        g.record_reject(0);
        g.record_reject(70 * SEC); // first reject is now outside the 60s window
        g.record_reject(75 * SEC);
        g.record_reject(80 * SEC);
        // Only 3 rejects (70,75,80) inside the window relative to 80s -> at threshold, not above.
        assert_eq!(g.check_at(81 * SEC), GuardOutcome::Pass);
    }

    #[test]
    fn auto_clears_after_the_cooldown() {
        let mut g = RejectStormGuard::new(60 * SEC, 3, 5 * 60 * SEC);
        for t in [0, 10, 20, 30] {
            g.record_reject(t * SEC);
        }
        assert_eq!(g.check_at(31 * SEC), GuardOutcome::BlockEntries);
        assert_eq!(g.check_at(30 * SEC + 5 * 60 * SEC + 1), GuardOutcome::Pass, "cooldown elapsed");
    }
}
