//! Deterministic replay. Every tick/signal/decision/order/fill is an
//! immutable event on a durable, JSON-Lines-encoded log for now (§P4); a
//! production log format (likely rkyv-framed) lands once the hot-path bus
//! in `crates/bus` is implemented. The replay contract — same input events
//! in, bit-identical orders out — is fixed from Phase 0 so Phase 1's
//! "24h replay is bit-identical" exit criterion (§17) has something to run
//! against.

use domain::Tick;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "kind")]
pub enum ReplayEvent {
    Tick(Tick),
}

/// Reads a JSON-Lines event log, one `ReplayEvent` per line.
pub fn read_events<R: Read>(reader: R) -> std::io::Result<Vec<ReplayEvent>> {
    BufReader::new(reader)
        .lines()
        .map(|line| {
            let line = line?;
            serde_json::from_str(&line).map_err(std::io::Error::other)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_a_tick_event_through_jsonl() {
        let tick = Tick { ts_ns: 1, recv_ns: 1, symbol_id: 1, bid: 100, ask: 101, bid_volume: 1, ask_volume: 1, flags: 0 };
        let event = ReplayEvent::Tick(tick);
        let line = serde_json::to_string(&event).unwrap();
        let events = read_events(line.as_bytes()).unwrap();
        assert_eq!(events, vec![event]);
    }
}
