//! Self-contained Nelder-Mead simplex optimizer for GARCH MLE.
//!
//! Minimizes the negative log-likelihood subject to parameter bounds.
//! The GARCH parameter space is 3-5 dimensional, so a simple Nelder-Mead
//! implementation is sufficient and avoids external optimizer dependencies.
//!
//! Bounds are enforced by projecting trial points back to the feasible region.
//!
//! # References
//!
//! - Nelder, J. A., & Mead, R. (1965). "A Simplex Method for Function
//!   Minimization." The Computer Journal, 7(4), 308-313.

/// Parameter bound: (lower, upper) for each dimension.
pub type Bounds = Vec<(f64, f64)>;

/// Optimization result.
#[derive(Clone, Debug)]
pub struct OptimResult {
    /// Optimal parameter vector.
    pub x: Vec<f64>,
    /// Objective function value at optimum.
    pub f_val: f64,
    /// Whether the optimizer converged.
    pub converged: bool,
    /// Number of iterations.
    pub iterations: usize,
}

/// Nelder-Mead simplex optimizer with box constraints.
pub struct NelderMead {
    /// Maximum iterations.
    pub max_iter: usize,
    /// Function value convergence tolerance.
    pub tol: f64,
}

impl NelderMead {
    /// Create a new optimizer with the given settings.
    #[must_use]
    pub fn new(max_iter: usize, tol: f64) -> Self {
        Self { max_iter, tol }
    }

