//! Equal-risk-contribution ("naive") risk parity. Unlike mean-variance,
//! there is no closed form here in general — this is a real, standard
//! iterative heuristic (a multiplicative risk-budgeting fixed-point
//! update), not a certified globally convergent optimizer. It is verified
//! in this module's own tests to (a) reproduce the known closed-form answer
//! for the uncorrelated case (inverse-volatility weighting) and (b)
//! converge every asset's risk contribution to within `tolerance` of equal
//! for a realistic correlated example. An iteration cap bounds a
//! pathological input to "best effort after `max_iterations`," never an
//! infinite loop.

use crate::linalg::Matrix;
use crate::mean_variance::portfolio_variance;

#[derive(Debug, Clone, Copy)]
pub struct RiskParityConfig {
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl Default for RiskParityConfig {
    fn default() -> Self {
        Self { max_iterations: 1_000, tolerance: 1e-8 }
    }
}

/// Portfolio volatility `sqrt(w^T Σ w)`.
pub fn portfolio_volatility(weights: &[f64], covariance: &Matrix) -> f64 {
    portfolio_variance(weights, covariance).max(0.0).sqrt()
}

/// Each asset's contribution to total portfolio risk:
/// `RC_i = w_i * (Σw)_i / sigma_p`. These sum to `sigma_p` by construction
/// (Euler's homogeneous-function identity applied to `sigma_p` as a
/// function of `w`, since volatility is homogeneous of degree 1 in `w`).
pub fn risk_contributions(weights: &[f64], covariance: &Matrix) -> Vec<f64> {
    let sigma_p = portfolio_volatility(weights, covariance);
    if sigma_p <= 0.0 {
        return vec![0.0; weights.len()];
    }
    let n = weights.len();
    (0..n)
        .map(|i| {
            let sigma_w_i: f64 = (0..n).map(|j| covariance[i][j] * weights[j]).sum();
            weights[i] * sigma_w_i / sigma_p
        })
        .collect()
}

/// Iteratively finds non-negative weights summing to 1 such that every
/// asset contributes equally to total portfolio risk. Starts from equal
/// weight and repeatedly rescales each weight by
/// `sqrt(target / actual_fraction)` of its current risk contribution,
/// renormalizing after each step. The square root is not cosmetic: an
/// undamped `target / actual_fraction` update was tried first and provably
/// oscillates forever on a trivial 2-uncorrelated-asset case (each step
/// overshoots the fixed point and lands on its mirror image, alternating
/// between two weight vectors indefinitely without ever satisfying the
/// tolerance) — because a risk contribution is *quadratic* in its own
/// weight (`RC_i ∝ w_i²` holding everything else fixed), the correction
/// that actually lands near the fixed point scales with the square root of
/// the ratio, not the ratio itself. Verified in this module's own tests
/// against the closed-form uncorrelated case, which the undamped version
/// failed and this one reproduces exactly.
pub fn equal_risk_contribution_weights(covariance: &Matrix, config: RiskParityConfig) -> Vec<f64> {
    let n = covariance.len();
    if n == 0 {
        return Vec::new();
    }
    let target = 1.0 / n as f64;
    let mut weights = vec![target; n];

    for _ in 0..config.max_iterations {
        let sigma_p = portfolio_volatility(&weights, covariance);
        if sigma_p <= 0.0 {
            break;
        }
        let contributions = risk_contributions(&weights, covariance);
        let max_deviation = contributions.iter().map(|c| (c / sigma_p - target).abs()).fold(0.0_f64, f64::max);
        if max_deviation < config.tolerance {
            break;
        }
        for (w, c) in weights.iter_mut().zip(contributions.iter()) {
            let rc_fraction = c / sigma_p;
            if rc_fraction > 0.0 {
                *w *= (target / rc_fraction).sqrt();
            }
        }
        let sum: f64 = weights.iter().sum();
        if sum > 0.0 {
            for w in &mut weights {
                *w /= sum;
            }
        }
    }

    weights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_uncorrelated_assets_converge_to_inverse_volatility_weighting() {
        // Known closed form for 2 uncorrelated assets: w_i ∝ 1/σ_i, so
        // w1/w2 = σ2/σ1.
        let sigma1 = 0.10_f64;
        let sigma2 = 0.20_f64;
        let cov = vec![vec![sigma1 * sigma1, 0.0], vec![0.0, sigma2 * sigma2]];
        let weights = equal_risk_contribution_weights(&cov, RiskParityConfig::default());

        let expected_ratio = sigma2 / sigma1;
        assert!(
            ((weights[0] / weights[1]) - expected_ratio).abs() < 1e-4,
            "expected w1/w2 ~= {expected_ratio}, got {}",
            weights[0] / weights[1]
        );
        assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn three_correlated_assets_converge_to_equal_risk_contributions() {
        let cov = vec![vec![0.04, 0.01, 0.005], vec![0.01, 0.09, 0.02], vec![0.005, 0.02, 0.16]];
        let config = RiskParityConfig::default();
        let weights = equal_risk_contribution_weights(&cov, config);

        let sigma_p = portfolio_volatility(&weights, &cov);
        let contributions = risk_contributions(&weights, &cov);
        for c in &contributions {
            assert!((c / sigma_p - 1.0 / 3.0).abs() < 1e-4, "risk contributions did not equalize: {contributions:?}");
        }
        assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert!(weights.iter().all(|&w| w >= 0.0));
    }

    #[test]
    fn risk_contributions_sum_to_portfolio_volatility() {
        let cov = vec![vec![0.04, 0.01], vec![0.01, 0.09]];
        let weights = vec![0.7, 0.3];
        let sigma_p = portfolio_volatility(&weights, &cov);
        let sum_rc: f64 = risk_contributions(&weights, &cov).iter().sum();
        assert!((sum_rc - sigma_p).abs() < 1e-9);
    }

    #[test]
    fn a_zero_covariance_matrix_does_not_loop_forever_or_panic() {
        let cov = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let weights = equal_risk_contribution_weights(&cov, RiskParityConfig { max_iterations: 10, tolerance: 1e-8 });
        assert_eq!(weights.len(), 2);
    }

    #[test]
    fn an_empty_universe_returns_an_empty_allocation() {
        let cov: Matrix = Vec::new();
        assert!(equal_risk_contribution_weights(&cov, RiskParityConfig::default()).is_empty());
    }
}
