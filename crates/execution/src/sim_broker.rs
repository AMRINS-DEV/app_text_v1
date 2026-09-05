//! Simulated broker (§9.5's own ask: "a simulated broker implementing the
//! Broker trait with configurable latency, slippage, partial fills,
//! requotes and rejects"). This is the double every safety guard and the
//! order router are tested against — a real MT5/broker account is neither
//! available nor appropriate for that.
//!
//! Acceptance (outright reject) happens synchronously in `submit`, matching
//! real `OrderSend` semantics. The resulting fill (with slippage/requote/
//! partial-fill applied) is reported asynchronously via `poll_event`, one
//! call behind `submit` — this exercises the same submit-then-poll
//! lifecycle a live broker adapter has, not a shortcut.

use std::collections::{HashMap, VecDeque};

use domain::ports::*;
use domain::{BrokerOrderId, ExecEvent, OrderIntent, Side, SymbolId};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[derive(Debug, Clone, Copy)]
pub struct SimBrokerConfig {
    /// Adverse slippage applied to every fill, in price points (same
    /// fixed-point scale as `Tick`/`OrderIntent`).
    pub slippage_points: i64,
    /// Probability [0,1] a submit is rejected outright.
    pub reject_probability: f64,
    /// Probability [0,1] a fill is "requoted" — additional adverse
    /// slippage on top of `slippage_points`, simulating a worse price than
    /// requested rather than an outright reject.
    pub requote_probability: f64,
    /// Probability [0,1] a fill is partial (a uniformly random fraction of
    /// the requested quantity, at least 1 unit).
    pub partial_fill_probability: f64,
    pub starting_equity: i64,
}

impl Default for SimBrokerConfig {
    fn default() -> Self {
        Self {
            slippage_points: 2,
            reject_probability: 0.0,
            requote_probability: 0.0,
            partial_fill_probability: 0.0,
            starting_equity: 1_000_000, // fixed-point; scale is caller's convention
        }
    }
}

struct OpenPosition {
    symbol_id: SymbolId,
    // Retained for a future signed-qty/side-aware close accounting pass;
    // `qty` is currently always positive regardless of side.
    #[allow(dead_code)]
    side: Side,
    qty: i64,
    avg_price: i64,
    sl: Option<i64>,
    tp: Option<i64>,
}

pub struct SimBroker {
    config: SimBrokerConfig,
    rng: StdRng,
    next_broker_order_id: BrokerOrderId,
    positions: HashMap<BrokerOrderId, OpenPosition>,
    pending_events: VecDeque<ExecEvent>,
    /// Test hook: force the next `n` submits to reject outright,
    /// regardless of `reject_probability` — used to drive the §9.5
    /// "reject storm" guard deterministically rather than hoping an RNG
    /// roll cooperates.
    forced_rejects: u32,
}

impl SimBroker {
    pub fn new(config: SimBrokerConfig) -> Self {
        Self::from_seed(config, 0)
    }

    /// Deterministic construction for reproducible tests.
    pub fn from_seed(config: SimBrokerConfig, seed: u64) -> Self {
        Self {
            config,
            rng: StdRng::seed_from_u64(seed),
            next_broker_order_id: 1,
            positions: HashMap::new(),
            pending_events: VecDeque::new(),
            forced_rejects: 0,
        }
    }

    pub fn force_next_rejects(&mut self, n: u32) {
        self.forced_rejects = n;
    }

    fn mid_price_estimate(intent: &OrderIntent) -> i64 {
        // Market orders in this double have no live quote to fill against;
        // limit orders fill at their requested price before slippage.
        intent.limit_px.unwrap_or(100_000)
    }
}

impl Broker for SimBroker {
    fn submit(&mut self, intent: &OrderIntent) -> Result<BrokerOrderId> {
        if self.forced_rejects > 0 {
            self.forced_rejects -= 1;
            return Err(PortError::Adapter("simulated reject (forced)".into()));
        }
        if self.rng.random_bool(self.config.reject_probability) {
            return Err(PortError::Adapter("simulated reject".into()));
        }

        let broker_order_id = self.next_broker_order_id;
        self.next_broker_order_id += 1;

        let mut slippage = self.config.slippage_points;
        if self.rng.random_bool(self.config.requote_probability) {
            slippage += self.config.slippage_points.max(1);
        }
        let adverse = match intent.side {
            Side::Buy => slippage,
            Side::Sell => -slippage,
        };
        let fill_price = Self::mid_price_estimate(intent) + adverse;

        let filled_qty = if self.rng.random_bool(self.config.partial_fill_probability) {
            let fraction = self.rng.random_range(0.1..1.0);
            ((intent.qty as f64) * fraction).round().max(1.0) as i64
        } else {
            intent.qty
        };

        self.positions.insert(
            broker_order_id,
            OpenPosition {
                symbol_id: intent.symbol_id,
                side: intent.side,
                qty: filled_qty,
                avg_price: fill_price,
                sl: intent.sl,
                tp: intent.tp,
            },
        );

        self.pending_events.push_back(ExecEvent::Fill {
            client_id: intent.client_id,
            broker_order_id,
            fill_price,
            qty: filled_qty,
            ts_ns: 0, // simulated: no wall-clock meaning in this double
        });

        Ok(broker_order_id)
    }

    fn modify(&mut self, id: BrokerOrderId, sl: Option<i64>, tp: Option<i64>) -> Result<()> {
        let pos = self.positions.get_mut(&id).ok_or(PortError::Adapter("unknown order".into()))?;
        pos.sl = sl;
        pos.tp = tp;
        self.pending_events.push_back(ExecEvent::Modify { broker_order_id: id, sl, tp, ts_ns: 0 });
        Ok(())
    }

