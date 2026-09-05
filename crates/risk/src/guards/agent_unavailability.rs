use super::{Guard, GuardOutcome};

/// §9.5: "LLM layer down -> Continue on model+rule signals only, at
/// reduced size." Trading doesn't stop when the agent layer does — §P2's
/// whole point is that agents are advisory, so this guard reduces size
/// rather than halting.
pub struct AgentUnavailabilityGuard {
    reduced_size_pct: u8,
    available: bool,
}

impl AgentUnavailabilityGuard {
    pub fn new(reduced_size_pct: u8) -> Self {
        Self { reduced_size_pct, available: true }
    }

    pub fn record_agent_health(&mut self, available: bool) {
        self.available = available;
    }
}

impl Guard for AgentUnavailabilityGuard {
    fn name(&self) -> &'static str {
        "agent_unavailability"
    }

    fn evaluate(&mut self) -> GuardOutcome {
        if self.available {
            GuardOutcome::Pass
        } else {
            GuardOutcome::ReduceSize { multiplier_pct: self.reduced_size_pct }
        }
    }

    fn reset(&mut self) {
        self.available = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_while_agents_are_healthy() {
        let mut g = AgentUnavailabilityGuard::new(50);
        assert_eq!(g.evaluate(), GuardOutcome::Pass);
    }

    #[test]
    fn reduces_size_when_agents_go_down_and_recovers_immediately_when_they_return() {
        let mut g = AgentUnavailabilityGuard::new(50);
        g.record_agent_health(false);
        assert_eq!(g.evaluate(), GuardOutcome::ReduceSize { multiplier_pct: 50 });
        g.record_agent_health(true);
        assert_eq!(g.evaluate(), GuardOutcome::Pass, "unlike halt-style guards, this tracks live agent health directly");
    }
}
