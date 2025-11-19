//! Multi-dimensional optimization for model calibration.
//!
//! Provides robust algorithms for calibrating financial models with multiple
//! parameters, such as SABR volatility surfaces, Heston stochastic volatility,
//! and multi-curve bootstrapping problems.
//!
//! # Algorithms
//!
//! - **Levenberg-Marquardt**: Damped least-squares for sum-of-squares objectives
//!
//! # Use Cases
//!
//! - **SABR calibration**: Fit α, β, ρ, ν to market smiles
//! - **Heston calibration**: Fit stochastic volatility parameters to vanilla options
//! - **Multi-curve bootstrapping**: Simultaneous curve fitting with constraints
//! - **Smile interpolation**: Parametric volatility surface construction
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::solver_multi::{LevenbergMarquardtSolver, MultiSolver};
//!
//! let solver = LevenbergMarquardtSolver::new().with_tolerance(1e-8);
//!
//! // Minimize sum of squares: (x-2)^2 + (y-3)^2
//! let objective = |params: &[f64]| -> f64 {
//!     (params[0] - 2.0).powi(2) + (params[1] - 3.0).powi(2)
//! };
//!
//! let initial = vec![0.0, 0.0];
//! let result = solver.minimize(objective, &initial, None).expect("Minimization should succeed");
//! assert!((result[0] - 2.0).abs() < 1e-6);
//! assert!((result[1] - 3.0).abs() < 1e-6);
//! ```
//!
//! # References
//!
//! - **Levenberg-Marquardt**:
//!   - Levenberg, K. (1944). "A Method for the Solution of Certain Non-Linear Problems
//!     in Least Squares." *Quarterly of Applied Mathematics*, 2(2), 164-168.
//!   - Marquardt, D. W. (1963). "An Algorithm for Least-Squares Estimation of Nonlinear
//!     Parameters." *Journal of the Society for Industrial and Applied Mathematics*, 11(2), 431-441.
//!   - Moré, J. J. (1978). "The Levenberg-Marquardt Algorithm: Implementation and Theory."
//!     *Numerical Analysis*, Lecture Notes in Mathematics, vol 630.
//!
//! - **Calibration Applications**:
//!   - Hagan, P. S., et al. (2002). "Managing Smile Risk." *Wilmott Magazine*, September 2002.
//!     (SABR calibration techniques)

use crate::error::InputError;
use crate::Result;
use ndarray::{Array1, Array2};

/// Trait for functions that can provide analytical derivatives.
///
/// This trait allows optimization algorithms to use exact derivatives
/// when available, significantly improving convergence speed and accuracy.
pub trait AnalyticalDerivatives {
    /// Compute the gradient of the objective function.
    ///
    /// # Arguments
    /// * `params` - Current parameter values
    /// * `gradient` - Output buffer for gradient (must be same length as params)
    fn gradient(&self, params: &[f64], gradient: &mut [f64]);

    /// Compute the Jacobian matrix for a system of equations.
    ///
    /// # Arguments
    /// * `params` - Current parameter values
    /// * `jacobian` - Output buffer for Jacobian matrix (rows = equations, cols = params)
    ///
    /// Default implementation returns None, indicating Jacobian is not available.
    fn jacobian(&self, _params: &[f64], _jacobian: &mut [Vec<f64>]) -> Option<()> {
        None
    }

    /// Returns true if analytical gradient is available.
    fn has_gradient(&self) -> bool {
        true
    }

    /// Returns true if analytical Jacobian is available.
    fn has_jacobian(&self) -> bool {
        false
    }
}

/// Multi-dimensional optimization/root-finding trait.
///
/// Provides a unified interface for solvers that can handle multiple
/// parameters simultaneously, essential for calibrating complex models
/// like SABR volatility surfaces.
pub trait MultiSolver: Send + Sync {
    /// Minimize objective function starting from initial guess.
    ///
    /// # Arguments
    /// * `objective` - Function to minimize, takes parameter vector and returns scalar
    /// * `initial` - Initial parameter guess
    /// * `bounds` - Optional box constraints for each parameter
    ///
    /// # Returns
    /// Optimal parameter vector that minimizes the objective
    fn minimize<Obj>(
        &self,
        objective: Obj,
        initial: &[f64],
        bounds: Option<&[(f64, f64)]>,
    ) -> Result<Vec<f64>>
    where
        Obj: Fn(&[f64]) -> f64;

