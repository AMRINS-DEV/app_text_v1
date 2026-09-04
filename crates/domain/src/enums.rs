use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Direction {
    Long,
    Short,
    Flat,
}

// Side/OrderType/TimeInForce/TradingMode also derive rkyv: they appear in
// `OrderIntent`, which crosses the MT5 bridge wire on the hot path (§5.2:
// "Order encode + send to bridge: 5-20 µs") and must zero-copy like `Tick`.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TimeInForce {
    Gtc,
    Ioc,
    Fok,
}

/// Risk profiles applied to the same code path — never separate strategy
/// implementations (design doc §9.1).
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TradingMode {
    Normal,
    Aggressive,
    Scalp,
    Paper,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RegimeTag {
    Trending,
    Ranging,
    Expansion,
    HighVolChoppy,
}

/// Attribution for a `Signal`: which agent, model artifact, or deterministic
/// rule produced it. Kept as data (not a trait object) so it can be logged,
/// replayed, and used as a fusion-weighting key (§8.4).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum SignalSource {
    Agent(String),
    Model(String),
    Rule(String),
}
