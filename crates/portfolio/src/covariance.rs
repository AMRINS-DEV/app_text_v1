//! Sample mean/covariance estimation from a historical returns matrix — the
//! real statistical input every allocator in this crate needs. §17 Phase
//! 9's "portfolio optimizer" has to estimate risk/return from *something*;
//! this module is that something, computed for real rather than assumed by
//! the caller.

use crate::linalg::Matrix;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CovarianceError {
    #[error("returns matrix must have at least 2 periods to estimate a sample covariance, got {0}")]
    TooFewPeriods(usize),
    #[error("all return rows must have the same number of assets")]
    RaggedRows,
}

/// `returns[t][i]` is asset `i`'s return in period `t`. Sample mean per asset.
pub fn sample_mean(returns: &[Vec<f64>]) -> Result<Vec<f64>, CovarianceError> {
    let n_periods = returns.len();
    if n_periods == 0 {
        return Err(CovarianceError::TooFewPeriods(n_periods));
    }
    let n_assets = returns[0].len();
    if returns.iter().any(|row| row.len() != n_assets) {
        return Err(CovarianceError::RaggedRows);
    }
    let mut mean = vec![0.0; n_assets];
    for row in returns {
        for (i, &r) in row.iter().enumerate() {
            mean[i] += r;
        }
    }
    for m in &mut mean {
        *m /= n_periods as f64;
    }
    Ok(mean)
}

/// Unbiased (`n - 1` denominator) sample covariance matrix.
pub fn sample_covariance(returns: &[Vec<f64>]) -> Result<Matrix, CovarianceError> {
    let n_periods = returns.len();
    if n_periods < 2 {
        return Err(CovarianceError::TooFewPeriods(n_periods));
    }
    let mean = sample_mean(returns)?;
    let n_assets = mean.len();
    let mut cov = vec![vec![0.0; n_assets]; n_assets];
    for row in returns {
        for i in 0..n_assets {
            for j in 0..n_assets {
                cov[i][j] += (row[i] - mean[i]) * (row[j] - mean[j]);
            }
        }
    }
    let denom = (n_periods - 1) as f64;
    for row in &mut cov {
        for v in row {
            *v /= denom;
        }
    }
    Ok(cov)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_mean_of_a_single_asset() {
        let returns = vec![vec![0.1], vec![0.2], vec![0.3]];
        let mean = sample_mean(&returns).unwrap();
        assert!((mean[0] - 0.2).abs() < 1e-12);
    }

    #[test]
    fn ragged_rows_are_rejected() {
        let returns = vec![vec![0.1, 0.2], vec![0.1]];
        assert_eq!(sample_mean(&returns), Err(CovarianceError::RaggedRows));
    }

    #[test]
    fn fewer_than_two_periods_is_rejected() {
        let returns = vec![vec![0.1, 0.2]];
        assert_eq!(sample_covariance(&returns), Err(CovarianceError::TooFewPeriods(1)));
    }

    #[test]
    fn two_perfectly_correlated_assets_have_equal_variance_and_covariance() {
        // Asset 2's return is always exactly 2x asset 1's -- perfectly
        // correlated, with a hand-computable variance ratio.
        let returns = vec![vec![0.01, 0.02], vec![0.02, 0.04], vec![-0.01, -0.02], vec![0.03, 0.06]];
        let cov = sample_covariance(&returns).unwrap();
        // Var(2X) = 4*Var(X); Cov(X,2X) = 2*Var(X).
        assert!((cov[1][1] - 4.0 * cov[0][0]).abs() < 1e-12);
        assert!((cov[0][1] - 2.0 * cov[0][0]).abs() < 1e-12);
        assert!((cov[0][1] - cov[1][0]).abs() < 1e-15, "covariance matrix must be symmetric");
    }

    #[test]
    fn independent_alternating_returns_have_zero_sample_covariance() {
        // Asset 1 alternates +1/-1 in step with the row index; asset 2
        // alternates on a different (orthogonal) pattern -- their sample
        // covariance should be exactly zero by construction.
        let returns = vec![vec![1.0, 1.0], vec![1.0, -1.0], vec![-1.0, 1.0], vec![-1.0, -1.0]];
        let cov = sample_covariance(&returns).unwrap();
        assert!(cov[0][1].abs() < 1e-12);
    }
}