    /// Minimize `f(x)` subject to `bounds` starting from `x0`.
    ///
    /// Uses the standard Nelder-Mead reflection/expansion/contraction/shrink
    /// algorithm with box-constraint projection.
    pub fn minimize<F>(&self, f: F, x0: &[f64], bounds: &Bounds) -> OptimResult
    where
        F: Fn(&[f64]) -> f64,
    {
        let n = x0.len();
        assert_eq!(bounds.len(), n, "bounds length must match x0 length");

        // Standard Nelder-Mead coefficients
        let alpha = 1.0; // reflection
        let gamma = 2.0; // expansion
        let rho = 0.5; // contraction
        let sigma = 0.5; // shrink

        // Build initial simplex: n+1 vertices
        let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(n + 1);
        simplex.push(project(x0, bounds));

        for i in 0..n {
            let mut v = x0.to_vec();
            let step = if v[i].abs() > 1e-10 {
                0.05 * v[i].abs()
            } else {
                0.00025
            };
            v[i] += step;
            simplex.push(project(&v, bounds));
        }

        // Evaluate function at each vertex
        let mut f_vals: Vec<f64> = simplex.iter().map(|v| f(v)).collect();

        let mut iterations = 0;
        let mut converged = false;

        for _ in 0..self.max_iter {
            iterations += 1;

            // Sort vertices by function value
            let mut order: Vec<usize> = (0..=n).collect();
            order.sort_by(|&a, &b| {
                f_vals[a]
                    .partial_cmp(&f_vals[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Check convergence: range of function values
            let f_best = f_vals[order[0]];
            let f_worst = f_vals[order[n]];
            if (f_worst - f_best).abs() < self.tol * (1.0 + f_best.abs()) {
                converged = true;
                break;
            }

            let i_best = order[0];
            let i_worst = order[n];
            let i_second_worst = order[n - 1];

            // Compute centroid of all vertices except the worst
            let mut centroid = vec![0.0; n];
            for &idx in &order[..n] {
                for j in 0..n {
                    centroid[j] += simplex[idx][j];
                }
            }
            for c in &mut centroid {
                *c /= n as f64;
            }

            // Reflection
            let x_r = project(
                &(0..n)
                    .map(|j| centroid[j] + alpha * (centroid[j] - simplex[i_worst][j]))
                    .collect::<Vec<_>>(),
                bounds,
            );
            let f_r = f(&x_r);

            if f_r < f_vals[i_second_worst] && f_r >= f_vals[i_best] {
                // Accept reflection
                simplex[i_worst] = x_r;
                f_vals[i_worst] = f_r;
                continue;
            }

            if f_r < f_vals[i_best] {
                // Expansion
                let x_e = project(
                    &(0..n)
                        .map(|j| centroid[j] + gamma * (x_r[j] - centroid[j]))
                        .collect::<Vec<_>>(),
                    bounds,
                );
                let f_e = f(&x_e);

                if f_e < f_r {
                    simplex[i_worst] = x_e;
                    f_vals[i_worst] = f_e;
                } else {
                    simplex[i_worst] = x_r;
                    f_vals[i_worst] = f_r;
                }
                continue;
            }

            // Contraction
            let x_c = if f_r < f_vals[i_worst] {
                // Outside contraction
                project(
                    &(0..n)
                        .map(|j| centroid[j] + rho * (x_r[j] - centroid[j]))
                        .collect::<Vec<_>>(),
                    bounds,
                )
            } else {
                // Inside contraction
                project(
                    &(0..n)
                        .map(|j| centroid[j] + rho * (simplex[i_worst][j] - centroid[j]))
                        .collect::<Vec<_>>(),
                    bounds,
                )
            };
            let f_c = f(&x_c);

            if f_c < f_vals[i_worst] {
                simplex[i_worst] = x_c;
                f_vals[i_worst] = f_c;
                continue;
            }

            // Shrink: move all vertices toward the best
            for &idx in &order[1..] {
                #[allow(clippy::needless_range_loop)]
                for j in 0..n {
                    simplex[idx][j] =
                        simplex[i_best][j] + sigma * (simplex[idx][j] - simplex[i_best][j]);
                }
                simplex[idx] = project(&simplex[idx], bounds);
                f_vals[idx] = f(&simplex[idx]);
            }
        }

        // Find best vertex
        let best_idx = f_vals
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        OptimResult {
            x: simplex[best_idx].clone(),
            f_val: f_vals[best_idx],
            converged,
            iterations,
        }
    }
}

/// Project a point onto the box-constrained feasible region.
fn project(x: &[f64], bounds: &Bounds) -> Vec<f64> {
    x.iter()
        .zip(bounds.iter())
        .map(|(&xi, &(lo, hi))| xi.clamp(lo, hi))
        .collect()
}

/// Compute the Hessian via central finite differences at `x`.
///
/// Used to estimate standard errors as diag(H^{-1}) at the MLE.
pub fn finite_diff_hessian<F>(f: &F, x: &[f64], step: f64) -> Vec<Vec<f64>>
where
    F: Fn(&[f64]) -> f64,
{
    let n = x.len();
    let mut hess = vec![vec![0.0; n]; n];
    let f0 = f(x);

    for i in 0..n {
        for j in i..n {
            let mut x_pp = x.to_vec();
            let mut x_pm = x.to_vec();
            let mut x_mp = x.to_vec();
            let mut x_mm = x.to_vec();

            let hi = step * (1.0 + x[i].abs());
            let hj = step * (1.0 + x[j].abs());

            x_pp[i] += hi;
            x_pp[j] += hj;
            x_pm[i] += hi;
            x_pm[j] -= hj;
            x_mp[i] -= hi;
            x_mp[j] += hj;
            x_mm[i] -= hi;
            x_mm[j] -= hj;

            let h_ij = if i == j {
                let mut x_p = x.to_vec();
                let mut x_m = x.to_vec();
                x_p[i] += hi;
                x_m[i] -= hi;
                (f(&x_p) - 2.0 * f0 + f(&x_m)) / (hi * hi)
            } else {
                (f(&x_pp) - f(&x_pm) - f(&x_mp) + f(&x_mm)) / (4.0 * hi * hj)
            };

            hess[i][j] = h_ij;
            hess[j][i] = h_ij;
        }
    }

    hess
}

/// Invert a small symmetric Hessian and return the diagonal of the inverse —
/// the per-parameter asymptotic variance estimates under the Cramér-Rao bound,
/// `Var(θ_i) = (H^{-1})_{ii}`.
///
/// First attempts Cholesky (appropriate for a well-conditioned symmetric PD
/// Hessian), falling back to Tikhonov-regularized inverse `(H + λI)^{-1}`
/// with increasing λ when the Hessian is ill-conditioned. This is the common
/// failure mode at the GARCH stationarity frontier (`α + β → 1`) where the
/// direct inverse is numerically unstable and `try_inverse()` can silently
/// return garbage or `None`.
///
/// Returns `None` only if regularization fails at the largest λ tested,
/// indicating a degenerate fit that should not be trusted.
///
/// The CR-bound formula is `diag(H^{-1})`, not `1/diag(H)`. Routes
/// through `invert_hessian_full` so the diagonal and (optionally) the
/// full covariance matrix come from the same regularized inverse, giving
/// a stable answer when H is near-singular.
#[must_use]
pub fn invert_hessian_diag(hess: &[Vec<f64>]) -> Option<Vec<f64>> {
    let inv = invert_hessian_full(hess)?;
    let n = inv.nrows();
    Some((0..n).map(|i| inv[(i, i)]).collect())
}

/// Full regularized inverse of the Hessian, suitable for recovering the
/// complete parameter covariance matrix.
///
/// Returns the nalgebra `DMatrix` form of `H^{-1}`, using a Cholesky path
/// for the PD case and a Tikhonov `(H + λI)^{-1}` fallback for ill-
/// conditioned Hessians. `λ` is scaled by the Frobenius norm of `H` so the
/// regularization is invariant under the choice of scale (e.g. returns in
/// decimals vs. percent).
///
/// Callers who want cross-parameter covariances (e.g. for delta-method
/// confidence intervals on functions of parameters like the GARCH
/// persistence `α + β`) should use this instead of
/// [`invert_hessian_diag`].
#[must_use]
pub fn invert_hessian_full(hess: &[Vec<f64>]) -> Option<nalgebra::DMatrix<f64>> {
    let n = hess.len();
    if n == 0 {
        return Some(nalgebra::DMatrix::<f64>::zeros(0, 0));
    }
    let base = nalgebra::DMatrix::from_fn(n, n, |i, j| hess[i][j]);

    // First: attempt Cholesky on the raw Hessian (symmetric PD fast path).
    if let Some(chol) = base.clone().cholesky() {
        return Some(chol.inverse());
    }

    // Fallback: Tikhonov-regularized inverse. λ is scaled by the Frobenius
    // norm so the regularization magnitude tracks the matrix scale.
    let frob = base.norm();
    let base_scale = if frob.is_finite() && frob > 0.0 {
        frob
    } else {
        1.0
    };
    let id = nalgebra::DMatrix::<f64>::identity(n, n);
    for k in 0..8 {
        let lambda = base_scale * 1e-8 * 10f64.powi(k);
        let reg = &base + &id * lambda;
        if let Some(chol) = reg.clone().cholesky() {
            return Some(chol.inverse());
        }
        if let Some(inv) = reg.try_inverse() {
            return Some(inv);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Multi-start optimization
// ---------------------------------------------------------------------------

/// First ten primes — used as Halton bases for deterministic low-discrepancy
/// perturbation of the multi-start initial points. Ten bases cover any
/// realistic GARCH/EGARCH/GJR parameter vector (≤ 5 dimensions in practice).
const HALTON_BASES: [usize; 10] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29];

/// Halton sequence element for index `n` in prime `base`.
///
/// Deterministic by construction — two calls with the same `(n, base)` always
/// return bit-identical `[0, 1)` draws. Used by [`halton_perturb`] to spread
/// multi-start initial points across the feasible region without an RNG.
#[inline]
fn halton(mut n: usize, base: usize) -> f64 {
    debug_assert!(base >= 2, "Halton base must be >= 2");
    let mut result = 0.0_f64;
    let mut f = 1.0_f64 / base as f64;
    while n > 0 {
        result += f * (n % base) as f64;
        n /= base;
        f /= base as f64;
    }
    result
}

/// Deterministic multiplicative-plus-additive perturbation of `x0` for
/// multi-start restart index `restart_idx` (1-based).
///
/// Perturbation is `x'_i = (x0_i + (2·h − 1) · scale · (|x0_i| + 1e-6))`
/// clamped to `bounds[i]`. Each parameter dimension uses its own Halton
/// base so restarts do not cluster along coordinate axes.
fn halton_perturb(x0: &[f64], bounds: &Bounds, restart_idx: usize, scale: f64) -> Vec<f64> {
    x0.iter()
        .enumerate()
        .map(|(i, &xi)| {
            let base = HALTON_BASES[i % HALTON_BASES.len()];
            let h = halton(restart_idx, base);
            let delta = (2.0 * h - 1.0) * scale * (xi.abs() + 1e-6);
            let candidate = xi + delta;
            let (lo, hi) = bounds[i];
            candidate.clamp(lo, hi)
        })
        .collect()
}

impl NelderMead {
    /// Multi-start minimization.
    ///
    /// Runs the inner Nelder-Mead minimizer from the original `x0` plus
    /// `num_restarts` deterministically Halton-perturbed initial points and
    /// returns the result with the lowest `f_val`. Non-finite objective
    /// values at a restart cause that restart to be skipped silently — so
    /// an initial point that lands in an infeasible region (e.g. fails the
    /// GARCH stationarity check in the closure) does not poison the result.
    ///
    /// Single-start Nelder-Mead on the GARCH likelihood is prone to
    /// plateau/ridge behavior near the stationarity frontier
    /// (`α + β → 1`); deterministic multi-start dramatically improves
    /// MLE quality on illiquid-asset return series and gives a more
    /// defensible standard-error estimate via the Hessian at the
    /// improved optimum.
    pub fn minimize_multi_start<F>(
        &self,
        f: F,
        x0: &[f64],
        bounds: &Bounds,
        num_restarts: usize,
        perturbation_scale: f64,
    ) -> OptimResult
    where
        F: Fn(&[f64]) -> f64,
    {
        let mut best = self.minimize(&f, x0, bounds);
        for k in 1..=num_restarts {
            let x_k = halton_perturb(x0, bounds, k, perturbation_scale);
            let result_k = self.minimize(&f, &x_k, bounds);
            if result_k.f_val.is_finite() && result_k.f_val < best.f_val {
                best = result_k;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimize_rosenbrock_2d() {
        // Rosenbrock function: minimum at (1, 1)
        let f = |x: &[f64]| {
            let a = 1.0 - x[0];
            let b = x[1] - x[0] * x[0];
            a * a + 100.0 * b * b
        };

        let opt = NelderMead::new(10_000, 1e-12);
        let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];
        let result = opt.minimize(f, &[0.0, 0.0], &bounds);

        assert!(result.converged);
        assert!((result.x[0] - 1.0).abs() < 1e-4);
        assert!((result.x[1] - 1.0).abs() < 1e-4);
        assert!(result.f_val < 1e-8);
    }

    #[test]
    fn minimize_quadratic() {
        // Simple quadratic: minimum at (3, -2)
        let f = |x: &[f64]| (x[0] - 3.0).powi(2) + (x[1] + 2.0).powi(2);

        let opt = NelderMead::new(1000, 1e-12);
        let bounds = vec![(-10.0, 10.0), (-10.0, 10.0)];
        let result = opt.minimize(f, &[0.0, 0.0], &bounds);

        assert!(result.converged);
        assert!((result.x[0] - 3.0).abs() < 1e-6);
        assert!((result.x[1] + 2.0).abs() < 1e-6);
    }

    #[test]
    fn minimize_non_finite_objective_reports_non_convergence() {
        let f = |_x: &[f64]| f64::NAN;
        let opt = NelderMead::new(8, 1e-12);
        let bounds = vec![(-1.0, 1.0), (-1.0, 1.0)];

        let result = opt.minimize(f, &[0.0, 0.0], &bounds);

        assert!(!result.converged);
        assert_eq!(result.iterations, 8);
        assert!(result.f_val.is_nan());
    }

    #[test]
    fn bounds_enforced() {
        let f = |x: &[f64]| (x[0] - 5.0).powi(2);
        let bounds = vec![(0.0, 3.0)];
        let opt = NelderMead::new(1000, 1e-12);
        let result = opt.minimize(f, &[1.0], &bounds);

        // Optimum is at 5.0 but bound is 3.0
        assert!(result.x[0] <= 3.0 + 1e-10);
        assert!((result.x[0] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn finite_diff_hessian_quadratic() {
        // f(x) = x0^2 + 2*x1^2, Hessian = [[2, 0], [0, 4]]
        let f = |x: &[f64]| x[0] * x[0] + 2.0 * x[1] * x[1];
        let h = finite_diff_hessian(&f, &[0.0, 0.0], 1e-5);

        assert!((h[0][0] - 2.0).abs() < 1e-4);
        assert!((h[1][1] - 4.0).abs() < 1e-4);
        assert!(h[0][1].abs() < 1e-6);
    }

    #[test]
    fn invert_diagonal_hessian() {
        let hess = vec![vec![2.0, 0.0], vec![0.0, 4.0]];
        let diag = invert_hessian_diag(&hess).unwrap();
        assert!((diag[0] - 0.5).abs() < 1e-10);
        assert!((diag[1] - 0.25).abs() < 1e-10);
    }

    #[test]
    fn invert_hessian_cross_off_diagonal_contributes_to_std_errors() {
        // The diagonal of the INVERSE is not the reciprocal of the
        // diagonal — off-diagonal Hessian entries must propagate into
        // per-parameter variance. A 2x2 Hessian with a large off-
        // diagonal gives a diagonal inverse entry noticeably larger
        // than `1/H_{ii}`.
        //
        // H = [[4, 3],      (H^{-1})_{11} = 4 / det(H) = 4 / (16 - 9) = 4/7
        //      [3, 4]]      1/H_{11}      = 0.25
        let hess = vec![vec![4.0, 3.0], vec![3.0, 4.0]];
        let diag = invert_hessian_diag(&hess).expect("PD matrix invertible");
        let expected = 4.0 / 7.0;
        assert!(
            (diag[0] - expected).abs() < 1e-10,
            "(H^-1)_{{11}} should be {expected} (includes off-diag coupling), got {}",
            diag[0]
        );
        // Naive 1/H_{11} would be 0.25 — far from the correct 4/7 ≈ 0.571.
        assert!(
            diag[0] > 0.5,
            "off-diagonal coupling must raise the diagonal inverse above 1/H_ii"
        );
    }

    #[test]
    fn invert_hessian_tikhonov_fallback_for_singular_matrix() {
        // Rank-1 Hessian: not invertible by Cholesky or LU. Tikhonov fallback
        // should still return a finite regularized inverse.
        let hess = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        let diag =
            invert_hessian_diag(&hess).expect("Tikhonov fallback must handle singular Hessian");
        assert!(
            diag.iter().all(|d| d.is_finite()),
            "regularized inverse must have finite diagonal: {diag:?}"
        );
    }

    #[test]
    fn invert_hessian_full_matches_diag() {
        // Full and diag helpers must agree on the diagonal of the inverse.
        let hess = vec![vec![4.0, 3.0], vec![3.0, 4.0]];
        let diag = invert_hessian_diag(&hess).expect("diag invertible");
        let full = invert_hessian_full(&hess).expect("full invertible");
        for i in 0..2 {
            assert!(
                (diag[i] - full[(i, i)]).abs() < 1e-12,
                "diag({i}) {} vs full({i},{i}) {}",
                diag[i],
                full[(i, i)]
            );
        }
    }

    #[test]
    fn halton_sequence_deterministic_and_bounded() {
        // First five Halton points in base 2 are well-known: 1/2, 1/4, 3/4, 1/8, 5/8.
        let vals: Vec<f64> = (1..=5).map(|n| halton(n, 2)).collect();
        let expected = [0.5, 0.25, 0.75, 0.125, 0.625];
        for (got, want) in vals.iter().zip(expected.iter()) {
            assert!(
                (got - want).abs() < 1e-15,
                "Halton(n, 2) mismatch: got {got}, want {want}"
            );
        }
        for v in vals {
            assert!((0.0..1.0).contains(&v), "Halton draws must be in [0, 1)");
        }
    }

    #[test]
    fn multi_start_escapes_local_minimum() {
        // Tilted double-well: f(x) = (x² − 1)² + 0.1·x.
        //   Global min: x ≈ −1, f ≈ −0.1.
        //   Local min:  x ≈ +1, f ≈ +0.1.
        // A single Nelder-Mead run starting at x = 2 gets trapped in the
        // x ≈ +1 basin because its tiny initial simplex step cannot hop the
        // barrier at x = 0. Halton-perturbed multi-start with a large
        // perturbation scale spreads subsequent starts across both basins
        // and discovers the global minimum.
        let f = |x: &[f64]| {
            let p = x[0] * x[0] - 1.0;
            p * p + 0.1 * x[0]
        };
        let opt = NelderMead::new(2_000, 1e-10);
        let bounds = vec![(-5.0, 5.0)];

        let x0 = [2.0];
        let single = opt.minimize(f, &x0, &bounds);
        let multi = opt.minimize_multi_start(f, &x0, &bounds, 6, 1.5);

        assert!(
            multi.f_val <= single.f_val + 1e-9,
            "multi-start f_val {} must not exceed single-start f_val {}",
            multi.f_val,
            single.f_val
        );
        assert!(
            single.f_val > 0.05,
            "single-start from x0=2 should stall at the x≈+1 local min (f≈+0.1), got {}",
            single.f_val
        );
        assert!(
            multi.f_val < -0.05,
            "multi-start should find the x≈-1 global min (f≈-0.1), got {}",
            multi.f_val
        );
    }
}
