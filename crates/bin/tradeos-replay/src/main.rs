//! Deterministic replay CLI (§4, §15 item 2). Reads a JSON-Lines event
//! log, runs it through the real `market-data`/`features`/`risk` pipeline
//! twice via `replay::pipeline`, and asserts the two runs are
//! byte-for-byte identical — the actual property behind the Phase 1 exit
//! criterion "24h replay is bit-identical" (§17), demonstrated here at
//! whatever scale the input file covers.

use std::env;
use std::fs::File;

use domain::{SymbolId, Tick};
use mock_mt5_bridge::synthetic_tick;
use replay::{pipeline::run_deterministic_pipeline, ReplayEvent};

fn main() {
    let mut args = env::args().skip(1);
    let (mut input, mut symbol_id, mut timeframe_seconds, mut usage) = (None, 1u16, 1u32, false);
    let mut generate_sample: Option<(String, u64)> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = args.next(),
            "--symbol-id" => symbol_id = args.next().and_then(|s| s.parse().ok()).unwrap_or(1),
            "--timeframe-seconds" => timeframe_seconds = args.next().and_then(|s| s.parse().ok()).unwrap_or(1),
            "--generate-sample" => {
                let path = args.next().expect("--generate-sample needs a path");
                let count: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1000);
                generate_sample = Some((path, count));
            }
            "--help" | "-h" => usage = true,
            other => {
                eprintln!("unknown argument: {other}");
                usage = true;
            }
        }
    }

    if let Some((path, count)) = generate_sample {
        let events: Vec<ReplayEvent> =
            (0..count).map(|seq| ReplayEvent::Tick(synthetic_tick(symbol_id, seq))).collect();
        let file = File::create(&path).unwrap_or_else(|e| panic!("failed to create {path}: {e}"));
        replay::write_events(&events, file).unwrap_or_else(|e| panic!("failed to write {path}: {e}"));
        println!("wrote {count} synthetic tick event(s) to {path}");
        return;
    }

    if usage || input.is_none() {
        eprintln!("usage: tradeos-replay --input <events.jsonl> [--symbol-id N] [--timeframe-seconds N]");
        eprintln!("       tradeos-replay --generate-sample <events.jsonl> [count]");
        std::process::exit(1);
    }

    let path = input.unwrap();
    let file = File::open(&path).unwrap_or_else(|e| panic!("failed to open {path}: {e}"));
    let events = replay::read_events(file).unwrap_or_else(|e| panic!("failed to parse {path}: {e}"));
    println!("read {} event(s) from {path}", events.len());

    let ticks: Vec<Tick> = events
        .into_iter()
        .map(|e| match e {
            ReplayEvent::Tick(t) => t,
        })
        .collect();

    let symbol_id: SymbolId = symbol_id;
    let run1 = run_deterministic_pipeline(&ticks, symbol_id, timeframe_seconds);
    let run2 = run_deterministic_pipeline(&ticks, symbol_id, timeframe_seconds);

    if run1 != run2 {
        eprintln!("DETERMINISM VIOLATION: replaying the same {} tick(s) twice produced different output", ticks.len());
        std::process::exit(1);
    }

    let signal_count = run1.iter().filter(|d| d.lots.is_some()).count();
    println!(
        "replayed {} tick(s) -> {} closed bar(s), {} placeholder signal(s) fired -- bit-identical across 2 runs",
        ticks.len(),
        run1.len(),
        signal_count
    );
}
