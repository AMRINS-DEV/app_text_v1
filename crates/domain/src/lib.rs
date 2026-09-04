//! Pure TradeOS domain types and ports. No I/O, no randomness (P5: the
//! execution/risk core is deterministic; all uncertainty is confined to the
//! agent layer and expressed as a probability number on `Signal`).
//!
//! Every type here is `#[repr(C)]` where it crosses a shared-memory ring
//! buffer, and every field matches `packages/proto/*.proto` 1:1 — see the
//! module doc comments for which boundary each type crosses.

pub mod enums;
pub mod ids;
pub mod order;
pub mod ports;
pub mod signal;
pub mod tick;

pub use enums::{Direction, RegimeTag, Side, SignalSource, TimeInForce, TradingMode, OrderType};
pub use ids::SymbolId;
pub use order::{BrokerOrderId, ExecEvent, OrderIntent};
pub use ports::{AccountSnapshot, Broker, FeedCaps, MarketDataSource, PortError, SymbolConstraints, SymbolSpec, Timeframe};
pub use signal::Signal;
pub use tick::{ArchivedTick, Bar, Tick, TickFlags};
