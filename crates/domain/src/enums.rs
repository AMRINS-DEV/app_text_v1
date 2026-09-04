use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Direction {
    Long,
    Short,
    Flat,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TimeInForce {
    Gtc,
    Ioc,
    Fok,
}

/// Risk profiles applied to the same code path — never separate strategy
/// implementations (design doc §9.1).
#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
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
