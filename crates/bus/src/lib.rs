//! Event bus abstractions. Two implementations are expected in later phases:
//! a NATS JetStream adapter (durable, replayable, cross-process — signals,
//! orders, fills) and an `iceoryx2`/`rtrb` ring-buffer adapter (hot path,
//! same-process ingest -> feature -> strategy -> risk pipeline, §5.1).
//! Everything published here must also be durable-loggable per P4: the
//! whole system replays deterministically from any timestamp.

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("not connected")]
    NotConnected,
    #[error("publish failed: {0}")]
    Publish(String),
    #[error("subscribe failed: {0}")]
    Subscribe(String),
}

pub type Result<T> = std::result::Result<T, BusError>;

/// Durable, replayable publish/subscribe (NATS JetStream in production).
/// This is the *cold-path* bus for signals/orders/fills crossing process
/// boundaries — the hot-path SPSC rings between core threads are a
/// separate, allocation-free mechanism (see `crates/market-data`, §5.1).
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish<T: Serialize + Send + Sync>(&self, subject: &str, event: &T) -> Result<()>;
    async fn subscribe<T: DeserializeOwned + Send>(&self, subject: &str) -> Result<Box<dyn Subscription<T>>>;
}

#[async_trait]
pub trait Subscription<T>: Send {
    async fn next(&mut self) -> Option<T>;
}
