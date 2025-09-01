//! Generic solver interface and implementations for calibration.
//!
//! Provides a unified interface for 1D root finding and multi-dimensional
//! optimization used throughout the calibration framework.

use finstack_core::math::root_finding::{brent, newton_raphson};
use finstack_core::{Result, F};

/// Generic solver trait for 1D root finding.
pub trait Solver: Send + Sync {
    /// Solve f(x) = 0 starting from initial guess.
    fn solve<Func>(&self, f: Func, initial_guess: F) -> Result<F>
    where
        Func: Fn(F) -> F;
}

/// Newton-Raphson solver with automatic derivative estimation.
#[derive(Clone, Debug)]
pub struct NewtonSolver {
    /// Convergence tolerance
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Finite difference step for derivative estimation
    pub fd_step: F,
}

impl Default for NewtonSolver {
    fn default() -> Self {
        Self {
            tolerance: 1e-12,
            max_iterations: 50,
            fd_step: 1e-8,
        }
    }
}

impl NewtonSolver {
    /// Create a new Newton solver with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set tolerance.
    pub fn with_tolerance(mut self, tolerance: F) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set maximum iterations.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }
}

impl Solver for NewtonSolver {
    fn solve<Func>(&self, f: Func, initial_guess: F) -> Result<F>
    where
        Func: Fn(F) -> F,
    {
        // Use automatic differentiation via finite differences
        let derivative = |x: F| -> F {
            let f_plus = f(x + self.fd_step);
            let f_minus = f(x - self.fd_step);
            (f_plus - f_minus) / (2.0 * self.fd_step)
        };

        newton_raphson(
            &f,
            derivative,
            initial_guess,
            self.tolerance,
            self.max_iterations,
        )
    }
}

/// Brent's method solver (robust, bracketing required).
#[derive(Clone, Debug)]
pub struct BrentSolver {
    /// Convergence tolerance
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Bracket expansion factor
    pub bracket_expansion: F,
}

impl Default for BrentSolver {
    fn default() -> Self {
        Self {
            tolerance: 1e-12,
            max_iterations: 100,
            bracket_expansion: 2.0,
        }
    }
}

impl BrentSolver {
    /// Create a new Brent solver with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set tolerance.
    pub fn with_tolerance(mut self, tolerance: F) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Find bracket around the root starting from initial guess.
    fn find_bracket<Func>(&self, f: &Func, initial_guess: F) -> Result<(F, F)>
    where
        Func: Fn(F) -> F,
    {
        let mut a = initial_guess - 0.01;
        let mut b = initial_guess + 0.01;

        // Expand bracket until we find a sign change
        for _ in 0..20 {
            let fa = f(a);
            let fb = f(b);

            if fa * fb < 0.0 {
                return Ok((a, b));
            }

            // Expand bracket
            let width = b - a;
            a -= width * self.bracket_expansion;
            b += width * self.bracket_expansion;
        }

        Err(finstack_core::Error::Internal) // Could not find bracket
    }
}

impl Solver for BrentSolver {
    fn solve<Func>(&self, f: Func, initial_guess: F) -> Result<F>
    where
        Func: Fn(F) -> F,
    {
        let (a, b) = self.find_bracket(&f, initial_guess)?;
        brent(f, a, b, self.tolerance, self.max_iterations)
    }
}

/// Hybrid solver that tries Newton first, falls back to Brent.
#[derive(Clone, Debug, Default)]
pub struct HybridSolver {
    newton: NewtonSolver,
    brent: BrentSolver,
}

impl HybridSolver {
    /// Create a new hybrid solver.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Solver for HybridSolver {
    fn solve<Func>(&self, f: Func, initial_guess: F) -> Result<F>
    where
        Func: Fn(F) -> F,
    {
        // Try Newton first
        match self.newton.solve(&f, initial_guess) {
            Ok(root) => Ok(root),
            Err(_) => {
                // Fall back to Brent
                self.brent.solve(f, initial_guess)
            }
        }
    }
}

/// Multi-dimensional optimization result.
#[derive(Clone, Debug)]
pub struct OptimizationResult {
    /// Solution vector
    pub solution: Vec<F>,
    /// Final objective value
    pub objective_value: F,
    /// Number of iterations
    pub iterations: usize,
    /// Convergence achieved
    pub converged: bool,
    /// Final gradient norm
    pub gradient_norm: F,
}

/// Multi-dimensional solver trait.
pub trait MultiDimSolver: Send + Sync {
    /// Solve multi-dimensional optimization problem.
    ///
    /// # Arguments
    /// * `objective` - Objective function f(x) -> F
    /// * `gradient` - Gradient function ∇f(x) -> Vec<F> (optional, can use finite differences)
    /// * `initial_guess` - Starting point
    /// * `bounds` - Optional parameter bounds
    fn solve_multi<ObjFunc, GradFunc>(
        &self,
        objective: ObjFunc,
        gradient: Option<GradFunc>,
        initial_guess: &[F],
        bounds: Option<&[(F, F)]>,
    ) -> Result<OptimizationResult>
    where
        ObjFunc: Fn(&[F]) -> F,
        GradFunc: Fn(&[F]) -> Vec<F>;
}

