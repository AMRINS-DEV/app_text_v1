//! End-to-end test: a real bound HTTP server (`TradingViewMarketData::subscribe`)
//! hit with a real `reqwest` client over loopback TCP — verifying the actual
//! server binds and serves, not just that the handler functions compute the
//! right values in isolation (`webhook`/`udf`'s own unit tests already cover
//! that).

use std::net::SocketAddr;
use std::time::Duration;

use adapter_tradingview::{TradingViewConfig, TradingViewMarketData};
use domain::ports::MarketDataSource;
use serde_json::json;

fn start_server(bind_addr: &str, token: &str) -> TradingViewMarketData {
    let mut md = TradingViewMarketData::new(TradingViewConfig {
        bind_addr: bind_addr.parse::<SocketAddr>().unwrap(),
        webhook_token: token.into(),
        symbols: vec![("EURUSD".into(), 1)],
        price_scale: 100_000,
    });
    md.subscribe(&[]).unwrap();
    md
}

#[tokio::test]
async fn webhook_alert_becomes_a_polled_tick_and_udf_config_is_served() {
    let mut md = start_server("127.0.0.1:29401", "secret");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let client = reqwest::Client::new();

    let config: serde_json::Value = client.get("http://127.0.0.1:29401/udf/config").send().await.unwrap().json().await.unwrap();
    assert_eq!(config["supports_search"], true);
    assert!(config["supported_resolutions"].as_array().unwrap().contains(&json!("1D")));

    let resp = client
        .post("http://127.0.0.1:29401/webhook")
        .json(&json!({ "token": "secret", "ticker": "EURUSD", "price": 1.10500, "time_ns": 1_000_000_000u64 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    tokio::time::sleep(Duration::from_millis(50)).await;
    let tick = md.poll_tick().expect("the webhook alert should have produced a polled tick");
    assert_eq!(tick.symbol_id, 1);
    assert_eq!(tick.bid, 110_500);
    assert_eq!(tick.ask, 110_500);
}

#[tokio::test]
async fn webhook_alert_with_the_wrong_token_is_rejected_and_produces_no_tick() {
    let mut md = start_server("127.0.0.1:29402", "secret");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post("http://127.0.0.1:29402/webhook")
        .json(&json!({ "token": "wrong", "ticker": "EURUSD", "price": 1.1, "time_ns": 1_000_000_000u64 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    assert!(md.poll_tick().is_none());
}

#[tokio::test]
async fn udf_symbols_endpoint_rejects_an_unknown_ticker() {
    let _md = start_server("127.0.0.1:29403", "secret");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let client = reqwest::Client::new();
    let resp = client.get("http://127.0.0.1:29403/udf/symbols?symbol=GBPUSD").send().await.unwrap();
    assert_eq!(resp.status(), 404);

    let ok = client.get("http://127.0.0.1:29403/udf/symbols?symbol=EURUSD").send().await.unwrap();
    assert_eq!(ok.status(), 200);
}

#[tokio::test]
async fn a_webhook_alert_is_queryable_through_the_real_udf_history_endpoint() {
    let mut md = start_server("127.0.0.1:29404", "secret");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let client = reqwest::Client::new();

    // Two alerts a minute apart close the first 1-minute bar.
    for (price, time_ns) in [(1.1000, 0u64), (1.1010, 70_000_000_000)] {
        let resp = client
            .post("http://127.0.0.1:29404/webhook")
            .json(&json!({ "token": "secret", "ticker": "EURUSD", "price": price, "time_ns": time_ns }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    // Drain the ticks so this test's own poll doesn't leak into another.
    while md.poll_tick().is_some() {}

    let history: serde_json::Value = client
        .get("http://127.0.0.1:29404/udf/history?symbol=EURUSD&resolution=1&from=0&to=1000")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(history["s"], "ok");
    assert_eq!(history["o"][0], 1.1);
}
