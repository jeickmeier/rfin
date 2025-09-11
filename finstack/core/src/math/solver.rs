//! Generic solver interfaces for 1D root finding.
//!
//! This module provides unified interfaces for 1D root finding algorithms
//! used throughout financial computations.
//!
//! The module includes:
//! - High-level solver trait interfaces for configurable, reusable solvers
//! - Object-oriented wrappers around core root finding algorithms
//! - Robust solver implementations with automatic fallback mechanisms
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::solver::{NewtonSolver, Solver};
//!
//! let solver = NewtonSolver::new().with_tolerance(1e-10);
//! let f = |x: f64| x * x - 2.0;
//! let root = solver.solve(f, 1.0).unwrap();
//! assert!((root - 2.0_f64.sqrt()).abs() < 1e-10);
//! ```

use super::root_finding::{brent, newton_raphson};
use crate::{Result, F};

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
    /// Initial bracket size (adaptive to initial guess if None)
    pub initial_bracket_size: Option<F>,
}

impl Default for BrentSolver {
    fn default() -> Self {
        Self {
            tolerance: 1e-12,
            max_iterations: 100,
            bracket_expansion: 2.0,
            initial_bracket_size: None, // Adaptive by default
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

    /// Set initial bracket size. If None, will use adaptive sizing.
    pub fn with_initial_bracket_size(mut self, size: Option<F>) -> Self {
        self.initial_bracket_size = size;
        self
    }

    /// Find bracket around the root starting from initial guess.
    fn find_bracket<Func>(&self, f: &Func, initial_guess: F) -> Result<(F, F)>
    where
        Func: Fn(F) -> F,
    {
        // Calculate adaptive initial bracket size
        let initial_size = self.initial_bracket_size.unwrap_or_else(|| {
            // Use 1% of the initial guess magnitude, with a minimum of 0.01
            let adaptive_size = initial_guess.abs() * 0.01;
            if adaptive_size < 1e-6 {
                0.01 // Fallback for values near zero
            } else {
                adaptive_size.min(1.0) // Cap at 1.0 for very large initial guesses
            }
        });

        let mut a = initial_guess - initial_size;
        let mut b = initial_guess + initial_size;

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

        Err(crate::Error::Internal) // Could not find bracket
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

    /// Set a unified tolerance for both Newton and Brent components.
    ///
    /// This ensures consistent convergence criteria regardless of which
    /// method ultimately succeeds.
    pub fn with_tolerance(mut self, tolerance: F) -> Self {
        self.newton.tolerance = tolerance;
        self.brent.tolerance = tolerance;
        self
    }

    /// Set a unified maximum iteration cap for both Newton and Brent components.
    ///
    /// This provides predictable iteration limits across fallback paths.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.newton.max_iterations = max_iterations;
        self.brent.max_iterations = max_iterations;
        self
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
    fn test_brent_solver_adaptive_bracket() {
        // Test with large initial guess to verify adaptive bracketing
        let solver = BrentSolver::new();

        // Solve x - 100 = 0 (root at x = 100)
        let f = |x: F| x - 100.0;
        let root = solver.solve(f, 95.0).unwrap(); // Start near the root

        assert!(f(root).abs() < 1e-10);
        assert!((root - 100.0).abs() < 1e-6);

        // Test with configurable bracket size
        let solver_custom = BrentSolver::new().with_initial_bracket_size(Some(5.0));
        let root2 = solver_custom.solve(f, 95.0).unwrap();
        assert!(f(root2).abs() < 1e-10);
    }
}
