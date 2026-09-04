use std::collections::VecDeque;

use super::{Guard, GuardOutcome};

/// §9.5: "Spread > 3× 20-period median -> Block entries; widen trailing."
/// Widening trailing stops is an execution-side concern this guard doesn't
/// own; it only ever reports whether entries should be blocked.
pub struct SpreadSpikeGuard {
    window: VecDeque<i64>,
    window_len: usize,
    multiplier: f64,
    tripped: bool,
}

impl SpreadSpikeGuard {
    pub fn new(window_len: usize, multiplier: f64) -> Self {
        Self { window: VecDeque::with_capacity(window_len), window_len, multiplier, tripped: false }
    }

    fn median(&self) -> Option<i64> {
        if self.window.is_empty() {
            return None;
        }
        let mut sorted: Vec<i64> = self.window.iter().copied().collect();
        sorted.sort_unstable();
        Some(sorted[sorted.len() / 2])
    }

    /// Feeds one spread reading in and checks it against the median of the
    /// preceding window (the new reading is not counted in its own
    /// baseline, so a single spike can actually be detected as one).
    pub fn record_spread(&mut self, spread: i64) {
        if self.tripped {
            return;
        }
        if let Some(median) = self.median() {
            if median > 0 && spread as f64 > median as f64 * self.multiplier {
                self.tripped = true;
            }
        }
        if self.window.len() == self.window_len {
            self.window.pop_front();
        }
        self.window.push_back(spread);
    }
}

impl Guard for SpreadSpikeGuard {
    fn name(&self) -> &'static str {
        "spread_spike"
    }

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

    #[test]
    fn passes_while_the_baseline_is_still_filling() {
        let mut g = SpreadSpikeGuard::new(20, 3.0);
        for _ in 0..5 {
            g.record_spread(10);
        }
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
    }

    #[test]
    fn passes_normal_variation_around_the_median() {
        let mut g = SpreadSpikeGuard::new(20, 3.0);
        for s in [10, 11, 9, 10, 12, 9, 10, 11, 10, 10, 9, 11, 10, 10, 12, 9, 10, 11, 10, 10] {
            g.record_spread(s);
        }
        g.record_spread(15); // 1.5x median, not a spike
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
    }

    #[test]
    fn trips_on_a_spike_over_the_multiplier() {
        let mut g = SpreadSpikeGuard::new(20, 3.0);
        for _ in 0..20 {
            g.record_spread(10); // median = 10
        }
        g.record_spread(35); // 3.5x median
        assert_eq!(g.evaluate(), GuardOutcome::BlockEntries);
    }
}
