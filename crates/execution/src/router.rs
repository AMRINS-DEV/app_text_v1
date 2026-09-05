//! Order router (§5.1 exec stage, §9.3). The only thing allowed to call
//! `Broker::submit` — the strategy VM and every agent produce, at most, an
//! `OrderIntent` for `crates/risk`'s expectancy gate to approve; only this
//! router turns an approved one into a real broker call.

use std::collections::HashMap;

use domain::ports::{Broker, PortError, Result};
use domain::{BrokerOrderId, ExecEvent, OrderIntent};

pub struct OrderRouter<B: Broker> {
    broker: B,
    /// `client_id` -> the broker order id it successfully produced.
    /// Doubling as the idempotency ledger: presence here means "already
    /// placed, don't place again" — a client_id that merely *attempted*
    /// and got broker-rejected is intentionally absent, so a genuine retry
    /// (e.g. after a transient reject) can still go through the broker
    /// again rather than being permanently locked out by its own failure.
    submitted: HashMap<u128, BrokerOrderId>,
}

impl<B: Broker> OrderRouter<B> {
    pub fn new(broker: B) -> Self {
        Self { broker, submitted: HashMap::new() }
    }

    /// Submits `intent`, enforcing two invariants no caller should be able
    /// to violate even by accident:
    /// - **atomic SL/TP** (§9.3: "SL is always set atomically with entry
    ///   ... never as a follow-up modify" — a follow-up can fail, leaving a
    ///   naked position, so this router refuses to even try);
    /// - **idempotent resubmission** (a `client_id` that already succeeded
    ///   returns the original result instead of placing a second order; a
    ///   `client_id` that previously failed is free to retry).
    pub fn submit(&mut self, intent: &OrderIntent) -> Result<BrokerOrderId> {
        if let Some(&existing) = self.submitted.get(&intent.client_id) {
            return Ok(existing);
        }
        if intent.sl.is_none() || intent.tp.is_none() {
            return Err(PortError::Adapter("OrderIntent must carry both sl and tp at submission (§9.3)".into()));
        }

        let result = self.broker.submit(intent);
        if let Ok(broker_order_id) = result {
            self.submitted.insert(intent.client_id, broker_order_id);
        }
        result
    }

    pub fn modify(&mut self, id: BrokerOrderId, sl: Option<i64>, tp: Option<i64>) -> Result<()> {
        self.broker.modify(id, sl, tp)
    }

    pub fn close(&mut self, id: BrokerOrderId, qty: Option<i64>) -> Result<()> {
        self.broker.close(id, qty)
    }

    pub fn poll_event(&mut self) -> Option<ExecEvent> {
        self.broker.poll_event()
    }

    pub fn broker(&self) -> &B {
        &self.broker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_broker::{SimBroker, SimBrokerConfig};
    use domain::{OrderType, Side, TimeInForce, TradingMode};
    use smallvec::SmallVec;

    fn intent(client_id: u128, sl: Option<i64>, tp: Option<i64>) -> OrderIntent {
        OrderIntent {
            client_id,
            symbol_id: 1,
            side: Side::Buy,
            qty: 10,
            order_type: OrderType::Market,
            limit_px: None,
            sl,
            tp,
            tif: TimeInForce::Gtc,
            mode: TradingMode::Normal,
            max_slippage_pts: 5,
            signal_ids: SmallVec::new(),
        }
    }

    #[test]
    fn rejects_orders_missing_sl_or_tp() {
        let mut router = OrderRouter::new(SimBroker::new(SimBrokerConfig::default()));
        assert!(router.submit(&intent(1, None, Some(101_000))).is_err());
        assert!(router.submit(&intent(2, Some(99_000), None)).is_err());
    }

    #[test]
    fn resubmitting_the_same_client_id_returns_the_original_result_not_a_new_order() {
        let mut router = OrderRouter::new(SimBroker::new(SimBrokerConfig::default()));
        let first = router.submit(&intent(1, Some(99_000), Some(101_000))).unwrap();
        let second = router.submit(&intent(1, Some(99_000), Some(101_000))).unwrap();
        assert_eq!(first, second);
        assert_eq!(router.broker().positions().unwrap().len(), 1, "must not have placed a second order");
    }

    #[test]
    fn a_client_id_that_failed_can_be_retried_and_succeed() {
        let mut broker = SimBroker::new(SimBrokerConfig::default());
        broker.force_next_rejects(1);
        let mut router = OrderRouter::new(broker);
        assert!(router.submit(&intent(1, Some(99_000), Some(101_000))).is_err());
        let retried = router.submit(&intent(1, Some(99_000), Some(101_000)));
        assert!(retried.is_ok(), "a client_id that failed must be retriable, not permanently locked out");
        assert_eq!(router.broker().positions().unwrap().len(), 1);
    }
}
