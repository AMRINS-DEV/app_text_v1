//! Order router. The only crate allowed to call `domain::Broker::submit`.
//! Idempotency-key deduplication is implemented (it's a pure, testable
//! concern); the actual routing against a live/simulated broker is Phase 2.

use std::collections::HashSet;

/// Tracks `OrderIntent::client_id`s already submitted so a resubmission
/// (e.g. after a retry) is a no-op rather than a duplicate order.
#[derive(Default)]
pub struct IdempotencyGuard {
    seen: HashSet<u128>,
}

impl IdempotencyGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` the first time this `client_id` is seen, `false` on
    /// every resubmission.
    pub fn admit(&mut self, client_id: u128) -> bool {
        self.seen.insert(client_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_submission_is_admitted_resubmission_is_not() {
        let mut guard = IdempotencyGuard::new();
        assert!(guard.admit(1));
        assert!(!guard.admit(1));
        assert!(guard.admit(2));
    }
}
