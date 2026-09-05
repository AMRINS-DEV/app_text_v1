//! Multi-account support (§17 Phase 9 scope: "Multi-account"). One
//! `OrderRouter` per account, keyed by `AccountId` — each account gets its
//! own idempotency ledger (a `client_id` colliding across two accounts is
//! two different orders, never confused, since each account's `OrderRouter`
//! owns a separate map) and its own broker connection. Nothing about §9.5's
//! own safety guards changes: they already operate per-position/per-order
//! against whichever `Broker` they're pointed at, so running N of them
//! through this dispatch layer is not a new code path for them, just a new
//! caller. `OrderRouter<B>` was already generic over any `B: Broker` — this
//! module adds account-id dispatch around it, not a second implementation
//! of order routing.
//!
//! `domain::ports::Broker` is object-safe and now has a `Box<dyn Broker>`
//! forwarding impl (see `domain::ports`'s own doc comment), so
//! `AccountManager<Box<dyn Broker>>` can hold accounts on different broker
//! platforms side by side — a concrete extension of Phase 8's "same
//! strategy runs on 2 adapters" claim to "at the same time."

use std::collections::HashMap;

use domain::ports::{Broker, PortError, Result};
use domain::{AccountSnapshot, BrokerOrderId, ExecEvent, OrderIntent};

use crate::router::OrderRouter;

pub type AccountId = u32;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AggregateSnapshot {
    pub equity: i64,
    pub balance: i64,
    pub free_margin: i64,
    pub n_accounts: usize,
}

#[derive(Default)]
pub struct AccountManager<B: Broker> {
    routers: HashMap<AccountId, OrderRouter<B>>,
}

impl<B: Broker> AccountManager<B> {
    pub fn new() -> Self {
        Self { routers: HashMap::new() }
    }

    /// Registers a new account. Refuses to silently overwrite an existing
    /// `id` — clobbering a live account's idempotency ledger by accident is
    /// exactly the "trust last-known-good state" failure mode §18's own
    /// conflicts table rules out project-wide.
    pub fn add_account(&mut self, id: AccountId, broker: B) -> Result<()> {
        if self.routers.contains_key(&id) {
            return Err(PortError::Adapter(format!("account {id} is already registered")));
        }
        self.routers.insert(id, OrderRouter::new(broker));
        Ok(())
    }

    pub fn remove_account(&mut self, id: AccountId) -> Option<OrderRouter<B>> {
        self.routers.remove(&id)
    }