    /// Solve system of equations f(x) = 0 using least squares.
    ///
    /// # Arguments
    /// * `residuals` - Function that computes residual vector
    /// * `initial` - Initial parameter guess
    ///
    /// # Returns
    /// Parameter vector that minimizes ||f(x)||²
    fn solve_system<Res>(&self, residuals: Res, initial: &[f64]) -> Result<Vec<f64>>
    where
        Res: Fn(&[f64], &mut [f64]),
    {
        // Default implementation: convert to minimization problem
        let objective = |params: &[f64]| -> f64 {
            let mut resid = vec![0.0; initial.len()];
            residuals(params, &mut resid);
            resid.iter().map(|r| r * r).sum()
        };
        self.minimize(objective, initial, None)
    }
}

/// Levenberg-Marquardt solver for non-linear least squares.
///
/// Combines Gauss-Newton and gradient descent methods using a damping parameter
/// λ that adapts based on progress. Particularly effective for sum-of-squares
/// objectives arising in curve calibration and parameter fitting.
///
/// # Algorithm
///
/// At each iteration, solves:
/// ```text
/// (J^T J + λI) δ = -J^T r
///
/// where:
///   J = Jacobian matrix
///   r = residual vector
///   λ = damping parameter (adaptive)
///   δ = parameter update
/// ```
///
/// - **λ → 0**: Gauss-Newton (fast near solution)
/// - **λ → ∞**: Gradient descent (robust far from solution)
///
/// # Convergence
///
/// Typically achieves quadratic convergence near the solution, switching to
/// linear convergence when far from optimum. More robust than pure Gauss-Newton.
///
/// # Use Cases
///
/// - SABR volatility surface calibration
/// - Swaption volatility cube fitting
/// - Yield curve bootstrapping
/// - Credit curve calibration
///
/// # References
///
/// - Levenberg, K. (1944). "A Method for the Solution of Certain Non-Linear Problems
///   in Least Squares." *Quarterly of Applied Mathematics*, 2(2), 164-168.
/// - Marquardt, D. W. (1963). "An Algorithm for Least-Squares Estimation of Nonlinear
///   Parameters." *SIAM Journal*, 11(2), 431-441.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LevenbergMarquardtSolver {
    /// Convergence tolerance for gradient norm
    pub tolerance: f64,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Initial damping parameter (lambda)
    pub lambda_init: f64,
    /// Factor for adjusting lambda (increase on failure, decrease on success)
    pub lambda_factor: f64,
    /// Finite difference step size for Jacobian approximation
    pub fd_step: f64,
    /// Minimum allowed step size (for numerical stability)
    pub min_step_size: f64,
}

impl Default for LevenbergMarquardtSolver {
    fn default() -> Self {
        Self {
            tolerance: 1e-8,
            max_iterations: 100,
            lambda_init: 1e-3,
            lambda_factor: 10.0,
            fd_step: 1e-8,
            min_step_size: 1e-12,
        }
    }
}

impl LevenbergMarquardtSolver {
    /// Create a new Levenberg-Marquardt solver with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set convergence tolerance.
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set maximum iterations.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set initial damping parameter.
    pub fn with_lambda_init(mut self, lambda: f64) -> Self {
        self.lambda_init = lambda;
        self
    }

    /// Set finite difference step size.
    pub fn with_fd_step(mut self, step: f64) -> Self {
        self.fd_step = step;
        self
    }

    /// Compute Jacobian matrix using finite differences.
    fn compute_jacobian<Obj>(&self, objective: &Obj, params: &[f64]) -> Vec<Vec<f64>>
    where
        Obj: Fn(&[f64]) -> f64,
    {
        let n = params.len();
        let mut jacobian = vec![vec![0.0; n]; 1]; // For scalar objective, Jacobian is 1×n

        let f0 = objective(params);
        let mut params_plus = params.to_vec();

        for j in 0..n {
            let h = (params[j].abs() * self.fd_step).max(self.fd_step);
            params_plus[j] = params[j] + h;
            let f_plus = objective(&params_plus);
            jacobian[0][j] = (f_plus - f0) / h;
            params_plus[j] = params[j]; // Reset
        }

        jacobian
    }

