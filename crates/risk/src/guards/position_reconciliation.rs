use std::collections::HashMap;

use domain::{BrokerOrderId, PositionSnapshot};

use super::{Guard, GuardOutcome};

/// §9.5: "Every 5s: Compare local position book vs broker; any divergence
/// -> halt + alert." The doc calls this "the #1 cause of catastrophic algo
/// losses" — this guard trips on *any* difference (missing position,
/// extra position, or a quantity mismatch), never tries to guess which
/// side is "probably right".
pub struct PositionReconciliationGuard {
    tripped: bool,
}

impl PositionReconciliationGuard {
    pub fn new() -> Self {
        Self { tripped: false }
    }

    /// Real check — needs both books, so it isn't the parameterless
    /// `Guard::evaluate`. Called on the periodic (§9.5: 5s) reconciliation
    /// tick.
    pub fn check(&mut self, local: &[PositionSnapshot], broker: &[PositionSnapshot]) -> GuardOutcome {
        if self.tripped {
            return GuardOutcome::HaltAndFlatten;
        }
        let local_map: HashMap<BrokerOrderId, i64> = local.iter().map(|p| (p.broker_order_id, p.qty)).collect();
        let broker_map: HashMap<BrokerOrderId, i64> = broker.iter().map(|p| (p.broker_order_id, p.qty)).collect();
        if local_map != broker_map {
            self.tripped = true;
            return GuardOutcome::HaltAndFlatten;
        }
        GuardOutcome::Pass
    }
}

impl Default for PositionReconciliationGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Guard for PositionReconciliationGuard {
    fn name(&self) -> &'static str {
        "position_reconciliation"
    }

    /// Reports the last known state — see `check` for the actual comparison.
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

    fn pos(id: BrokerOrderId, qty: i64) -> PositionSnapshot {
        PositionSnapshot { broker_order_id: id, symbol_id: 1, qty, avg_price: 100_000 }
    }

    #[test]
    fn matching_books_pass() {
        let mut g = PositionReconciliationGuard::new();
        let local = vec![pos(1, 100), pos(2, 50)];
        let broker = vec![pos(2, 50), pos(1, 100)]; // order doesn't matter
        assert_eq!(g.check(&local, &broker), GuardOutcome::Pass);
    }

    #[test]
    fn a_quantity_mismatch_trips_it() {
        let mut g = PositionReconciliationGuard::new();
        let local = vec![pos(1, 100)];
        let broker = vec![pos(1, 99)];
        assert_eq!(g.check(&local, &broker), GuardOutcome::HaltAndFlatten);
    }

    #[test]
    fn a_position_missing_from_the_broker_side_trips_it() {
        let mut g = PositionReconciliationGuard::new();
        let local = vec![pos(1, 100)];
        let broker: Vec<PositionSnapshot> = vec![];
        assert_eq!(g.check(&local, &broker), GuardOutcome::HaltAndFlatten);
    }

    #[test]
    fn an_unexpected_extra_broker_position_trips_it() {
        let mut g = PositionReconciliationGuard::new();
        let local: Vec<PositionSnapshot> = vec![];
        let broker = vec![pos(1, 100)];
        assert_eq!(g.check(&local, &broker), GuardOutcome::HaltAndFlatten);
    }

    #[test]
    fn once_tripped_it_latches_even_if_the_books_later_agree() {
        let mut g = PositionReconciliationGuard::new();
        g.check(&[pos(1, 100)], &[]);
        assert_eq!(g.check(&[pos(1, 100)], &[pos(1, 100)]), GuardOutcome::HaltAndFlatten);
        g.reset();
        assert_eq!(g.check(&[pos(1, 100)], &[pos(1, 100)]), GuardOutcome::Pass);
    }
}
