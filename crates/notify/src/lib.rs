//! Outbound notification port. Every §9.5 safety guard trip and the kill
//! switch must reach a human through at least one of these channels.

use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("delivery failed: {0}")]
    Delivery(String),
}

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, message: &str) -> std::result::Result<(), NotifyError>;
}
