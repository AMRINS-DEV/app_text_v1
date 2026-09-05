//! CLI entry point for the mock MT5 bridge (§5.4). Runs both the tick
//! publisher and the order responder concurrently until Ctrl-C, or for a
//! fixed tick count with `--ticks`.
//!
//! Usage:
//!   mock-mt5-bridge --tick-addr tcp://127.0.0.1:28001 --order-addr tcp://127.0.0.1:28002 [--ticks N]

use mock_mt5_bridge::{run_market_data, run_order_responder, MarketDataConfig};

#[tokio::main]
async fn main() {
    let mut tick_addr = "tcp://127.0.0.1:28001".to_string();
    let mut order_addr = "tcp://127.0.0.1:28002".to_string();
    let mut max_ticks: Option<u64> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--tick-addr" => tick_addr = args.next().expect("--tick-addr needs a value"),
            "--order-addr" => order_addr = args.next().expect("--order-addr needs a value"),
            "--ticks" => {
                max_ticks = Some(args.next().expect("--ticks needs a value").parse().expect("--ticks must be a number"))
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(1);
            }
        }
    }

    println!("mock-mt5-bridge: ticks on {tick_addr}, orders on {order_addr}");

    let market_data = run_market_data(MarketDataConfig {
        bind_addr: tick_addr,
        symbol_id: 1,
        max_ticks,
        ticks_per_heartbeat: 100,
    });
    let orders = run_order_responder(order_addr, None);

    tokio::select! {
        res = market_data => { if let Err(e) = res { eprintln!("market data task ended: {e}"); } }
        res = orders => { if let Err(e) = res { eprintln!("order responder task ended: {e}"); } }
        _ = tokio::signal::ctrl_c() => { println!("shutting down"); }
    }
}