/// Levenberg-Marquardt solver for non-linear least squares.
#[derive(Clone, Debug)]
pub struct LevenbergMarquardtSolver {
    /// Convergence tolerance
    pub tolerance: F,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Initial damping parameter
    pub initial_lambda: F,
    /// Damping adjustment factor
    pub lambda_factor: F,
}

impl Default for LevenbergMarquardtSolver {
    fn default() -> Self {
        Self {
            tolerance: 1e-10,
            max_iterations: 100,
            initial_lambda: 0.001,
            lambda_factor: 10.0,
        }
    }
}

impl LevenbergMarquardtSolver {
    /// Create a new Levenberg-Marquardt solver.
    pub fn new() -> Self {
        Self::default()
    }
}

impl MultiDimSolver for LevenbergMarquardtSolver {
    fn solve_multi<ObjFunc, GradFunc>(
        &self,
        objective: ObjFunc,
        gradient: Option<GradFunc>,
        initial_guess: &[F],
        _bounds: Option<&[(F, F)]>,
    ) -> Result<OptimizationResult>
    where
        ObjFunc: Fn(&[F]) -> F,
        GradFunc: Fn(&[F]) -> Vec<F>,
    {
        let mut x = initial_guess.to_vec();
        let mut lambda = self.initial_lambda;
        let mut obj_val = objective(&x);

        for iteration in 0..self.max_iterations {
            // Compute gradient (analytical or finite differences)
            let grad = if let Some(ref grad_fn) = gradient {
                grad_fn(&x)
            } else {
                // Finite difference gradient
                let mut grad = vec![0.0; x.len()];
                let h = 1e-8;
                for i in 0..x.len() {
                    let mut x_plus = x.clone();
                    let mut x_minus = x.clone();
                    x_plus[i] += h;
                    x_minus[i] -= h;
                    grad[i] = (objective(&x_plus) - objective(&x_minus)) / (2.0 * h);
                }
                grad
            };

            // Check convergence
            let grad_norm = grad.iter().map(|g| g * g).sum::<F>().sqrt();
            if grad_norm < self.tolerance {
                return Ok(OptimizationResult {
                    solution: x,
                    objective_value: obj_val,
                    iterations: iteration,
                    converged: true,
                    gradient_norm: grad_norm,
                });
            }

            // Levenberg-Marquardt step (simplified)
            // In practice, this would use proper Hessian approximation
            for i in 0..x.len() {
                x[i] -= grad[i] / (1.0 + lambda);
            }

            let new_obj_val = objective(&x);
            if new_obj_val < obj_val {
                // Good step, reduce damping
                lambda /= self.lambda_factor;
                obj_val = new_obj_val;
            } else {
                // Bad step, increase damping and revert
                lambda *= self.lambda_factor;
                for i in 0..x.len() {
                    x[i] += grad[i] / (1.0 + lambda); // Revert step
                }
            }
        }

        // Failed to converge
        Ok(OptimizationResult {
            solution: x,
            objective_value: obj_val,
            iterations: self.max_iterations,
            converged: false,
            gradient_norm: F::INFINITY,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newton_solver() {
        let solver = NewtonSolver::new();

        // Solve x^2 - 2 = 0 (root should be sqrt(2))
        let f = |x: F| x * x - 2.0;
        let root = solver.solve(f, 1.0).unwrap();

        assert!((root - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_brent_solver() {
        let solver = BrentSolver::new();

        // Solve x^3 - x - 1 = 0 (has root around 1.32)
        let f = |x: F| x * x * x - x - 1.0;
        let root = solver.solve(f, 1.0).unwrap();

        assert!(f(root).abs() < 1e-10);
        assert!((root - 1.3247179572447).abs() < 1e-6);
    }

    #[test]
    fn test_hybrid_solver_fallback() {
        let solver = HybridSolver::new();

        // Function with discontinuous derivative (Newton may fail)
        let f = |x: F| if x > 0.0 { x - 1.0 } else { -x - 1.0 };
        let root = solver.solve(f, 0.5).unwrap();

        assert!(f(root).abs() < 1e-10);
    }

    #[test]
    fn test_lm_solver_rosenbrock() {
        let solver = LevenbergMarquardtSolver::new();

        // Minimize Rosenbrock function (minimum at (1,1))
        let objective = |x: &[F]| {
            let (a, b) = (1.0, 100.0);
            (a - x[0]).powi(2) + b * (x[1] - x[0].powi(2)).powi(2)
        };

        let result = solver
            .solve_multi(
                objective,
                None::<fn(&[F]) -> Vec<F>>, // Use finite differences
                &[0.0, 0.0],                // Start at origin
                None,
            )
            .unwrap();

        // Should converge reasonably close to (1,1)
        assert!(result.converged);
    }
}
