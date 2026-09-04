//! Deterministic replay CLI. Phase 0: reads a JSON-Lines event log and
//! reports the event count. Feeding replayed events back through the core
//! pipeline and diffing the resulting order stream against a recorded
//! baseline (the actual §15 regression check) is Phase 1 scope — it needs
//! `crates/bin/tradeos-core`'s pipeline to be runnable headlessly first.

use std::env;
use std::fs::File;

fn main() {
    let mut args = env::args().skip(1);
    let (mut input, mut usage) = (None, false);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = args.next(),
            "--help" | "-h" => usage = true,
            other => {
                eprintln!("unknown argument: {other}");
                usage = true;
            }
        }
    }
    if usage || input.is_none() {
        eprintln!("usage: tradeos-replay --input <events.jsonl>");
        std::process::exit(1);
    }

    let path = input.unwrap();
    let file = File::open(&path).unwrap_or_else(|e| panic!("failed to open {path}: {e}"));
    let events = replay::read_events(file).unwrap_or_else(|e| panic!("failed to parse {path}: {e}"));
    println!("read {} event(s) from {path}", events.len());
}
