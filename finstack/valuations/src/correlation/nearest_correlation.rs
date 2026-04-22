//! Higham (2002) nearest correlation matrix projection.
//!
//! Given a symmetric matrix `A` that is *approximately* a correlation matrix
//! (e.g. a sample correlation corrupted by estimation noise or a user-supplied
//! matrix with small PSD violations), this module finds the closest valid
//! correlation matrix in Frobenius norm:
//!
//! ```text
//! min ‖X − A‖_F
//!   s.t. X = Xᵀ, diag(X) = 1, X ⪰ 0.
//! ```
//!
//! Higham's alternating-projections algorithm uses Dykstra's correction to
//! iteratively project onto the PSD cone and the "unit diagonal" hyperplane
//! until convergence. This is the standard remedy for real-world correlation
//! matrices that fail Cholesky by a small margin.
//!
//! # References
//!
//! - Higham, N. J. (2002). "Computing the nearest correlation matrix — A
//!   problem from finance." *IMA Journal of Numerical Analysis*, 22(3), 329–343.
//! - Borsdorf, R., & Higham, N. J. (2010). "A preconditioned Newton algorithm
//!   for the nearest correlation matrix." *IMA Journal of Numerical Analysis*,
//!   30(1), 94–107.
//!
//! # When to use
//!
//! Use `nearest_correlation_matrix` when an upstream pipeline produces a matrix
//! that *should* be a correlation matrix but has small numerical defects
//! (typical causes: thresholded sample estimates, shrinkage, missing-data
//! imputation, user-edited blocks). When the input is wildly off — e.g.
//! asymmetric by more than rounding, diagonal very far from 1, or the target
//! application rejects any silent modification — validate first and let the
//! caller decide.

use super::error::{Error, Result};

/// Convergence parameters for [`nearest_correlation_matrix`].
///
/// The defaults (`max_iter = 200`, `tol = 1e-10`) are a conservative balance
/// between runtime and accuracy for correlation matrices up to ~50×50 that
/// typically arise in credit and rates applications.
#[derive(Debug, Clone, Copy)]
pub struct NearestCorrelationOpts {
    /// Maximum number of alternating projection iterations.
    pub max_iter: usize,
    /// Frobenius-norm tolerance on the change between successive iterates.
    pub tol: f64,
}

impl Default for NearestCorrelationOpts {
    fn default() -> Self {
        Self {
            max_iter: 200,
            tol: 1e-10,
        }
    }
}

/// Compute the nearest correlation matrix to `input` using Higham's
/// alternating-projection algorithm with Dykstra's correction.
///
/// The input is expected to be nearly symmetric with a diagonal close to 1.
/// Gross violations (asymmetry beyond 1e-6, diagonal entries far from 1, etc.)
/// are rejected with an error rather than silently "fixed", because those
/// usually indicate upstream data corruption rather than numerical noise.
///
/// # Arguments
///
/// * `input` — Flattened `n × n` matrix in row-major order.
/// * `n`     — Matrix dimension.
/// * `opts`  — Convergence settings (use `NearestCorrelationOpts::default()`).
///
/// # Returns
///
/// A new `n × n` matrix (row-major) that is symmetric, has unit diagonal, and
/// is positive semidefinite.
///
/// # Errors
///
/// * [`Error::InvalidSize`] if `input.len() != n * n`.
/// * [`Error::NotSymmetric`] if the input deviates from symmetry by more than
///   `1e-6` at any off-diagonal entry.
/// * [`Error::DiagonalNotOne`] if any diagonal entry is further than `1e-3`
///   from `1.0`.
/// * [`Error::NotPositiveSemiDefinite`] if the algorithm fails to converge
///   within `opts.max_iter` iterations.
pub fn nearest_correlation_matrix(
    input: &[f64],
    n: usize,
    opts: NearestCorrelationOpts,
) -> Result<Vec<f64>> {
    if input.len() != n * n {
        return Err(Error::InvalidSize {
            expected: n,
            actual: input.len(),
        });
    }
    if n == 0 {
        return Ok(Vec::new());
    }

    // Sanity gates: reject pathological inputs. Small defects are fine —
    // those are precisely what the projection is designed to repair — but a
    // diagonal of 0.5 or a blatantly asymmetric entry is almost certainly a
    // data bug that must be surfaced, not silently reshaped.
    const SYMMETRY_GATE: f64 = 1e-6;
    const DIAGONAL_GATE: f64 = 1e-3;
    for i in 0..n {
        let diag = input[i * n + i];
        if (diag - 1.0).abs() > DIAGONAL_GATE {
            return Err(Error::DiagonalNotOne {
                index: i,
                value: diag,
            });
        }
        for j in (i + 1)..n {
            let diff = (input[i * n + j] - input[j * n + i]).abs();
            if diff > SYMMETRY_GATE {
                return Err(Error::NotSymmetric { i, j, diff });
            }
        }
    }

    // Symmetrize to machine precision before starting the projection so that
    // the Jacobi eigensolver gets a perfectly symmetric iterate.
    let mut y = symmetrize(input, n);
    let mut s = vec![0.0_f64; n * n];

    let mut prev = y.clone();
    for _ in 0..opts.max_iter {
        // Dykstra correction: r = y - s
        let mut r = vec![0.0; n * n];
        for k in 0..(n * n) {
            r[k] = y[k] - s[k];
        }

        let x = project_psd(&r, n);

        for k in 0..(n * n) {
            s[k] = x[k] - r[k];
        }

        // Project onto unit diagonal.
        let mut next = x;
        for i in 0..n {
            next[i * n + i] = 1.0;
        }

        if frobenius_diff(&next, &prev) < opts.tol {
            return Ok(next);
        }
        prev = next.clone();
        y = next;
    }

    Err(Error::NotPositiveSemiDefinite { row: 0 })
}