    /// Compute gradient using analytical derivatives if available, otherwise finite differences.
    fn compute_gradient_with_analytical<Obj, D>(
        &self,
        objective: &Obj,
        params: &[f64],
        derivatives: Option<&D>,
    ) -> Vec<f64>
    where
        Obj: Fn(&[f64]) -> f64,
        D: AnalyticalDerivatives,
    {
        if let Some(deriv) = derivatives {
            if deriv.has_gradient() {
                let mut gradient = vec![0.0; params.len()];
                deriv.gradient(params, &mut gradient);
                return gradient;
            }
        }

        // Fall back to finite differences
        let jacobian = self.compute_jacobian(objective, params);
        jacobian[0].clone()
    }

    /// Compute Jacobian for a system of residuals.
    fn compute_jacobian_system<Res>(
        &self,
        residuals: &Res,
        params: &[f64],
        n_residuals: usize,
    ) -> Vec<Vec<f64>>
    where
        Res: Fn(&[f64], &mut [f64]),
    {
        let n_params = params.len();
        let mut jacobian = vec![vec![0.0; n_params]; n_residuals];

        let mut r0 = vec![0.0; n_residuals];
        residuals(params, &mut r0);

        let mut params_plus = params.to_vec();
        let mut r_plus = vec![0.0; n_residuals];

        for j in 0..n_params {
            let h = (params[j].abs() * self.fd_step).max(self.fd_step);
            params_plus[j] = params[j] + h;
            residuals(&params_plus, &mut r_plus);

            for i in 0..n_residuals {
                jacobian[i][j] = (r_plus[i] - r0[i]) / h;
            }
            params_plus[j] = params[j]; // Reset
        }

        jacobian
    }

    /// Solve the normal equations (J^T J + λI) δ = -J^T r
    fn solve_normal_equations(
        &self,
        jacobian: &[Vec<f64>],
        residuals: &[f64],
        lambda: f64,
    ) -> Result<Vec<f64>> {
        let n = jacobian[0].len(); // Number of parameters
        let m = jacobian.len(); // Number of residuals

        // Compute J^T J
        let mut jtj = vec![vec![0.0; n]; n];
        for (i, row) in jtj.iter_mut().enumerate().take(n) {
            for (j, entry) in row.iter_mut().enumerate().take(n) {
                for jacobian_row in jacobian.iter().take(m) {
                    *entry += jacobian_row[i] * jacobian_row[j];
                }
            }
        }

        // Add damping term λI
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            jtj[i][i] += lambda;
        }

        // Compute -J^T r
        let mut jtr = vec![0.0; n];
        for (i, jtr_entry) in jtr.iter_mut().enumerate().take(n) {
            for jacobian_row in jacobian.iter().take(m).enumerate() {
                let (k, row) = jacobian_row;
                *jtr_entry -= row[i] * residuals[k];
            }
        }

