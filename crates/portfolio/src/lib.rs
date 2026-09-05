//! Portfolio-level capital allocation across concurrent strategies/accounts
//! (§17 Phase 9 scope: "portfolio optimizer"). Sits above `crates/risk`'s
//! per-trade Kelly sizing — Kelly answers "how much to risk on *this*
//! trade," this crate answers "how much capital does *this strategy/
//! account* get in the first place." Real math throughout: a from-scratch
//! Cholesky-based SPD solver (`linalg`), sample covariance estimation
//! (`covariance`), closed-form Markowitz mean-variance weights
//! (`mean_variance`), and an iterative equal-risk-contribution risk-parity
//! solver (`risk_parity`) — see each module's own doc comment for what's
//! closed-form vs. heuristic and why.

pub mod allocation;
pub mod covariance;
pub mod linalg;
pub mod mean_variance;
pub mod risk_parity;

pub use allocation::allocate_capital;
pub use covariance::{sample_covariance, sample_mean, CovarianceError};
pub use linalg::{cholesky, solve_spd, LinAlgError, Matrix};
pub use mean_variance::{long_only_max_sharpe_weights, max_sharpe_weights, minimum_variance_weights, portfolio_variance};
pub use risk_parity::{equal_risk_contribution_weights, portfolio_volatility, risk_contributions, RiskParityConfig};

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// End-to-end: historical returns -> covariance -> risk-parity weights
    /// -> dollar allocations, all through this crate's own public API, with
    /// no step's output treated as a given.
    #[test]
    fn full_pipeline_from_historical_returns_to_dollar_allocations() {
        let returns = vec![
            vec![0.01, -0.02, 0.03],
            vec![-0.01, 0.01, -0.01],
            vec![0.02, 0.00, 0.02],
            vec![0.00, -0.01, 0.01],
            vec![0.01, 0.02, -0.02],
        ];
        let cov = sample_covariance(&returns).unwrap();
        let weights = equal_risk_contribution_weights(&cov, RiskParityConfig::default());
        assert_eq!(weights.len(), 3);
        assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1e-6);

        let allocations = allocate_capital(&weights, 250_000.0);
        assert_eq!(allocations.len(), 3);
        assert!((allocations.iter().sum::<f64>() - 250_000.0).abs() < 1e-3);
    }
}
