use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::{Direction, RegimeTag, SignalSource};
use crate::ids::SymbolId;

/// An agent/model/rule's advisory opinion. Signals have **zero**
/// order-placement authority (design doc P2, §10.3) — the strategy VM in
/// `crates/strategy` fuses them (§8.4) and the risk/execution core is the
/// only component that can emit an `OrderIntent`.
///
/// Crosses process boundaries over NATS JetStream / gRPC as the protobuf
/// message in `packages/proto/signal.proto` — this type is the in-process
/// Rust-native representation used by the strategy VM and fusion logic.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Signal {
    /// ULID, represented as a u128.
    pub id: u128,
    pub source: SignalSource,
    pub symbol_id: SymbolId,
    pub direction: Direction,
    /// Calibrated P(target hit before stop) — never a raw model score (§8.3).
    pub probability: f32,
    /// The model/agent's own uncertainty in [0,1].
    pub confidence: f32,
    /// Expected return in R multiples.
    pub expected_r: f32,
    /// Validity horizon of the thesis, milliseconds.
    pub horizon_ms: u64,
    /// Hard expiry (nanoseconds since epoch) — core discards after this,
    /// unconditionally, per P6 (fail closed).
    pub ttl_ns: u64,
    pub regime: RegimeTag,
    /// Reproducibility anchor: hash of the feature snapshot this signal was
    /// computed against. The core rejects any signal whose hash doesn't
    /// match a known feature snapshot (§10.3) to prevent stale/hallucinated
    /// context from reaching the strategy VM.
    pub features_hash: u64,
    /// Pointer to the Postgres/graph explanation record.
    pub evidence_ref: Option<Uuid>,
}

impl Signal {
    #[inline]
    pub fn is_expired(&self, now_ns: u64) -> bool {
        now_ns >= self.ttl_ns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Signal {
        Signal {
            id: 1,
            source: SignalSource::Model("gbdt-v3".into()),
            symbol_id: 1,
            direction: Direction::Long,
            probability: 0.58,
            confidence: 0.7,
            expected_r: 0.4,
            horizon_ms: 60_000,
            ttl_ns: 1_000,
            regime: RegimeTag::Trending,
            features_hash: 42,
            evidence_ref: None,
        }
    }

    #[test]
    fn expiry_is_inclusive_and_fails_closed() {
        let s = sample();
        assert!(!s.is_expired(999));
        assert!(s.is_expired(1_000));
        assert!(s.is_expired(1_001));
    }

    #[test]
    fn serde_roundtrip() {
        let s = sample();
        let json = serde_json::to_string(&s).unwrap();
        let back: Signal = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
