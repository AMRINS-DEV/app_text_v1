//! Integration test: `adapter-mt5` against `mock-mt5-bridge` over real ZMQ
//! sockets on loopback. This is the closest this environment can get to
//! the design doc's §17 Phase 1 exit criterion ("Ticks flow with p99 <
//! 50 µs bridge→core") without a real MT5 terminal — the bridge side here
//! is a Rust test double speaking the identical wire protocol
//! (`adapter_mt5::protocol`), not the real MQL5 EA.

use adapter_mt5::Mt5MarketData;
use domain::ports::MarketDataSource;
use mock_mt5_bridge::{now_ns, run_market_data, MarketDataConfig};
use std::time::{Duration, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ticks_flow_end_to_end_with_correct_sequence_and_no_gaps() {
    let addr = "tcp://127.0.0.1:29201";
    let bridge = tokio::spawn(run_market_data(MarketDataConfig {
        bind_addr: addr.into(),
        symbol_id: 1,
        max_ticks: Some(1000),
        ticks_per_heartbeat: 250,
    }));

    tokio::time::sleep(Duration::from_millis(150)).await;

    let mut md = Mt5MarketData::new(addr);
    md.subscribe(&[]).unwrap();

    let mut received: Vec<domain::Tick> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while received.len() < 1000 && Instant::now() < deadline {
        match md.poll_tick() {
            Some(tick) => received.push(tick),
            None => tokio::time::sleep(Duration::from_micros(50)).await,
        }
    }

    assert_eq!(received.len(), 1000, "did not receive all published ticks before the deadline");
    // No gaps expected on a clean loopback run with no dropped frames.
    assert_eq!(md.last_seq(), 999);
    for tick in &received {
        assert_eq!(tick.flags() & domain::TickFlags::GAP, domain::TickFlags::empty());
    }

    bridge.await.expect("bridge task panicked").expect("bridge task returned an error");
}

/// Measures actual bridge -> core latency (send timestamp to `poll_tick`
/// receipt) and reports p50/p99/p999 against the §5.2 budget honestly —
/// this asserts the mechanism works, not that the number hits the target,
/// since a pure-Rust ZMQ implementation over TCP loopback is not the same
/// transport the design doc benchmarks (native libzmq or shared memory).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bridge_to_core_latency_is_measured_against_the_budget() {
    let addr = "tcp://127.0.0.1:29202";
    const N: u64 = 2000;
    let bridge =
        tokio::spawn(run_market_data(MarketDataConfig { bind_addr: addr.into(), symbol_id: 1, max_ticks: Some(N), ticks_per_heartbeat: 0 }));

    tokio::time::sleep(Duration::from_millis(150)).await;

    let mut md = Mt5MarketData::new(addr);
    md.subscribe(&[]).unwrap();

    let mut latencies_ns = Vec::with_capacity(N as usize);
    let deadline = Instant::now() + Duration::from_secs(15);
    while (latencies_ns.len() as u64) < N && Instant::now() < deadline {
        if let Some(tick) = md.poll_tick() {
            let recv_ns = now_ns();
            latencies_ns.push(recv_ns.saturating_sub(tick.ts_ns));
        } else {
            tokio::task::yield_now().await;
        }
    }
    bridge.await.expect("bridge task panicked").expect("bridge task returned an error");

    assert_eq!(latencies_ns.len() as u64, N, "did not receive all ticks before the deadline");
    latencies_ns.sort_unstable();
    let p50 = latencies_ns[latencies_ns.len() / 2];
    let p99 = latencies_ns[latencies_ns.len() * 99 / 100];
    let p999 = latencies_ns[(latencies_ns.len() * 999 / 1000).min(latencies_ns.len() - 1)];
    println!("bridge->core latency (ns): p50={p50} p99={p99} p999={p999}  [§5.2 budget: p99 < 50_000 ns]");

    // Intentionally not asserting p99 < 50_000: a pure-Rust ZMQ client over
    // TCP loopback in a shared CI sandbox is a different transport than the
    // native-libzmq/shared-memory path §5.2 budgets for. Recording the
    // number is the point — see docs/protocol.md for the honest comparison.
    assert!(p50 > 0, "sanity: latency must be measurable");
}