        // Solve using Gaussian elimination with partial pivoting
        self.solve_linear_system(&jtj, &jtr)
    }

    /// Solve linear system using ndarray with improved numerical stability.
    fn solve_linear_system(&self, a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>> {
        let n = a.len();
        if n == 0 || b.len() != n {
            return Err(InputError::Invalid.into());
        }

        // Convert to ndarray format for better memory layout and operations
        let mut matrix = Array2::<f64>::zeros((n, n));
        for (i, row) in a.iter().enumerate() {
            if row.len() != n {
                return Err(InputError::Invalid.into());
            }
            for (j, &val) in row.iter().enumerate() {
                matrix[[i, j]] = val;
            }
        }
        let mut x = Array1::from_vec(b.to_vec());

        // Simple but numerically stable Gaussian elimination with partial pivoting
        // Using ndarray for better memory access patterns
        for k in 0..n {
            // Find pivot
            let mut max_idx = k;
            let mut max_val = matrix[[k, k]].abs();
            for i in (k + 1)..n {
                let val = matrix[[i, k]].abs();
                if val > max_val {
                    max_val = val;
                    max_idx = i;
                }
            }

            if max_val < 1e-15 {
                return Err(InputError::Invalid.into());
            }

            // Swap rows if needed
            if max_idx != k {
                for j in 0..n {
                    let tmp = matrix[[k, j]];
                    matrix[[k, j]] = matrix[[max_idx, j]];
                    matrix[[max_idx, j]] = tmp;
                }
                let tmp = x[k];
                x[k] = x[max_idx];
                x[max_idx] = tmp;
            }

            // Eliminate
            for i in (k + 1)..n {
                let factor = matrix[[i, k]] / matrix[[k, k]];
                for j in (k + 1)..n {
                    matrix[[i, j]] -= factor * matrix[[k, j]];
                }
                x[i] -= factor * x[k];
            }
        }

        // Back substitution
        for i in (0..n).rev() {
            for j in (i + 1)..n {
                x[i] -= matrix[[i, j]] * x[j];
            }
            x[i] /= matrix[[i, i]];
        }

        Ok(x.to_vec())
    }

    /// Apply box constraints to parameters.
    fn apply_bounds(&self, params: &mut [f64], bounds: Option<&[(f64, f64)]>) {
        if let Some(bounds) = bounds {
            for (i, (lo, hi)) in bounds.iter().enumerate().take(params.len()) {
                params[i] = params[i].clamp(*lo, *hi);
            }
        }
    }

    /// Minimize objective function with analytical derivatives.
    ///
    /// # Arguments
    /// * `objective` - Function to minimize
    /// * `derivatives` - Provider of analytical derivatives
    /// * `initial` - Initial parameter guess
    /// * `bounds` - Optional box constraints
    ///
    /// # Returns
    /// Optimal parameter vector
    pub fn minimize_with_derivatives<Obj, D>(
        &self,
        objective: Obj,
        derivatives: &D,
        initial: &[f64],
        bounds: Option<&[(f64, f64)]>,
    ) -> Result<Vec<f64>>
    where
        Obj: Fn(&[f64]) -> f64,
        D: AnalyticalDerivatives,
    {
        if initial.is_empty() {
            return Err(InputError::Invalid.into());
        }

        let mut params = initial.to_vec();
        let mut lambda = self.lambda_init;
        let mut best_value = objective(&params);
        let mut best_params = params.clone();

        for _iter in 0..self.max_iterations {
            // Use analytical gradient if available
            let gradient =
                self.compute_gradient_with_analytical(&objective, &params, Some(derivatives));

            // Check convergence
            let grad_norm: f64 = gradient.iter().map(|g| g * g).sum::<f64>().sqrt();
            if grad_norm < self.tolerance {
                return Ok(params);
            }

            // For scalar objective, create Jacobian from gradient
            let jacobian = vec![gradient.clone()];
            let residual = vec![objective(&params)];

            // Solve for step
            let step = self.solve_normal_equations(&jacobian, &residual, lambda)?;

            // Check step size
            let step_norm: f64 = step.iter().map(|s| s * s).sum::<f64>().sqrt();
            if step_norm < self.min_step_size {
                return Ok(params);
            }

            // Try the step
            let mut new_params = params.clone();
            for (i, &s) in step.iter().enumerate() {
                new_params[i] += s;
            }
            self.apply_bounds(&mut new_params, bounds);

            let new_value = objective(&new_params);

            // Accept or reject step
            if new_value < best_value {
                params = new_params;
                best_value = new_value;
                best_params = params.clone();
                lambda /= self.lambda_factor;
                lambda = lambda.max(1e-15);
            } else {
                lambda *= self.lambda_factor;
                lambda = lambda.min(1e15);
            }
        }

        Ok(best_params)
    }

    /// Solve system with analytical Jacobian.
    pub fn solve_system_with_jacobian<Res, D>(
        &self,
        residuals: Res,
        derivatives: &D,
        initial: &[f64],
    ) -> Result<Vec<f64>>
    where
        Res: Fn(&[f64], &mut [f64]),
        D: AnalyticalDerivatives,
    {
        if initial.is_empty() {
            return Err(InputError::Invalid.into());
        }

        // Determine number of residuals
        let mut test_residuals = vec![0.0; initial.len() * 2];
        residuals(initial, &mut test_residuals);
        let n_residuals = test_residuals
            .iter()
            .position(|&r| r == 0.0)
            .unwrap_or(test_residuals.len());
        let mut resid_vec = vec![0.0; n_residuals];

        let mut params = initial.to_vec();
        let mut lambda = self.lambda_init;

        for _iter in 0..self.max_iterations {
            // Compute residuals
            residuals(&params, &mut resid_vec);

            // Use analytical Jacobian if available
            let jacobian = if derivatives.has_jacobian() {
                let mut jac = vec![vec![0.0; params.len()]; n_residuals];
                if derivatives.jacobian(&params, &mut jac).is_some() {
                    jac
                } else {
                    // Fall back to finite differences
                    self.compute_jacobian_system(&residuals, &params, n_residuals)
                }
            } else {
                self.compute_jacobian_system(&residuals, &params, n_residuals)
            };

            // Check convergence
            let resid_norm: f64 = resid_vec.iter().map(|r| r * r).sum::<f64>().sqrt();
            if resid_norm < self.tolerance {
                return Ok(params);
            }

            // Solve for step
            let step = self.solve_normal_equations(&jacobian, &resid_vec, lambda)?;

            // Check step size
            let step_norm: f64 = step.iter().map(|s| s * s).sum::<f64>().sqrt();
            if step_norm < self.min_step_size {
                return Ok(params);
            }

            // Try the step
            let mut new_params = params.clone();
            for (i, &s) in step.iter().enumerate() {
                new_params[i] += s;
            }

            // Evaluate new residuals
            let mut new_resid = vec![0.0; n_residuals];
            residuals(&new_params, &mut new_resid);
            let new_norm: f64 = new_resid.iter().map(|r| r * r).sum::<f64>().sqrt();

            // Accept or reject
            if new_norm < resid_norm {
                params = new_params;
                lambda /= self.lambda_factor;
                lambda = lambda.max(1e-15);
            } else {
                lambda *= self.lambda_factor;
                lambda = lambda.min(1e15);
            }
        }

        Ok(params)
    }
}

