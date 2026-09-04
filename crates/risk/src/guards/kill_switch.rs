use super::{Guard, GuardOutcome};

/// §9.5: "Dashboard button / Telegram command / hardware key -> Immediate
/// flatten + halt, single atomic operation." The "atomic" part is enforced
/// by whoever calls `trigger()` and then actually flattens positions in
/// response to the resulting `HaltAndFlatten` — this guard's own job is
/// just to never forget it was pulled.
#[derive(Default)]
pub struct KillSwitchGuard {
    triggered: bool,
}

impl KillSwitchGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trigger(&mut self) {
        self.triggered = true;
    }
}

impl Guard for KillSwitchGuard {
    fn name(&self) -> &'static str {
        "kill_switch"
    }

    fn evaluate(&mut self) -> GuardOutcome {
        if self.triggered {
            GuardOutcome::HaltAndFlatten
        } else {
            GuardOutcome::Pass
        }
    }

    fn reset(&mut self) {
        self.triggered = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_until_triggered_then_latches() {
        let mut g = KillSwitchGuard::new();
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
        g.trigger();
        assert_eq!(g.evaluate(), GuardOutcome::HaltAndFlatten);
        assert_eq!(g.evaluate(), GuardOutcome::HaltAndFlatten, "stays halted without an explicit reset");
        g.reset();
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
    }
}
