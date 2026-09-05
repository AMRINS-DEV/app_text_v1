//! Turning normalized portfolio weights into dollar (or account-currency)
//! capital allocations — the last step connecting this crate's optimizers
//! to a real total-equity figure such as
//! `execution::AccountManager::aggregate_snapshot().equity`. Kept as a
//! plain `f64` parameter here rather than a dependency on `execution`, so
//! this crate stays a pure, independently testable numerical library; the
//! wiring itself is one multiplication at the call site.

/// `weights` need not already sum to exactly 1 (floating-point drift from
/// an iterative solver like `risk_parity::equal_risk_contribution_weights`
/// is real) — this renormalizes defensively before scaling, so the
/// returned allocations always sum to `total_equity` up to floating-point
/// rounding.
pub fn allocate_capital(weights: &[f64], total_equity: f64) -> Vec<f64> {
    let sum: f64 = weights.iter().sum();
    if sum.abs() < 1e-12 {
        let n = weights.len();
        if n == 0 {
            return Vec::new();
        }
        return vec![total_equity / n as f64; n];
    }
    weights.iter().map(|w| (w / sum) * total_equity).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_summing_to_one_scale_directly() {
        let allocations = allocate_capital(&[0.5, 0.3, 0.2], 100_000.0);
        assert_eq!(allocations, vec![50_000.0, 30_000.0, 20_000.0]);
    }

    #[test]
    fn weights_not_summing_to_one_are_renormalized_first() {
        let allocations = allocate_capital(&[1.0, 1.0], 100_000.0);
        assert_eq!(allocations, vec![50_000.0, 50_000.0]);
    }

    #[test]
    fn allocations_always_sum_to_total_equity() {
        let allocations = allocate_capital(&[0.2, 0.5, 0.9], 40_000.0);
        let sum: f64 = allocations.iter().sum();
        assert!((sum - 40_000.0).abs() < 1e-6);
    }

    #[test]
    fn all_zero_weights_fall_back_to_equal_split_rather_than_dividing_by_zero() {
        let allocations = allocate_capital(&[0.0, 0.0], 10_000.0);
        assert_eq!(allocations, vec![5_000.0, 5_000.0]);
    }

    #[test]
    fn an_empty_universe_allocates_nothing() {
        assert!(allocate_capital(&[], 10_000.0).is_empty());
    }
}