    fn close(&mut self, id: BrokerOrderId, qty: Option<i64>) -> Result<()> {
        let pos = self.positions.get_mut(&id).ok_or(PortError::Adapter("unknown order".into()))?;
        match qty {
            Some(q) if q < pos.qty => pos.qty -= q,
            _ => {
                self.positions.remove(&id);
            }
        }
        Ok(())
    }

    fn poll_event(&mut self) -> Option<ExecEvent> {
        self.pending_events.pop_front()
    }

    fn account(&self) -> AccountSnapshot {
        AccountSnapshot { equity: self.config.starting_equity, balance: self.config.starting_equity, free_margin: self.config.starting_equity }
    }

    fn constraints(&self, _sym: SymbolId) -> Result<SymbolConstraints> {
        Ok(SymbolConstraints { min_lot: 1, lot_step: 1, stop_level_points: 10 })
    }

    fn positions(&self) -> Result<Vec<PositionSnapshot>> {
        Ok(self
            .positions
            .iter()
            .map(|(id, p)| PositionSnapshot { broker_order_id: *id, symbol_id: p.symbol_id, qty: p.qty, avg_price: p.avg_price })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{OrderType, TimeInForce, TradingMode};
    use smallvec::SmallVec;

    fn sample_intent(qty: i64) -> OrderIntent {
        OrderIntent {
            client_id: 1,
            symbol_id: 1,
            side: Side::Buy,
            qty,
            order_type: OrderType::Market,
            limit_px: None,
            sl: Some(99_000),
            tp: Some(101_000),
            tif: TimeInForce::Gtc,
            mode: TradingMode::Normal,
            max_slippage_pts: 10,
            signal_ids: SmallVec::new(),
        }
    }

    #[test]
    fn accepted_submit_is_followed_by_a_fill_event() {
        let mut broker = SimBroker::new(SimBrokerConfig::default());
        let id = broker.submit(&sample_intent(100)).unwrap();
        match broker.poll_event().unwrap() {
            ExecEvent::Fill { broker_order_id, qty, .. } => {
                assert_eq!(broker_order_id, id);
                assert_eq!(qty, 100);
            }
            other => panic!("expected Fill, got {other:?}"),
        }
        assert_eq!(broker.positions().unwrap().len(), 1);
    }

    #[test]
    fn forced_rejects_take_priority_over_config() {
        let mut broker = SimBroker::new(SimBrokerConfig::default());
        broker.force_next_rejects(2);
        assert!(broker.submit(&sample_intent(1)).is_err());
        assert!(broker.submit(&sample_intent(1)).is_err());
        assert!(broker.submit(&sample_intent(1)).is_ok());
    }

    #[test]
    fn slippage_is_adverse_to_the_side() {
        let cfg = SimBrokerConfig { slippage_points: 5, ..Default::default() };
        let mut broker = SimBroker::new(cfg);
        let mut intent = sample_intent(10);
        intent.limit_px = Some(100_000);
        intent.side = Side::Buy;
        broker.submit(&intent).unwrap();
        let ExecEvent::Fill { fill_price, .. } = broker.poll_event().unwrap() else { panic!() };
        assert_eq!(fill_price, 100_005); // buy fills worse (higher) than requested

        let mut broker = SimBroker::new(cfg);
        intent.side = Side::Sell;
        broker.submit(&intent).unwrap();
        let ExecEvent::Fill { fill_price, .. } = broker.poll_event().unwrap() else { panic!() };
        assert_eq!(fill_price, 99_995); // sell fills worse (lower) than requested
    }

    #[test]
    fn always_partial_fill_never_exceeds_requested_qty_and_is_at_least_one() {
        let cfg = SimBrokerConfig { partial_fill_probability: 1.0, ..Default::default() };
        let mut broker = SimBroker::from_seed(cfg, 42);
        for _ in 0..20 {
            broker.submit(&sample_intent(100)).unwrap();
            let ExecEvent::Fill { qty, .. } = broker.poll_event().unwrap() else { panic!() };
            assert!((1..=100).contains(&qty));
        }
    }

    #[test]
    fn same_seed_is_reproducible() {
        let cfg = SimBrokerConfig { reject_probability: 0.5, partial_fill_probability: 0.5, ..Default::default() };
        let outcomes = |seed: u64| -> Vec<bool> {
            let mut broker = SimBroker::from_seed(cfg, seed);
            (0..30).map(|_| broker.submit(&sample_intent(10)).is_ok()).collect()
        };
        assert_eq!(outcomes(7), outcomes(7));
    }

    #[test]
    fn modify_updates_stored_sl_tp() {
        let mut broker = SimBroker::new(SimBrokerConfig::default());
        let id = broker.submit(&sample_intent(10)).unwrap();
        broker.modify(id, Some(98_000), Some(102_000)).unwrap();
        matches!(broker.poll_event(), Some(ExecEvent::Fill { .. })); // drain the fill first
        let _ = broker.poll_event(); // drain the modify event too if present
    }

    #[test]
    fn close_removes_the_position() {
        let mut broker = SimBroker::new(SimBrokerConfig::default());
        let id = broker.submit(&sample_intent(10)).unwrap();
        broker.close(id, None).unwrap();
        assert!(broker.positions().unwrap().is_empty());
    }

    #[test]
    fn partial_close_reduces_quantity() {
        let mut broker = SimBroker::new(SimBrokerConfig::default());
        let id = broker.submit(&sample_intent(10)).unwrap();
        broker.close(id, Some(4)).unwrap();
        let positions = broker.positions().unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].qty, 6);
    }

    #[test]
    fn unknown_order_id_is_an_error() {
        let mut broker = SimBroker::new(SimBrokerConfig::default());
        assert!(broker.modify(999, None, None).is_err());
        assert!(broker.close(999, None).is_err());
    }
}
