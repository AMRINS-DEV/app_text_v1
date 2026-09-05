//! CLI entry point for the mock cTrader server (§17 Phase 8). Runs the
//! spot-price stream and the order responder concurrently on two OS threads
//! until both connections close.
//!
//! Usage:
//!   mock-ctrader-server --md-addr 127.0.0.1:29301 --order-addr 127.0.0.1:29302 [--ticks N]

use mock_ctrader_server::{run_market_data, run_order_responder, MarketDataConfig};

fn main() {
    let mut md_addr = "127.0.0.1:29301".to_string();
    let mut order_addr = "127.0.0.1:29302".to_string();
    let mut max_ticks: Option<u64> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--md-addr" => md_addr = args.next().expect("--md-addr needs a value"),
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

    println!("mock-ctrader-server: spot on {md_addr}, orders on {order_addr}");

    let md_handle = std::thread::spawn(move || run_market_data(MarketDataConfig { bind_addr: md_addr, symbol_id: 1, max_ticks }));
    let order_handle = std::thread::spawn(move || run_order_responder(order_addr, None));

    if let Err(e) = md_handle.join().expect("market data thread panicked") {
        eprintln!("market data task ended: {e}");
    }
    if let Err(e) = order_handle.join().expect("order responder thread panicked") {
        eprintln!("order responder task ended: {e}");
    }
}
