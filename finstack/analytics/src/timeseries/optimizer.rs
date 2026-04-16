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

/// Invert a small matrix and return diagonal elements (for standard errors).
/// Returns None if the matrix is singular.
#[must_use]
pub fn invert_hessian_diag(hess: &[Vec<f64>]) -> Option<Vec<f64>> {
    let n = hess.len();
    if n == 0 {
        return Some(vec![]);
    }

    // Use nalgebra for matrix inversion
    let mat = nalgebra::DMatrix::from_fn(n, n, |i, j| hess[i][j]);
    let inv = mat.try_inverse()?;

    Some((0..n).map(|i| inv[(i, i)]).collect())
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
}