impl MultiSolver for LevenbergMarquardtSolver {
    fn minimize<Obj>(
        &self,
        objective: Obj,
        initial: &[f64],
        bounds: Option<&[(f64, f64)]>,
    ) -> Result<Vec<f64>>
    where
        Obj: Fn(&[f64]) -> f64,
    {
        if initial.is_empty() {
            return Err(InputError::Invalid.into());
        }

        let mut params = initial.to_vec();
        let mut lambda = self.lambda_init;
        let mut best_value = objective(&params);
        let mut best_params = params.clone();

        for _iter in 0..self.max_iterations {
            // Compute gradient (Jacobian for scalar function)
            let jacobian = self.compute_jacobian(&objective, &params);
            let gradient: Vec<f64> = jacobian[0].clone();

            // Check convergence
            let grad_norm: f64 = gradient.iter().map(|g| g * g).sum::<f64>().sqrt();
            if grad_norm < self.tolerance {
                return Ok(params);
            }

            // For scalar objective, residual is just the objective value
            let residual = vec![objective(&params)];

            // Solve for step
            let step = self.solve_normal_equations(&jacobian, &residual, lambda)?;

            // Check step size
            let step_norm: f64 = step.iter().map(|s| s * s).sum::<f64>().sqrt();
            if step_norm < self.min_step_size {
                return Ok(params);
            }

            // Try the step
            let mut new_params = params.clone();
            for (i, &s) in step.iter().enumerate() {
                new_params[i] += s;
            }
            self.apply_bounds(&mut new_params, bounds);

            let new_value = objective(&new_params);

            // Accept or reject step
            if new_value < best_value {
                // Accept: decrease lambda (more Newton-like)
                params = new_params;
                best_value = new_value;
                best_params = params.clone();
                lambda /= self.lambda_factor;
                lambda = lambda.max(1e-15);
            } else {
                // Reject: increase lambda (more gradient descent-like)
                lambda *= self.lambda_factor;
                lambda = lambda.min(1e15);
            }
        }

        Ok(best_params)
    }