    pub fn account_ids(&self) -> impl Iterator<Item = AccountId> + '_ {
        self.routers.keys().copied()
    }

    fn router_mut(&mut self, id: AccountId) -> Result<&mut OrderRouter<B>> {
        self.routers.get_mut(&id).ok_or_else(|| PortError::Adapter(format!("unknown account {id}")))
    }

    pub fn submit(&mut self, account: AccountId, intent: &OrderIntent) -> Result<BrokerOrderId> {
        self.router_mut(account)?.submit(intent)
    }

    pub fn modify(&mut self, account: AccountId, id: BrokerOrderId, sl: Option<i64>, tp: Option<i64>) -> Result<()> {
        self.router_mut(account)?.modify(id, sl, tp)
    }

    pub fn close(&mut self, account: AccountId, id: BrokerOrderId, qty: Option<i64>) -> Result<()> {
        self.router_mut(account)?.close(id, qty)
    }

    pub fn poll_event(&mut self, account: AccountId) -> Option<ExecEvent> {
        self.routers.get_mut(&account)?.poll_event()
    }

    /// Drains one pending event per account per call (matches
    /// `Broker::poll_event`'s own non-blocking, one-at-a-time shape) —
    /// a caller wanting every currently pending event across every account
    /// loops this until it returns an empty `Vec`.
    pub fn poll_all_events(&mut self) -> Vec<(AccountId, ExecEvent)> {
        self.routers.iter_mut().filter_map(|(id, router)| router.poll_event().map(|event| (*id, event))).collect()
    }

    pub fn account_snapshot(&self, account: AccountId) -> Option<AccountSnapshot> {
        self.routers.get(&account).map(|router| router.broker().account())
    }

    /// Real aggregation, not a placeholder: sums equity/balance/free_margin
    /// across every registered account's own broker snapshot — the input a
    /// portfolio-level view (§17 Phase 9's own "portfolio optimizer" scope
    /// item) needs to start from.
    pub fn aggregate_snapshot(&self) -> AggregateSnapshot {
        self.routers.values().map(|r| r.broker().account()).fold(
            AggregateSnapshot { n_accounts: self.routers.len(), ..Default::default() },
            |mut acc, snap| {
                acc.equity += snap.equity;
                acc.balance += snap.balance;
                acc.free_margin += snap.free_margin;
                acc
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_broker::{SimBroker, SimBrokerConfig};
    use domain::{OrderType, Side, TimeInForce, TradingMode};
    use smallvec::SmallVec;

    fn intent(client_id: u128) -> OrderIntent {
        OrderIntent {
            client_id,
            symbol_id: 1,
            side: Side::Buy,
            qty: 10,
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

    fn manager_with_two_accounts() -> AccountManager<SimBroker> {
        let mut manager = AccountManager::new();
        manager.add_account(1, SimBroker::from_seed(SimBrokerConfig::default(), 1)).unwrap();
        manager.add_account(2, SimBroker::from_seed(SimBrokerConfig::default(), 2)).unwrap();
        manager
    }

    #[test]
    fn adding_a_duplicate_account_id_is_rejected() {
        let mut manager = manager_with_two_accounts();
        assert!(manager.add_account(1, SimBroker::new(SimBrokerConfig::default())).is_err());
    }

    #[test]
    fn submitting_to_an_unknown_account_is_rejected() {
        let mut manager = manager_with_two_accounts();
        assert!(manager.submit(999, &intent(1)).is_err());
    }

    #[test]
    fn the_same_client_id_on_two_accounts_produces_two_independent_orders() {
        let mut manager = manager_with_two_accounts();
        let a = manager.submit(1, &intent(1)).unwrap();
        let b = manager.submit(2, &intent(1)).unwrap();
        // Each account's own OrderRouter starts its broker_order_id
        // sequence at 1 independently — both succeeding, rather than the
        // second being treated as an idempotent resubmission of the
        // first, is exactly the account isolation this module exists for.
        assert_eq!(a, 1);
        assert_eq!(b, 1);
        assert_eq!(manager.account_snapshot(1).unwrap().equity, manager.account_snapshot(2).unwrap().equity);
    }

    #[test]
    fn resubmitting_the_same_client_id_on_the_same_account_is_still_idempotent() {
        let mut manager = manager_with_two_accounts();
        let first = manager.submit(1, &intent(1)).unwrap();
        let second = manager.submit(1, &intent(1)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn aggregate_snapshot_sums_every_registered_accounts_equity() {
        let manager = manager_with_two_accounts();
        let per_account_equity = manager.account_snapshot(1).unwrap().equity;
        let aggregate = manager.aggregate_snapshot();
        assert_eq!(aggregate.n_accounts, 2);
        assert_eq!(aggregate.equity, per_account_equity * 2);
    }

    #[test]
    fn poll_all_events_collects_one_event_per_account_with_a_pending_fill() {
        let mut manager = manager_with_two_accounts();
        manager.submit(1, &intent(1)).unwrap();
        manager.submit(2, &intent(2)).unwrap();

        let mut events = manager.poll_all_events();
        events.sort_by_key(|(id, _)| *id);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, 1);
        assert_eq!(events[1].0, 2);
    }

    #[test]
    fn removing_an_account_makes_it_unknown_again() {
        let mut manager = manager_with_two_accounts();
        assert!(manager.remove_account(1).is_some());
        assert!(manager.submit(1, &intent(1)).is_err());
        assert_eq!(manager.account_ids().count(), 1);
    }

    #[test]
    fn heterogeneous_brokers_can_share_one_manager_via_box_dyn_broker() {
        let mut manager: AccountManager<Box<dyn Broker>> = AccountManager::new();
        manager.add_account(1, Box::new(SimBroker::from_seed(SimBrokerConfig::default(), 10))).unwrap();
        manager.add_account(2, Box::new(SimBroker::from_seed(SimBrokerConfig { starting_equity: 2_000_000, ..Default::default() }, 20))).unwrap();

        manager.submit(1, &intent(1)).unwrap();
        manager.submit(2, &intent(2)).unwrap();

        let aggregate = manager.aggregate_snapshot();
        assert_eq!(aggregate.n_accounts, 2);
        assert_eq!(aggregate.equity, 1_000_000 + 2_000_000);
    }
}
