//! Reconciliation soak test (§9.5, §17 Phase 2 exit criterion: "reconciliation
//! never diverges over 72h soak"). This environment can't run a literal 72
//! real hours; instead it drives many thousands of randomized order-lifecycle
//! cycles — submit/modify/partial-close/full-close, with a nonzero broker
//! reject rate — through the real `OrderRouter` + `SimBroker`, maintaining a
//! local position book the same way a real core would (purely from the
//! `ExecEvent`s the router hands back), and asserts reconciliation never
//! spuriously trips. A second test then deliberately corrupts the local book
//! and asserts the same guard reliably *does* catch it — a reconciliation
//! guard that can never trip is not proven safe, it's just untested.

use std::collections::HashMap;

use domain::ports::Broker;
use domain::{BrokerOrderId, ExecEvent, OrderIntent, OrderType, PositionSnapshot, Side, TimeInForce, TradingMode};
use execution::{OrderRouter, SimBroker, SimBrokerConfig};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use risk::{GuardOutcome, PositionReconciliationGuard};
use smallvec::SmallVec;

struct LocalBook {
    positions: HashMap<BrokerOrderId, PositionSnapshot>,
}

impl LocalBook {
    fn new() -> Self {
        Self { positions: HashMap::new() }
    }

    /// Updates purely from the event stream — exactly what a real core's
    /// position tracker would see, never by peeking at the broker double's
    /// internal state.
    fn apply(&mut self, event: &ExecEvent) {
        match *event {
            ExecEvent::Fill { broker_order_id, fill_price, qty, .. } => {
                self.positions
                    .entry(broker_order_id)
                    .and_modify(|p| p.qty += qty)
                    .or_insert(PositionSnapshot { broker_order_id, symbol_id: 1, qty, avg_price: fill_price });
            }
            ExecEvent::Modify { .. } | ExecEvent::Reject { .. } => {}
        }
    }

    fn apply_close(&mut self, id: BrokerOrderId, qty: Option<i64>) {
        match qty {
            Some(q) => {
                if let Some(p) = self.positions.get_mut(&id) {
                    p.qty -= q;
                    if p.qty <= 0 {
                        self.positions.remove(&id);
                    }
                }
            }
            None => {
                self.positions.remove(&id);
            }
        }
    }

    fn snapshot(&self) -> Vec<PositionSnapshot> {
        self.positions.values().copied().collect()
    }
}

fn intent(client_id: u128, qty: i64) -> OrderIntent {
    OrderIntent {
        client_id,
        symbol_id: 1,
        side: Side::Buy,
        qty,
        order_type: OrderType::Market,
        limit_px: None,
        sl: Some(99_000),
        tp: Some(101_000),
        tif: TimeInForce::Gtc,
        mode: TradingMode::Normal,
        max_slippage_pts: 5,
        signal_ids: SmallVec::new(),
    }
}

#[test]
fn reconciliation_never_spuriously_diverges_across_thousands_of_randomized_cycles() {
    let mut router = OrderRouter::new(SimBroker::from_seed(
        SimBrokerConfig { reject_probability: 0.05, partial_fill_probability: 0.2, requote_probability: 0.1, ..Default::default() },
        1234,
    ));
    let mut book = LocalBook::new();
    let mut guard = PositionReconciliationGuard::new();
    let mut rng = StdRng::seed_from_u64(99);
    let mut open_ids: Vec<BrokerOrderId> = Vec::new();
    let mut next_client_id: u128 = 1;

    const CYCLES: usize = 20_000;
    for i in 0..CYCLES {
        let action = rng.random_range(0..3);
        match action {
            0 => {
                // Submit a new order.
                let qty = rng.random_range(1..100);
                if let Ok(id) = router.submit(&intent(next_client_id, qty)) {
                    next_client_id += 1;
                    open_ids.push(id);
                }
                while let Some(event) = router.poll_event() {
                    book.apply(&event);
                }
            }
            1 if !open_ids.is_empty() => {
                // Fully close a random open position.
                let idx = rng.random_range(0..open_ids.len());
                let id = open_ids.remove(idx);
                let _ = router.close(id, None);
                book.apply_close(id, None);
            }
            2 if !open_ids.is_empty() => {
                // Partially close a random open position.
                let idx = rng.random_range(0..open_ids.len());
                let id = open_ids[idx];
                if let Some(pos) = book.positions.get(&id) {
                    let close_qty = (pos.qty / 2).max(1);
                    let _ = router.close(id, Some(close_qty));
                    book.apply_close(id, Some(close_qty));
                    if !book.positions.contains_key(&id) {
                        open_ids.remove(idx);
                    }
                }
            }
            _ => {}
        }

        // §9.5's own cadence is "every 5s" — here, every cycle, since there's
        // no wall clock in this test and more frequent checking is strictly
        // stronger evidence of no divergence, not weaker.
        let outcome = guard.check(&book.snapshot(), &router.broker().positions().unwrap());
        assert_eq!(outcome, GuardOutcome::Pass, "spurious reconciliation divergence at cycle {i}");
    }
}

#[test]
fn reconciliation_reliably_catches_a_genuinely_corrupted_local_book() {
    let mut router = OrderRouter::new(SimBroker::new(SimBrokerConfig::default()));
    let mut book = LocalBook::new();
    let mut guard = PositionReconciliationGuard::new();

    let id = router.submit(&intent(1, 100)).unwrap();
    while let Some(event) = router.poll_event() {
        book.apply(&event);
    }
    assert_eq!(guard.check(&book.snapshot(), &router.broker().positions().unwrap()), GuardOutcome::Pass);

    // Corrupt the local book the way a real bug would: silently drop a fill
    // (e.g. a missed event) so the local view understates the true position.
    book.positions.get_mut(&id).unwrap().qty -= 1;

    assert_eq!(
        guard.check(&book.snapshot(), &router.broker().positions().unwrap()),
        GuardOutcome::HaltAndFlatten,
        "a genuine divergence must trip the guard, not pass silently"
    );
}