/// Project a symmetric matrix onto the PSD cone by zeroing negative
/// eigenvalues (the Higham projection step).
///
/// Audit P3 #34: spectral decomposition now delegates to
/// [`finstack_core::math::linalg::symmetric_eigen`] (divide-and-conquer
/// tridiagonal QR, `O(n³)`) instead of the previous hand-rolled Jacobi
/// sweeps. The old Jacobi path capped at `100·n²` sweeps and each sweep
/// was `O(n²)` for pivot search + `O(n)` per-rotation update,
/// degenerating to roughly `O(n⁵)` for `n > 40` correlation matrices —
/// enough to dominate Higham wall-time on portfolio-scale matrices.
fn project_psd(matrix: &[f64], n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }

    // Defensive: `symmetric_eigen` only fails on shape mismatch; we
    // already know `matrix.len() == n * n` here, but fall through to a
    // zero matrix if the invariant is ever broken so callers observe a
    // degenerate projection instead of a panic.
    let Ok((eigenvalues, eigenvectors)) = finstack_core::math::linalg::symmetric_eigen(matrix, n)
    else {
        return vec![0.0; n * n];
    };

    // Reconstruct using only non-negative eigenvalues: X = V · diag(max(λ, 0)) · Vᵀ.
    // `eigenvectors[i * n + k]` is the `i`-th component of the `k`-th
    // eigenvector, matching the previous Jacobi layout.
    let mut out = vec![0.0_f64; n * n];
    for i in 0..n {
        for j in i..n {
            let mut sum = 0.0_f64;
            for k in 0..n {
                let lambda = eigenvalues[k].max(0.0);
                if lambda == 0.0 {
                    continue;
                }
                sum += lambda * eigenvectors[i * n + k] * eigenvectors[j * n + k];
            }
            out[i * n + j] = sum;
            out[j * n + i] = sum;
        }
    }
    out
}

fn symmetrize(matrix: &[f64], n: usize) -> Vec<f64> {
    let mut out = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            out[i * n + j] = 0.5 * (matrix[i * n + j] + matrix[j * n + i]);
        }
    }
    out
}

