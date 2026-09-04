//! Deterministic replay. Every tick/signal/decision/order/fill is an
//! immutable event on a durable, JSON-Lines-encoded log for now (§P4); a
//! production log format (likely rkyv-framed) lands once the hot-path bus
//! in `crates/bus` is implemented. The replay contract — same input events
//! in, bit-identical orders out — is fixed from Phase 0 so Phase 1's
//! "24h replay is bit-identical" exit criterion (§17) has something to run
//! against.

pub mod pipeline;

use domain::Tick;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};

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

/// Writes a JSON-Lines event log, one `ReplayEvent` per line — the
/// recording counterpart to `read_events`, e.g. for capturing a live or
/// mock-bridge tick stream to replay later.
pub fn write_events<W: Write>(events: &[ReplayEvent], mut writer: W) -> std::io::Result<()> {
    for event in events {
        let line = serde_json::to_string(event).map_err(std::io::Error::other)?;
        writeln!(writer, "{line}")?;
    }
    Ok(())
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

    #[test]
    fn write_then_read_roundtrips_multiple_events() {
        let events = vec![
            ReplayEvent::Tick(Tick { ts_ns: 1, recv_ns: 1, symbol_id: 1, bid: 100, ask: 101, bid_volume: 1, ask_volume: 1, flags: 0 }),
            ReplayEvent::Tick(Tick { ts_ns: 2, recv_ns: 2, symbol_id: 1, bid: 105, ask: 106, bid_volume: 1, ask_volume: 1, flags: 0 }),
        ];
        let mut buf = Vec::new();
        write_events(&events, &mut buf).unwrap();
        let read_back = read_events(buf.as_slice()).unwrap();
        assert_eq!(read_back, events);
    }
}
