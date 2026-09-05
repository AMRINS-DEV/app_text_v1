//! A small, from-scratch Cholesky decomposition and triangular solver for
//! symmetric positive-definite matrices — exactly the shape a real
//! covariance matrix has (in the non-degenerate case). Deliberately not a
//! dependency on `nalgebra`/`ndarray-linalg`: every matrix this crate ever
//! inverts is small (one row/column per strategy or account, not per
//! tick), so a compact, auditable solver here is worth more than a large
//! general-purpose linear algebra dependency for a handful of solves — the
//! same "build it, the scale doesn't need the dependency" call Phase 7 made
//! for its own backtest runner over NautilusTrader.

pub type Matrix = Vec<Vec<f64>>;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LinAlgError {
    #[error("matrix must be square, got {rows}x{cols}")]
    NotSquare { rows: usize, cols: usize },
    #[error("matrix and vector dimensions do not match: {matrix_dim}x{matrix_dim} vs length {vector_len}")]
    DimensionMismatch { matrix_dim: usize, vector_len: usize },
    #[error(
        "matrix is not positive definite (failed at pivot {0}) — likely a singular or \
         near-singular covariance matrix (e.g. duplicate/perfectly correlated assets)"
    )]
    NotPositiveDefinite(usize),
}

/// A pivot at or below this is treated as "not positive definite," not just
/// an exact zero-or-negative check. A matrix that is exactly singular in
/// real arithmetic (e.g. two literally duplicate assets/rows) generally
/// does *not* land on an exact `0.0` pivot once floating-point rounding
/// accumulates through the decomposition — verified directly: a hand-built
/// duplicate-row 2x2 case landed on a pivot of `6.9e-18`, not `0.0`, purely
/// from `sqrt`/division rounding. `1e-10` is far above that noise floor and
/// far below any realistic return-series variance (rarely below `1e-8`),
/// so it separates "genuinely singular" from "legitimately small variance"
/// without misclassifying either in practice.
const PIVOT_EPSILON: f64 = 1e-10;

/// Decomposes symmetric positive-definite `a` into lower-triangular `l`
/// such that `l * l^T == a`. Returns `NotPositiveDefinite` the moment a
/// diagonal pivot would require the square root of a value at or below
/// `PIVOT_EPSILON` — seeing `PIVOT_EPSILON`'s own doc comment for why an
/// exact `<= 0.0` check is not reliable here.
pub fn cholesky(a: &Matrix) -> Result<Matrix, LinAlgError> {
    let n = a.len();
    if a.iter().any(|row| row.len() != n) {
        return Err(LinAlgError::NotSquare { rows: n, cols: a.first().map(|r| r.len()).unwrap_or(0) });
    }
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i][j];
            for (l_ik, l_jk) in l[i].iter().zip(l[j].iter()).take(j) {
                sum -= l_ik * l_jk;
            }
            if i == j {
                if sum <= PIVOT_EPSILON {
                    return Err(LinAlgError::NotPositiveDefinite(i));
                }
                l[i][j] = sum.sqrt();
            } else {
                l[i][j] = sum / l[j][j];
            }
        }
    }
    Ok(l)
}

/// Solves `a * x = b` for symmetric positive-definite `a` via its Cholesky
/// factor: forward-substitute `l*y = b`, then back-substitute `l^T*x = y`.
pub fn solve_spd(a: &Matrix, b: &[f64]) -> Result<Vec<f64>, LinAlgError> {
    let n = a.len();
    if b.len() != n {
        return Err(LinAlgError::DimensionMismatch { matrix_dim: n, vector_len: b.len() });
    }
    let l = cholesky(a)?;

    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut sum = b[i];
        for (k, y_k) in y.iter().enumerate().take(i) {
            sum -= l[i][k] * y_k;
        }
        y[i] = sum / l[i][i];
    }

    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = y[i];
        for k in (i + 1)..n {
            sum -= l[k][i] * x[k]; // l^T[i][k] == l[k][i]
        }
        x[i] = sum / l[i][i];
    }
    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cholesky_of_identity_is_identity() {
        let a = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        assert_eq!(cholesky(&a).unwrap(), a);
    }

    #[test]
    fn cholesky_reconstructs_the_original_matrix() {
        let a = vec![vec![4.0, 2.0], vec![2.0, 3.0]];
        let l = cholesky(&a).unwrap();
        for i in 0..2 {
            for j in 0..2 {
                let recon: f64 = (0..2).map(|k| l[i][k] * l[j][k]).sum();
                assert!((recon - a[i][j]).abs() < 1e-9, "reconstruction mismatch at ({i},{j})");
            }
        }
    }

    #[test]
    fn a_non_positive_definite_matrix_is_rejected() {
        // [[1,2],[2,1]] has eigenvalues -1 and 3 -- not SPD.
        let a = vec![vec![1.0, 2.0], vec![2.0, 1.0]];
        assert!(matches!(cholesky(&a), Err(LinAlgError::NotPositiveDefinite(_))));
    }

    #[test]
    fn a_non_square_matrix_is_rejected() {
        let a = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];
        assert!(matches!(cholesky(&a), Err(LinAlgError::NotSquare { rows: 2, cols: 3 })));
    }

    #[test]
    fn solve_spd_produces_a_solution_verified_by_direct_substitution() {
        let a = vec![vec![4.0, 2.0], vec![2.0, 3.0]];
        let b = vec![1.0, 2.0];
        let x = solve_spd(&a, &b).unwrap();
        for i in 0..2 {
            let recon: f64 = (0..2).map(|j| a[i][j] * x[j]).sum();
            assert!((recon - b[i]).abs() < 1e-9);
        }
    }

    #[test]
    fn solve_spd_handles_a_diagonal_matrix_trivially() {
        let a = vec![vec![2.0, 0.0], vec![0.0, 5.0]];
        let x = solve_spd(&a, &[4.0, 10.0]).unwrap();
        assert!((x[0] - 2.0).abs() < 1e-12);
        assert!((x[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn dimension_mismatch_is_rejected() {
        let a = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        assert!(matches!(solve_spd(&a, &[1.0, 2.0, 3.0]), Err(LinAlgError::DimensionMismatch { matrix_dim: 2, vector_len: 3 })));
    }
}