fn frobenius_diff(a: &[f64], b: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), b.len());
    let mut acc = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let d = x - y;
        acc += d * d;
    }
    acc.sqrt()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::correlation::factor_model::validate_correlation_matrix;

    fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).abs())
            .fold(0.0_f64, f64::max)
    }

    #[test]
    fn identity_is_fixed_point() {
        let input = vec![1.0, 0.0, 0.0, 1.0];
        let repaired =
            nearest_correlation_matrix(&input, 2, NearestCorrelationOpts::default()).expect("ok");
        assert!(max_abs_diff(&input, &repaired) < 1e-12);
    }

    #[test]
    fn already_valid_matrix_is_unchanged() {
        let input = vec![1.0, 0.5, 0.3, 0.5, 1.0, 0.4, 0.3, 0.4, 1.0];
        let repaired =
            nearest_correlation_matrix(&input, 3, NearestCorrelationOpts::default()).expect("ok");
        // Valid PSD correlation matrices are a fixed point of the projection.
        assert!(max_abs_diff(&input, &repaired) < 1e-8);
        validate_correlation_matrix(&repaired, 3).expect("repaired matrix is valid");
    }

    #[test]
    fn non_psd_matrix_is_projected_to_valid_correlation() {
        // Higham (2002) canonical counter-example: symmetric, unit diagonal,
        // but not PSD (smallest eigenvalue is negative).
        let input = vec![
            1.0, -0.55, -0.55, //
            -0.55, 1.0, -0.55, //
            -0.55, -0.55, 1.0,
        ];
        assert!(validate_correlation_matrix(&input, 3).is_err());

        let repaired =
            nearest_correlation_matrix(&input, 3, NearestCorrelationOpts::default()).expect("ok");
        validate_correlation_matrix(&repaired, 3).expect("repaired matrix is valid");

        for i in 0..3 {
            assert!((repaired[i * 3 + i] - 1.0).abs() < 1e-10);
            for j in (i + 1)..3 {
                let diff = (repaired[i * 3 + j] - repaired[j * 3 + i]).abs();
                assert!(diff < 1e-10);
            }
        }
    }

    #[test]
    fn rejects_wrong_size() {
        let input = vec![1.0, 0.5, 0.5, 1.0];
        let err = nearest_correlation_matrix(&input, 3, NearestCorrelationOpts::default())
            .expect_err("size mismatch");
        assert!(matches!(err, Error::InvalidSize { .. }));
    }

    #[test]
    fn rejects_diagonal_far_from_one() {
        let input = vec![0.5, 0.1, 0.1, 1.0];
        let err = nearest_correlation_matrix(&input, 2, NearestCorrelationOpts::default())
            .expect_err("diagonal guard");
        assert!(matches!(err, Error::DiagonalNotOne { .. }));
    }

    #[test]
    fn rejects_gross_asymmetry() {
        let input = vec![1.0, 0.5, 0.3, 1.0];
        let err = nearest_correlation_matrix(&input, 2, NearestCorrelationOpts::default())
            .expect_err("symmetry guard");
        assert!(matches!(err, Error::NotSymmetric { .. }));
    }

    /// Audit P3 #34: smoke test for the `n > 40` regime where the old
    /// Jacobi eigensolver's `100·n²` sweep cap degenerated to roughly
    /// `O(n⁵)`. With SymmetricEigen this runs in fractions of a second
    /// and must still produce a valid correlation matrix.
    #[test]
    fn nearest_corr_scales_past_forty_dimensions() {
        let n = 60;
        // Construct a "near-correlation" matrix: identity plus a low-rank
        // perturbation that makes some eigenvalues mildly negative.
        let mut input = vec![0.0; n * n];
        for i in 0..n {
            input[i * n + i] = 1.0;
            for j in (i + 1)..n {
                let rho = 0.2 + 0.6 * ((i as f64 + 1.0) / (j as f64 + 1.0)); // off-diag > 1 sometimes
                let rho = rho.clamp(-0.9, 0.9);
                input[i * n + j] = rho;
                input[j * n + i] = rho;
            }
        }

        let opts = NearestCorrelationOpts {
            max_iter: 400,
            tol: 1e-9,
        };
        let repaired = nearest_correlation_matrix(&input, n, opts).expect("converges");

        // Unit diagonal, symmetry, and PSD — the three invariants the
        // projection must restore.
        validate_correlation_matrix(&repaired, n).expect("valid correlation matrix");
        for i in 0..n {
            assert!(
                (repaired[i * n + i] - 1.0).abs() < 1e-9,
                "diag[{i}] = {}",
                repaired[i * n + i]
            );
            for j in (i + 1)..n {
                let d = (repaired[i * n + j] - repaired[j * n + i]).abs();
                assert!(d < 1e-10, "asym at ({i},{j}): {d}");
            }
        }
    }
}
