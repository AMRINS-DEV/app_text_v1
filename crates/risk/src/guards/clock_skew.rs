use super::{Guard, GuardOutcome};

/// §9.5: "Broker vs local time drift > 500ms -> Halt (indicates feed or
/// VPS problem)."
pub struct ClockSkewGuard {
    max_skew_ns: u64,
    tripped: bool,
}

impl ClockSkewGuard {
    pub fn new(max_skew_ns: u64) -> Self {
        Self { max_skew_ns, tripped: false }
    }

    pub fn record_skew(&mut self, broker_ns: u64, local_ns: u64) {
        if self.tripped {
            return;
        }
        let skew = broker_ns.abs_diff(local_ns);
        if skew > self.max_skew_ns {
            self.tripped = true;
        }
    }
}

impl Guard for ClockSkewGuard {
    fn name(&self) -> &'static str {
        "clock_skew"
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

    const MS: u64 = 1_000_000;

    #[test]
    fn passes_within_tolerance_in_either_direction() {
        let mut g = ClockSkewGuard::new(500 * MS);
        g.record_skew(1_000 * MS, 1_400 * MS); // broker behind local by 400ms
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
        g.record_skew(1_900 * MS, 1_500 * MS); // broker ahead of local by 400ms
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
    }

    #[test]
    fn trips_beyond_tolerance() {
        let mut g = ClockSkewGuard::new(500 * MS);
        g.record_skew(1_000 * MS, 1_600 * MS); // 600ms drift
        assert_eq!(g.evaluate(), GuardOutcome::HaltAndFlatten);
    }
}