    fn solve_system<Res>(&self, residuals: Res, initial: &[f64]) -> Result<Vec<f64>>
    where
        Res: Fn(&[f64], &mut [f64]),
    {
        if initial.is_empty() {
            return Err(InputError::Invalid.into());
        }

        // Determine number of residuals
        let mut test_residuals = vec![0.0; initial.len() * 2]; // Assume at most 2x params
        residuals(initial, &mut test_residuals);
        let n_residuals = test_residuals
            .iter()
            .position(|&r| r == 0.0)
            .unwrap_or(test_residuals.len());
        let mut resid_vec = vec![0.0; n_residuals];

        let mut params = initial.to_vec();
        let mut lambda = self.lambda_init;

        for _iter in 0..self.max_iterations {
            // Compute residuals and Jacobian
            residuals(&params, &mut resid_vec);
            let jacobian = self.compute_jacobian_system(&residuals, &params, n_residuals);

            // Check convergence
            let resid_norm: f64 = resid_vec.iter().map(|r| r * r).sum::<f64>().sqrt();
            if resid_norm < self.tolerance {
                return Ok(params);
            }

            // Solve for step
            let step = self.solve_normal_equations(&jacobian, &resid_vec, lambda)?;

            // Check step size
            let step_norm: f64 = step.iter().map(|s| s * s).sum::<f64>().sqrt();
            if step_norm < self.min_step_size {
                return Ok(params);
            }

            // Try the step
            let mut new_params = params.clone();
            for (i, &s) in step.iter().enumerate() {
                new_params[i] += s;
            }

            // Evaluate new residuals
            let mut new_resid = vec![0.0; n_residuals];
            residuals(&new_params, &mut new_resid);
            let new_norm: f64 = new_resid.iter().map(|r| r * r).sum::<f64>().sqrt();

            // Accept or reject
            if new_norm < resid_norm {
                params = new_params;
                lambda /= self.lambda_factor;
                lambda = lambda.max(1e-15);
            } else {
                lambda *= self.lambda_factor;
                lambda = lambda.min(1e15);
            }
        }

        Ok(params)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenberg_marquardt_simple_minimum() {
        let solver = LevenbergMarquardtSolver::new().with_tolerance(1e-10);

        // Minimize (x-2)^2 + (y-3)^2
        let objective =
            |params: &[f64]| -> f64 { (params[0] - 2.0).powi(2) + (params[1] - 3.0).powi(2) };

        let initial = vec![0.0, 0.0];
        let result = solver
            .minimize(objective, &initial, None)
            .expect("Minimization should succeed in test");

        assert!((result[0] - 2.0).abs() < 1e-6);
        assert!((result[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_levenberg_marquardt_with_bounds() {
        let solver = LevenbergMarquardtSolver::new();

        // Minimize (x-5)^2 + (y-5)^2 with bounds
        let objective =
            |params: &[f64]| -> f64 { (params[0] - 5.0).powi(2) + (params[1] - 5.0).powi(2) };

        let initial = vec![0.0, 0.0];
        let bounds = vec![(0.0, 3.0), (0.0, 3.0)]; // Constrain solution
        let result = solver
            .minimize(objective, &initial, Some(&bounds))
            .expect("Minimization should succeed in test");

        // Solution should be at boundary (3, 3)
        assert!((result[0] - 3.0).abs() < 1e-6);
        assert!((result[1] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_levenberg_marquardt_system() {
        let solver = LevenbergMarquardtSolver::new().with_tolerance(1e-8);

        // Solve system: x + y = 5, x - y = 1 (solution: x=3, y=2)
        let residuals = |params: &[f64], resid: &mut [f64]| {
            resid[0] = params[0] + params[1] - 5.0;
            resid[1] = params[0] - params[1] - 1.0;
        };

        let initial = vec![0.0, 0.0];
        let result = solver
            .solve_system(residuals, &initial)
            .expect("System solving should succeed in test");

        assert!((result[0] - 3.0).abs() < 1e-6);
        assert!((result[1] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_least_squares_fitting() {
        let solver = LevenbergMarquardtSolver::new()
            .with_tolerance(1e-8)
            .with_max_iterations(100);

        // Fit a quadratic y = a*x^2 + b*x + c to some data points
        // True parameters: a=1, b=-2, c=3
        let x_data = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let y_data = [11.0, 6.0, 3.0, 2.0, 3.0]; // y = x^2 - 2*x + 3

        // Least squares objective
        let objective = move |params: &[f64]| -> f64 {
            let a = params[0];
            let b = params[1];
            let c = params[2];

            x_data
                .iter()
                .zip(y_data.iter())
                .map(|(&x, &y_true)| {
                    let y_pred = a * x * x + b * x + c;
                    (y_pred - y_true).powi(2)
                })
                .sum()
        };

        let initial = vec![0.5, 0.5, 0.5];
        let result = solver
            .minimize(objective, &initial, None)
            .expect("Minimization should succeed in test");

        // Should recover the true parameters (within reasonable tolerance)
        assert!(
            (result[0] - 1.0).abs() < 0.01,
            "a = {}, expected 1.0",
            result[0]
        );
        assert!(
            (result[1] - (-2.0)).abs() < 0.01,
            "b = {}, expected -2.0",
            result[1]
        );
        assert!(
            (result[2] - 3.0).abs() < 0.01,
            "c = {}, expected 3.0",
            result[2]
        );
    }


    #[test]
    fn test_analytical_derivatives_simple() {
        // Simple quadratic with analytical derivatives
        struct QuadraticDerivatives;

        impl AnalyticalDerivatives for QuadraticDerivatives {
            fn gradient(&self, params: &[f64], gradient: &mut [f64]) {
                // f(x,y) = (x-2)^2 + (y-3)^2
                gradient[0] = 2.0 * (params[0] - 2.0);
                gradient[1] = 2.0 * (params[1] - 3.0);
            }
        }

        let solver = LevenbergMarquardtSolver::new().with_tolerance(1e-10);

        let objective =
            |params: &[f64]| -> f64 { (params[0] - 2.0).powi(2) + (params[1] - 3.0).powi(2) };

        let derivatives = QuadraticDerivatives;
        let initial = vec![0.0, 0.0];

        let result = solver
            .minimize_with_derivatives(objective, &derivatives, &initial, None)
            .expect("Minimization should succeed in test");

        assert!((result[0] - 2.0).abs() < 1e-8);
        assert!((result[1] - 3.0).abs() < 1e-8);
    }

    #[test]
    fn test_jacobian_system() {
        // System: x^2 + y^2 = 4, x - y = 0
        struct CircleLineJacobian;

        impl AnalyticalDerivatives for CircleLineJacobian {
            fn gradient(&self, _params: &[f64], _gradient: &mut [f64]) {
                // Not used for this test
            }

            fn jacobian(&self, params: &[f64], jacobian: &mut [Vec<f64>]) -> Option<()> {
                let x = params[0];
                let y = params[1];

                jacobian[0][0] = 2.0 * x; // df1/dx
                jacobian[0][1] = 2.0 * y; // df1/dy
                jacobian[1][0] = 1.0; // df2/dx
                jacobian[1][1] = -1.0; // df2/dy

                Some(())
            }

            fn has_jacobian(&self) -> bool {
                true
            }

            fn has_gradient(&self) -> bool {
                false
            }
        }

        let solver = LevenbergMarquardtSolver::new().with_tolerance(1e-10);

        let residuals = |params: &[f64], resid: &mut [f64]| {
            resid[0] = params[0] * params[0] + params[1] * params[1] - 4.0;
            resid[1] = params[0] - params[1];
        };

        let derivatives = CircleLineJacobian;
        let initial = vec![1.0, 0.0];

        let result = solver
            .solve_system_with_jacobian(residuals, &derivatives, &initial)
            .expect("Minimization should succeed in test");

        let expected = 2.0_f64.sqrt();
        assert!((result[0] - expected).abs() < 1e-8);
        assert!((result[1] - expected).abs() < 1e-8);
    }
}
