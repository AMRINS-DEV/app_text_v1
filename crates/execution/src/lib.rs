//! Order router + simulated broker (§5.1 exec stage, §9.3, §9.5). The only
//! crate allowed to call `domain::Broker::submit`.

pub mod idempotency;
pub mod router;
pub mod sim_broker;

pub use idempotency::IdempotencyGuard;
pub use router::OrderRouter;
pub use sim_broker::{SimBroker, SimBrokerConfig};
