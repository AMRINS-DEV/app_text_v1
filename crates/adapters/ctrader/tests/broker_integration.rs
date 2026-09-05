//! Integration test: `adapter-ctrader` against `mock-ctrader-server` over
//! real TCP sockets on loopback — the closest this environment can get to
//! exercising the adapter end-to-end without a real cTrader account (the
//! same "test double, not a live account" split as
//! `adapter-mt5`/`mock-mt5-bridge`'s own integration test).

use std::time::{Duration, Instant};

use adapter_ctrader::{CTraderBroker, CTraderMarketData};
use domain::ports::{Broker, MarketDataSource, SymbolSpec};
use domain::{ExecEvent, OrderIntent, OrderType, Side, TimeInForce, TradingMode};
use mock_ctrader_server::{run_market_data, run_order_responder, MarketDataConfig};
use smallvec::SmallVec;

fn sample_intent(client_id: u128) -> OrderIntent {
    OrderIntent {
        client_id,
        symbol_id: 1,
        side: Side::Buy,
        qty: 100,
        order_type: OrderType::Market,
        limit_px: Some(100_000),
        sl: Some(99_000),
        tp: Some(101_000),
        tif: TimeInForce::Gtc,
        mode: TradingMode::Normal,
        max_slippage_pts: 5,
        signal_ids: SmallVec::new(),
    }
}

#[test]
fn spot_ticks_flow_end_to_end_over_a_real_socket() {
    let addr = "127.0.0.1:29311";
    let server = std::thread::spawn(move || {
        run_market_data(MarketDataConfig { bind_addr: addr.into(), symbol_id: 1, max_ticks: Some(500) })
    });

    std::thread::sleep(Duration::from_millis(100));

    let mut md = CTraderMarketData::new(addr);
    md.subscribe(&[SymbolSpec { symbol_id: 1, price_digits: 5 }]).unwrap();

    let mut received = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while received.len() < 500 && Instant::now() < deadline {
        match md.poll_tick() {
            Some(tick) => received.push(tick),
            None => std::thread::sleep(Duration::from_micros(200)),
        }
    }

    assert_eq!(received.len(), 500, "did not receive all published ticks before the deadline");
    for tick in &received {
        assert_eq!(tick.symbol_id, 1);
        assert!(tick.ask > tick.bid);
    }

    server.join().expect("server thread panicked").expect("server task returned an error");
}

#[test]
fn submit_is_accepted_then_a_fill_arrives_asynchronously_via_poll_event() {
    let addr = "127.0.0.1:29312";
    let server = std::thread::spawn(move || run_order_responder(addr.into(), Some(1)));
    std::thread::sleep(Duration::from_millis(100));

    let mut broker = CTraderBroker::new(addr);
    let broker_order_id = broker.submit(&sample_intent(1)).expect("submit should be accepted");
    assert_eq!(broker_order_id, 1);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut event = None;
    while event.is_none() && Instant::now() < deadline {
        event = broker.poll_event();
        if event.is_none() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    match event.expect("a fill event should eventually arrive") {
        ExecEvent::Fill { broker_order_id: id, qty, .. } => {
            assert_eq!(id, broker_order_id);
            assert_eq!(qty, 100);
        }
        other => panic!("expected Fill, got {other:?}"),
    }

    server.join().expect("server thread panicked").expect("server task returned an error");
}

#[test]
fn full_submit_amend_positions_close_roundtrip() {
    let addr = "127.0.0.1:29313";
    let server = std::thread::spawn(move || run_order_responder(addr.into(), Some(4)));
    std::thread::sleep(Duration::from_millis(100));

    let mut broker = CTraderBroker::new(addr);
    let id = broker.submit(&sample_intent(1)).unwrap();

    broker.modify(id, Some(98_000), Some(102_000)).expect("amend should succeed for a known order");

    let positions = broker.positions().expect("positions request should succeed");
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].broker_order_id, id);
    assert_eq!(positions[0].qty, 100);

    broker.close(id, None).expect("close should succeed for a known order");

    server.join().expect("server thread panicked").expect("server task returned an error");
}

#[test]
fn amending_an_unknown_order_is_rejected() {
    let addr = "127.0.0.1:29314";
    let server = std::thread::spawn(move || run_order_responder(addr.into(), Some(1)));
    std::thread::sleep(Duration::from_millis(100));

    let mut broker = CTraderBroker::new(addr);
    let result = broker.modify(999, Some(1), Some(2));
    assert!(result.is_err());

    server.join().expect("server thread panicked").expect("server task returned an error");
}

#[test]
fn account_request_returns_the_servers_snapshot() {
    let addr = "127.0.0.1:29315";
    let server = std::thread::spawn(move || run_order_responder(addr.into(), Some(1)));
    std::thread::sleep(Duration::from_millis(100));

    let broker = CTraderBroker::new(addr);
    let account = broker.account();
    assert_eq!(account.equity, 1_000_000);
    assert_eq!(account.balance, 1_000_000);

    server.join().expect("server thread panicked").expect("server task returned an error");
}
