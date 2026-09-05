//! §17 Phase 8's literal exit criterion, made concrete rather than argued in
//! prose: "same strategy runs on 2 adapters with no core change."
//! `execution::OrderRouter<B>` is generic over `B: domain::ports::Broker`
//! and is never touched by this test — `run_router_scenario` below is one
//! function, unaware which `Broker` it's driving, called once with
//! `execution::SimBroker` (Phase 2's own double) and once with this crate's
//! `CTraderBroker` against a real `mock-ctrader-server` over a real TCP
//! socket. If both runs report the same externally observable outcomes,
//! the trait boundary in `domain::ports` really is the only thing a new
//! platform needs to satisfy, exactly as `domain::ports`'s own doc comment
//! claims.

use std::time::{Duration, Instant};

use adapter_ctrader::CTraderBroker;
use domain::ports::Broker;
use domain::{ExecEvent, OrderIntent, OrderType, Side, TimeInForce, TradingMode};
use execution::{OrderRouter, SimBroker, SimBrokerConfig};
use mock_ctrader_server::run_order_responder;
use smallvec::SmallVec;

fn intent(client_id: u128, sl: Option<i64>, tp: Option<i64>) -> OrderIntent {
    OrderIntent {
        client_id,
        symbol_id: 1,
        side: Side::Buy,
        qty: 100,
        order_type: OrderType::Market,
        limit_px: Some(100_000),
        sl,
        tp,
        tif: TimeInForce::Gtc,
        mode: TradingMode::Normal,
        max_slippage_pts: 5,
        signal_ids: SmallVec::new(),
    }
}

#[derive(Debug, PartialEq)]
struct ScenarioOutcome {
    rejected_missing_sl_tp: bool,
    first_submit_ok: bool,
    resubmission_returns_same_id: bool,
    a_fill_event_is_observed_for_the_first_order: bool,
    positions_after_first_order_len: usize,
    positions_after_close_len: usize,
}

/// The exact same call sequence against `router`, regardless of which
/// `Broker` backs it: reject an order missing SL/TP, submit a valid one,
/// resubmit the same `client_id` (must be idempotent), observe the
/// resulting fill via `poll_event`, then close the position. Every
/// assertion in this function is about `OrderRouter`'s own behavior, not
/// about which broker is underneath — that's the point.
fn run_router_scenario<B: Broker>(mut router: OrderRouter<B>) -> ScenarioOutcome {
    let rejected_missing_sl_tp = router.submit(&intent(1, None, Some(101_000))).is_err();

    let first = router.submit(&intent(2, Some(99_000), Some(101_000)));
    let first_submit_ok = first.is_ok();
    let first_id = first.ok();

    let resubmission_returns_same_id = match (&first_id, router.submit(&intent(2, Some(99_000), Some(101_000)))) {
        (Some(a), Ok(b)) => *a == b,
        _ => false,
    };

    let mut observed_fill = false;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !observed_fill && Instant::now() < deadline {
        match router.poll_event() {
            Some(ExecEvent::Fill { broker_order_id, .. }) if Some(broker_order_id) == first_id => observed_fill = true,
            Some(_) => continue,
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }

    let positions_after_first_order_len = router.broker().positions().map(|p| p.len()).unwrap_or(usize::MAX);

    if let Some(id) = first_id {
        let _ = router.close(id, None);
    }
    let positions_after_close_len = router.broker().positions().map(|p| p.len()).unwrap_or(usize::MAX);

    ScenarioOutcome {
        rejected_missing_sl_tp,
        first_submit_ok,
        resubmission_returns_same_id,
        a_fill_event_is_observed_for_the_first_order: observed_fill,
        positions_after_first_order_len,
        positions_after_close_len,
    }
}

#[test]
fn the_same_order_router_scenario_behaves_identically_over_simbroker_and_ctraderbroker() {
    let sim_outcome = run_router_scenario(OrderRouter::new(SimBroker::new(SimBrokerConfig::default())));

    let addr = "127.0.0.1:29320";
    // Requests that actually reach the broker: the rejected-before-router
    // submit and the idempotent resubmit never call `Broker::submit` at
    // all, but unlike `SimBroker`, `CTraderBroker::positions()` is a real
    // network round trip too — submit(1) + positions(1) + close(1) +
    // positions(1) = 4.
    let server = std::thread::spawn(move || run_order_responder(addr.into(), Some(4)));
    std::thread::sleep(Duration::from_millis(100));
    let ctrader_outcome = run_router_scenario(OrderRouter::new(CTraderBroker::new(addr)));
    server.join().expect("server thread panicked").expect("server task returned an error");

    assert_eq!(
        sim_outcome, ctrader_outcome,
        "OrderRouter<SimBroker> and OrderRouter<CTraderBroker> must produce identical externally observable \
         outcomes for the same call sequence — the whole point of the trait boundary in domain::ports"
    );
    // Sanity: the shared scenario actually exercised something on both
    // sides, rather than both trivially reporting all-false/zero.
    assert!(sim_outcome.rejected_missing_sl_tp);
    assert!(sim_outcome.first_submit_ok);
    assert!(sim_outcome.resubmission_returns_same_id);
    assert!(sim_outcome.a_fill_event_is_observed_for_the_first_order);
    assert_eq!(sim_outcome.positions_after_first_order_len, 1);
    assert_eq!(sim_outcome.positions_after_close_len, 0);
}
