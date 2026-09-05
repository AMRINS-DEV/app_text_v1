//! Order router + simulated broker (§5.1 exec stage, §9.3, §9.5). The only
//! crate allowed to call `domain::Broker::submit`. §17 Phase 9 adds
//! multi-account dispatch (`account_manager`) on top of the same
//! `OrderRouter` every prior phase already built — see that module's own
//! doc comment.

pub mod account_manager;
pub mod idempotency;
pub mod router;
pub mod sim_broker;

pub use account_manager::{AccountId, AccountManager, AggregateSnapshot};
pub use idempotency::IdempotencyGuard;
pub use router::OrderRouter;
pub use sim_broker::{SimBroker, SimBrokerConfig};
