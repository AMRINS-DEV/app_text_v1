//! Markowitz mean-variance portfolios: minimum-variance and (unconstrained)
//! maximum-Sharpe/tangency weights, both via the closed-form solution
//! `Σ⁻¹ · target`, normalized to sum to 1 — computed through
//! `linalg::solve_spd` rather than an explicit matrix inverse (numerically
//! the standard choice: solving is cheaper and better-conditioned than
//! inverting then multiplying).

use crate::linalg::{solve_spd, LinAlgError, Matrix};

/// The global minimum-variance portfolio: `w ∝ Σ⁻¹ · 1`. Ignores expected
/// returns entirely (by design — the GMV portfolio only cares about risk).
pub fn minimum_variance_weights(covariance: &Matrix) -> Result<Vec<f64>, LinAlgError> {
    let n = covariance.len();
    let ones = vec![1.0; n];
    let raw = solve_spd(covariance, &ones)?;
    Ok(normalize(&raw))
}

/// The tangency (maximum Sharpe ratio) portfolio: `w ∝ Σ⁻¹ · (μ - r_f)`.
/// Unconstrained — it can produce negative (short) weights, which is the
/// mathematically correct unconstrained optimum, not a bug. A long-only
/// portfolio is a different, inequality-constrained optimization problem
/// this closed form does not solve; see `long_only_max_sharpe_weights` for
/// a documented, simpler approximation instead.
///
/// Known degenerate case, not handled specially here: if the *aggregate*
/// unnormalized exposure `1ᵀΣ⁻¹(μ-r_f)` is negative (every asset's raw,
/// pre-normalization tangency exposure sums to a negative number — e.g. no
/// asset offers an attractive risk-adjusted excess return at all),
/// normalizing by it flips every weight's sign, silently returning the
/// *minimum*-Sharpe portfolio instead of the maximum one (a documented
/// two-fund-separation-theorem corner case, not specific to this
/// implementation). Detecting and handling that case is out of this
/// function's scope; callers with expected returns that could plausibly be
/// collectively unattractive should check `expected_returns.iter().sum()`
/// against `risk_free_rate` themselves before trusting the sign of the
/// result.
pub fn max_sharpe_weights(expected_returns: &[f64], covariance: &Matrix, risk_free_rate: f64) -> Result<Vec<f64>, LinAlgError> {
    let excess: Vec<f64> = expected_returns.iter().map(|r| r - risk_free_rate).collect();
    let raw = solve_spd(covariance, &excess)?;
    Ok(normalize(&raw))
}

/// A practical, explicitly documented approximation to long-only
/// max-Sharpe: clamp the unconstrained tangency weights at zero (drop short
/// positions) and renormalize the remainder to sum to 1. This is *not* the
/// globally optimal long-only portfolio — that requires quadratic
/// programming with inequality constraints, out of this crate's scope —
/// the same "real logic, honestly narrower scope than the full problem"
/// choice this project makes elsewhere (e.g. `agents_validation.backtest`'s
/// fixed cost-in-R standing in for a full market-impact model).
pub fn long_only_max_sharpe_weights(expected_returns: &[f64], covariance: &Matrix, risk_free_rate: f64) -> Result<Vec<f64>, LinAlgError> {
    let raw = max_sharpe_weights(expected_returns, covariance, risk_free_rate)?;
    let clamped: Vec<f64> = raw.iter().map(|w| w.max(0.0)).collect();
    Ok(normalize(&clamped))
}

fn normalize(weights: &[f64]) -> Vec<f64> {
    let sum: f64 = weights.iter().sum();
    if sum.abs() < 1e-12 {
        // Degenerate (e.g. every weight clamped to zero): fall back to
        // equal weight rather than dividing by ~zero.
        let n = weights.len();
        return vec![1.0 / n as f64; n];
    }
    weights.iter().map(|w| w / sum).collect()
}

pub fn portfolio_variance(weights: &[f64], covariance: &Matrix) -> f64 {
    let n = weights.len();
    let mut variance = 0.0;
    for i in 0..n {
        for j in 0..n {
            variance += weights[i] * covariance[i][j] * weights[j];
        }
    }
    variance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimum_variance_weights_sum_to_one() {
        let cov = vec![vec![0.04, 0.0, 0.0], vec![0.0, 0.09, 0.0], vec![0.0, 0.0, 0.01]];
        let weights = minimum_variance_weights(&cov).unwrap();
        assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn minimum_variance_weights_are_inverse_variance_when_uncorrelated() {
        // For a diagonal covariance matrix the GMV closed form reduces to
        // w_i ∝ 1/σ_i² -- a hand-verifiable special case of Σ⁻¹·1.
        let cov = vec![vec![0.04, 0.0], vec![0.0, 0.01]]; // σ1²=0.04, σ2²=0.01
        let weights = minimum_variance_weights(&cov).unwrap();
        let expected_ratio = (1.0 / 0.01) / (1.0 / 0.04); // w2/w1
        assert!(((weights[1] / weights[0]) - expected_ratio).abs() < 1e-9);
        // The lower-variance asset should get more weight.
        assert!(weights[1] > weights[0]);
    }

    #[test]
    fn max_sharpe_weights_reduce_to_return_over_variance_when_uncorrelated() {
        let cov = vec![vec![0.04, 0.0], vec![0.0, 0.01]];
        let returns = vec![0.08, 0.02];
        let weights = max_sharpe_weights(&returns, &cov, 0.0).unwrap();
        // Diagonal Σ: Σ⁻¹μ component i is μ_i/σ_i².
        let raw0 = 0.08 / 0.04;
        let raw1 = 0.02 / 0.01;
        let expected_ratio = raw1 / raw0;
        assert!(((weights[1] / weights[0]) - expected_ratio).abs() < 1e-9);
    }

    #[test]
    fn long_only_clamps_negative_weights_and_still_sums_to_one() {
        let cov = vec![vec![0.04, 0.0], vec![0.0, 0.01]];
        // A negative expected return on asset 1 pushes its unconstrained
        // tangency weight negative, while asset 2's strong positive return
        // keeps the *aggregate* raw exposure positive -- avoiding the
        // documented negative-aggregate-denominator corner case
        // `max_sharpe_weights` itself calls out (which would flip every
        // sign on normalization and defeat this test's premise).
        let returns = vec![-0.02, 0.10];
        let unconstrained = max_sharpe_weights(&returns, &cov, 0.0).unwrap();
        assert!(unconstrained[0] < 0.0, "test setup should produce a short position to clamp");

        let long_only = long_only_max_sharpe_weights(&returns, &cov, 0.0).unwrap();
        assert_eq!(long_only[0], 0.0);
        assert!((long_only.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert!(long_only.iter().all(|&w| w >= 0.0));
    }

    #[test]
    fn a_singular_covariance_matrix_from_duplicate_assets_is_rejected() {
        // Two assets with identical returns in every period -- perfectly
        // correlated, so the covariance matrix is singular.
        let cov = vec![vec![0.04, 0.04], vec![0.04, 0.04]];
        assert!(minimum_variance_weights(&cov).is_err());
    }

    #[test]
    fn portfolio_variance_matches_a_hand_computed_two_asset_example() {
        let weights = vec![0.6, 0.4];
        let cov = vec![vec![0.04, 0.01], vec![0.01, 0.09]];
        // w^T Σ w = 0.6²*0.04 + 2*0.6*0.4*0.01 + 0.4²*0.09
        let expected = 0.36 * 0.04 + 2.0 * 0.6 * 0.4 * 0.01 + 0.16 * 0.09;
        assert!((portfolio_variance(&weights, &cov) - expected).abs() < 1e-12);
    }
}
