//! Generic solver interfaces for 1D root finding.
//!
//! This module provides unified interfaces for 1D root finding algorithms
//! commonly used in financial computations such as implied volatility calculation,
//! yield-to-maturity solving, and internal rate of return computation.
//!
//! # Algorithms
//!
//! - [`NewtonSolver`]: Newton-Raphson method with finite difference derivatives (or analytic via [`solve_with_derivative`](NewtonSolver::solve_with_derivative))
//! - [`BrentSolver`]: Brent's method (robust bracketing method)
//!
//! # Mathematical Foundation
//!
//! ## Newton-Raphson Method
//!
//! Iteratively refines an initial guess using:
//! ```text
//! x_{n+1} = x_n - f(x_n) / f'(x_n)
//! ```
//!
//! Convergence is quadratic near the root but requires a good initial guess
//! and may fail if the derivative is small or the function is non-smooth.
//!
//! ## Brent's Method
//!
//! Combines bisection, secant method, and inverse quadratic interpolation
//! to guarantee convergence while achieving superlinear convergence rate.
//! Requires a bracketing interval [a, b] where f(a) and f(b) have opposite signs.
//!
//! # Examples
//!
//! ## Newton-Raphson for square root
//!
//! ```
//! use finstack_core::math::solver::{NewtonSolver, Solver};
//!
//! let solver = NewtonSolver::new().with_tolerance(1e-10);
//! let f = |x: f64| x * x - 2.0;
//! let root = solver.solve(f, 1.0).expect("Root finding should succeed");
//! assert!((root - 2.0_f64.sqrt()).abs() < 1e-10);
//! ```
//!
//! ## Brent's method for transcendental equation
//!
//! ```
//! use finstack_core::math::solver::{BrentSolver, Solver};
//!
//! let solver = BrentSolver::new();
//! let f = |x: f64| x * x - 2.0;
//! let root = solver.solve(f, 1.5).expect("Root finding should succeed");
//! assert!((root - 2.0_f64.sqrt()).abs() < 1e-10);
//! ```
//!
//! # References
//!
//! - **Newton-Raphson**:
//!   - Press, W. H., et al. (2007). *Numerical Recipes: The Art of Scientific Computing*
//!     (3rd ed.). Cambridge University Press. Section 9.4.
//!   - Burden, R. L., & Faires, J. D. (2010). *Numerical Analysis* (9th ed.).
//!     Brooks/Cole. Section 2.3.
//!
//! - **Brent's Method**:
//!   - Brent, R. P. (1973). *Algorithms for Minimization without Derivatives*.
//!     Prentice-Hall. Chapter 4.
//!   - Press, W. H., et al. (2007). *Numerical Recipes* (3rd ed.). Section 9.3.

use crate::Result;

/// Generic solver trait for 1D root finding.
pub trait Solver: Send + Sync {
    /// Solve f(x) = 0 starting from initial guess.
    fn solve<Func>(&self, f: Func, initial_guess: f64) -> Result<f64>
    where
        Func: Fn(f64) -> f64;
}

/// Newton-Raphson solver with automatic derivative estimation.
///
/// Implements the classic Newton-Raphson root finding algorithm with finite
/// difference approximation for the derivative. This provides a balance between
/// convergence speed and ease of use (no need to provide analytical derivatives).
///
/// # Algorithm
///
/// The solver iterates using:
/// ```text
/// x_{n+1} = x_n - f(x_n) / f'(x_n)
/// ```
///
/// where `f'(x)` is approximated using scale-adaptive central differences:
/// ```text
/// h = base_step × max(|x|, 1.0)
/// f'(x) ≈ (f(x + h) - f(x - h)) / (2h)
/// ```
///
/// The adaptive step size prevents catastrophic cancellation for large-magnitude
/// problems while maintaining accuracy for small values.
///
/// # Convergence
///
/// - **Rate**: Quadratic near the root (number of correct digits roughly doubles each iteration)
/// - **Requirements**: Good initial guess, smooth function, non-zero derivative
/// - **Failure modes**: May diverge if initial guess is poor or derivative is near zero
///
/// # Use Cases
///
/// - Implied volatility calculation (Black-Scholes)
/// - Yield-to-maturity solving
/// - Internal rate of return (IRR)
/// - Duration-matched portfolio optimization
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::solver::{NewtonSolver, Solver};
///
/// // Solve for implied volatility (simplified example)
/// let target_price = 10.5;
/// let solver = NewtonSolver::new().with_tolerance(1e-6);
///
/// let price_error = |vol: f64| {
///     // In practice, this would call Black-Scholes formula
///     let price = vol * 100.0; // Simplified
///     price - target_price
/// };
///
/// let implied_vol = solver.solve(price_error, 0.2).expect("Root finding should succeed");
/// assert!((price_error(implied_vol)).abs() < 1e-6);
/// ```
///
/// # References
///
/// - Press, W. H., et al. (2007). *Numerical Recipes* (3rd ed.). Section 9.4.
///   Recommends h ≈ sqrt(epsilon) × max(|x|, 1) for scale-adaptive derivatives.
/// - Ralston, A., & Rabinowitz, P. (2001). *A First Course in Numerical Analysis*
///   (2nd ed.). Dover. Chapter 8.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NewtonSolver {
    /// Convergence tolerance
    pub tolerance: f64,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Base finite difference step for derivative estimation (scaled adaptively)
    pub fd_step: f64,
    /// Minimum derivative threshold (absolute and relative guard)
    pub min_derivative: f64,
}

impl Default for NewtonSolver {
    fn default() -> Self {
        Self {
            tolerance: 1e-12,
            max_iterations: 50,
            fd_step: 1e-8,
            min_derivative: 1e-14, // More permissive than legacy 1e-10
        }
    }
}

impl NewtonSolver {
    /// Create a new Newton solver with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set tolerance.
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set maximum iterations.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set minimum derivative threshold.
    pub fn with_min_derivative(mut self, min_derivative: f64) -> Self {
        self.min_derivative = min_derivative;
        self
    }

    /// Compute scale-adaptive finite difference step.
    ///
    /// Uses the formula: h = base_step × max(|x|, 1.0)
    ///
    /// This prevents catastrophic cancellation for large-magnitude problems
    /// while maintaining accuracy for small values.
    ///
    /// # References
    ///
    /// Press, W. H., et al. (2007). *Numerical Recipes* (3rd ed.). Section 9.4.
    #[inline]
    fn adaptive_fd_step(&self, x: f64) -> f64 {
        let scale = x.abs().max(1.0);
        self.fd_step * scale
    }
}

impl Solver for NewtonSolver {
    fn solve<Func>(&self, f: Func, initial_guess: f64) -> Result<f64>
    where
        Func: Fn(f64) -> f64,
    {
        // Use automatic differentiation via scale-adaptive finite differences
        let derivative = |x: f64| -> f64 {
            let h = self.adaptive_fd_step(x);
            let f_plus = f(x + h);
            let f_minus = f(x - h);
            (f_plus - f_minus) / (2.0 * h)
        };

        self.newton_method(&f, derivative, initial_guess)
    }
}

impl NewtonSolver {
    /// Solve using Newton-Raphson with an analytic derivative.
    ///
    /// This method provides better performance and numerical stability compared to
    /// the automatic finite-difference approach in [`solve`](Solver::solve) when
    /// an analytic derivative is available.
    ///
    /// # Performance Benefits
    ///
    /// - **2x fewer function evaluations**: No need to compute `f(x+h)` and `f(x-h)`
    /// - **Better numerical stability**: Avoids finite-difference cancellation errors
    /// - **Faster convergence**: Exact derivatives lead to more accurate Newton steps
    ///
    /// # When to Use
    ///
    /// Use this method when you can cheaply compute the derivative analytically:
    /// - **XIRR/IRR**: Derivative of NPV with respect to rate is known analytically
    /// - **Implied volatility**: Vega (∂Price/∂σ) is available from option pricing
    /// - **Yield-to-maturity**: Duration (∂Price/∂y) is known from bond pricing
    /// - **Calibration**: When instrument sensitivities are already computed
    ///
    /// # Arguments
    ///
    /// * `f` - Function to find the root of (f(x) = 0)
    /// * `f_prime` - Derivative of f with respect to x
    /// * `initial_guess` - Starting point for iteration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_core::math::solver::NewtonSolver;
    ///
    /// let solver = NewtonSolver::new();
    ///
    /// // Solve x^2 - 4 = 0 with analytic derivative (2x)
    /// let f = |x: f64| x * x - 4.0;
    /// let f_prime = |x: f64| 2.0 * x;
    ///
    /// let root = solver.solve_with_derivative(f, f_prime, 1.0)
    ///     .expect("Root finding should succeed");
    /// assert!((root - 2.0).abs() < 1e-10);
    /// ```
    ///
    /// # References
    ///
    /// - Press, W. H., et al. (2007). *Numerical Recipes* (3rd ed.). Section 9.4.
    ///   "When derivatives are available analytically, Newton-Raphson is the method of choice."
    pub fn solve_with_derivative<F, G>(&self, f: F, f_prime: G, initial_guess: f64) -> Result<f64>
    where
        F: Fn(f64) -> f64,
        G: Fn(f64) -> f64,
    {
        self.newton_method(&f, f_prime, initial_guess)
    }

    /// Core Newton-Raphson method implementation.
    fn newton_method<Func, DFunc>(&self, f: Func, f_prime: DFunc, x0: f64) -> Result<f64>
    where
        Func: Fn(f64) -> f64,
        DFunc: Fn(f64) -> f64,
    {
        use crate::error::InputError;

        let mut x = x0;

        for _ in 0..self.max_iterations {
            let fx = f(x);
            if !fx.is_finite() {
                return Err(InputError::Invalid.into());
            }

            // Check for convergence
            if fx.abs() < self.tolerance {
                return Ok(x);
            }

            let fpx = f_prime(x);
            if !fpx.is_finite() {
                return Err(InputError::Invalid.into());
            }

            // Avoid division by zero with both absolute and relative guards
            // Uses more permissive threshold (1e-14) and checks relative to function value
            const MIN_DERIVATIVE_REL: f64 = 1e-6;
            if fpx.abs() < self.min_derivative && fpx.abs() < MIN_DERIVATIVE_REL * fx.abs() {
                return Err(InputError::Invalid.into());
            }

            let x_new = x - fx / fpx;

            // Check for convergence in x
            if (x_new - x).abs() < self.tolerance {
                return Ok(x_new);
            }

            x = x_new;
        }

        Err(InputError::Invalid.into())
    }
}

/// Brent's method solver (robust, bracketing required).
///
/// Implements Brent's root-finding algorithm, which combines bisection,
/// secant method, and inverse quadratic interpolation. This provides
/// guaranteed convergence with superlinear convergence rate.
///
/// # Algorithm
///
/// Brent's method maintains a bracketing interval [a, b] where f(a) and f(b)
/// have opposite signs. At each iteration, it chooses between:
/// 1. **Inverse quadratic interpolation**: Fast when applicable
/// 2. **Secant method**: Reliable fallback
/// 3. **Bisection**: Guaranteed progress
///
/// The algorithm automatically selects the most appropriate method based on
/// convergence criteria and numerical stability.
///
/// # Convergence
///
/// - **Rate**: Superlinear (order ≈ 1.618)
/// - **Guarantee**: Always converges if initial bracket is valid
/// - **Robustness**: Handles discontinuous derivatives and poor initial guesses
///
/// # Use Cases
///
/// Preferred over Newton-Raphson when:
/// - Function has discontinuous derivatives (e.g., piecewise functions)
/// - Initial guess quality is uncertain
/// - Absolute convergence guarantee is required
/// - Function evaluation is cheap relative to derivative computation
///
/// Common applications:
/// - Bond yield-to-maturity (when price/yield curve is complex)
/// - Option implied volatility with exotic payoffs
/// - Credit curve calibration
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::solver::{BrentSolver, Solver};
///
/// let solver = BrentSolver::new();
///
/// // Solve x^3 - 2x - 5 = 0
/// let f = |x: f64| x.powi(3) - 2.0 * x - 5.0;
/// let root = solver.solve(f, 2.0).expect("Root finding should succeed");
///
/// assert!((f(root)).abs() < 1e-10);
/// assert!((root - 2.0946).abs() < 1e-4);
/// ```
///
/// # References
///
/// - Brent, R. P. (1973). *Algorithms for Minimization without Derivatives*.
///   Prentice-Hall. Chapter 4.
/// - Press, W. H., et al. (2007). *Numerical Recipes: The Art of Scientific Computing*
///   (3rd ed.). Cambridge University Press. Section 9.3.
/// - Forsythe, G. E., Malcolm, M. A., & Moler, C. B. (1977). *Computer Methods
///   for Mathematical Computations*. Prentice-Hall.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BrentSolver {
    /// Convergence tolerance
    pub tolerance: f64,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Bracket expansion factor
    pub bracket_expansion: f64,
    /// Initial bracket size (adaptive to initial guess if None)
    pub initial_bracket_size: Option<f64>,
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
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set initial bracket size. If None, will use adaptive sizing.
    pub fn with_initial_bracket_size(mut self, size: Option<f64>) -> Self {
        self.initial_bracket_size = size;
        self
    }

    /// Set maximum iterations.
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Find bracket around the root starting from initial guess.
    fn find_bracket<Func>(&self, f: &Func, initial_guess: f64) -> Result<(f64, f64)>
    where
        Func: Fn(f64) -> f64,
    {
        use crate::error::InputError;

        // Maximum bracket width to prevent overflow
        const MAX_BRACKET_WIDTH: f64 = 1e6;
        const MIN_VALUE: f64 = -1e6;
        const MAX_VALUE: f64 = 1e6;

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

            // Check for non-finite function values
            if !fa.is_finite() || !fb.is_finite() {
                return Err(InputError::Invalid.into());
            }

            if fa * fb < 0.0 {
                return Ok((a, b));
            }

            // Expand bracket with overflow protection
            let width = b - a;

            // Stop if bracket is unreasonably wide
            if width > MAX_BRACKET_WIDTH {
                break;
            }

            // Expand with bounds checking to prevent overflow
            a = (a - width * self.bracket_expansion).max(MIN_VALUE);
            b = (b + width * self.bracket_expansion).min(MAX_VALUE);

            // Stop if we've hit the bounds
            if a <= MIN_VALUE && b >= MAX_VALUE {
                break;
            }
        }

        Err(crate::Error::Calibration {
            message: format!(
                "Could not find bracket for root within [{}, {}]",
                MIN_VALUE, MAX_VALUE
            ),
            category: "root_finding".to_string(),
        })
    }
}

impl Solver for BrentSolver {
    fn solve<Func>(&self, f: Func, initial_guess: f64) -> Result<f64>
    where
        Func: Fn(f64) -> f64,
    {
        let (a, b) = self.find_bracket(&f, initial_guess)?;
        self.brent_method(f, a, b)
    }
}

impl BrentSolver {
    /// Core Brent's method implementation.
    ///
    /// Requirements: `f(lo)` and `f(hi)` must have opposite signs.
    fn brent_method<Func>(&self, mut f: Func, lo: f64, hi: f64) -> Result<f64>
    where
        Func: FnMut(f64) -> f64,
    {
        use crate::error::InputError;

        let flo = f(lo);
        let fhi = f(hi);
        // Reject non-finite endpoint evaluations
        if !(flo.is_finite() && fhi.is_finite()) {
            return Err(InputError::Invalid.into());
        }
        // Early exit if an endpoint is already a root
        if flo == 0.0 {
            return Ok(lo);
        }
        if fhi == 0.0 {
            return Ok(hi);
        }
        // Require a valid bracket
        if flo.signum() == fhi.signum() {
            return Err(InputError::Invalid.into());
        }

        let mut a = lo;
        let mut b = hi;
        let mut fa = flo;
        let mut fb = fhi;
        let mut c = a;
        let mut fc = fa;
        let mut d = b - a;
        let mut e = d;

        for _ in 0..self.max_iterations {
            if fb.signum() == fc.signum() {
                c = a;
                fc = fa;
                d = b - a;
                e = d;
            }
            if fc.abs() < fb.abs() {
                a = b;
                b = c;
                c = a;
                fa = fb;
                fb = fc;
                fc = fa;
            }
            // Convergence checks
            let tol1 = 2.0 * f64::EPSILON * b.abs() + 0.5 * self.tolerance;
            let xm = 0.5 * (c - b);
            if xm.abs() <= tol1 || fb == 0.0 {
                return Ok(b);
            }

            if e.abs() >= tol1 && fa.abs() > fb.abs() {
                // Attempt inverse quadratic interpolation or secant
                let s = fb / fa;
                let (p, q) = if a == c {
                    // Secant method
                    (2.0 * xm * s, 1.0 - s)
                } else {
                    // Inverse quadratic interpolation
                    let q1 = fa / fc;
                    let r = fb / fc;
                    let p = s * (2.0 * xm * q1 * (q1 - r) - (b - a) * (r - 1.0));
                    let q = (q1 - 1.0) * (r - 1.0) * (s - 1.0);
                    (p, q)
                };
                let mut p = p;
                let mut q = q;
                if p > 0.0 {
                    q = -q;
                } else {
                    p = -p;
                }
                let cond1 = 2.0 * p < 3.0 * xm * q - (tol1 * q).abs();
                let cond2 = p < (e * q).abs() * 0.5;
                if cond1 && cond2 {
                    e = d;
                    d = p / q;
                } else {
                    d = xm;
                    e = d;
                }
            } else {
                d = xm;
                e = d;
            }

            a = b;
            fa = fb;
            if d.abs() > tol1 {
                b += d;
            } else {
                b += tol1.copysign(xm);
            }
            fb = f(b);
        }

        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newton_solver() {
        let solver = NewtonSolver::new();

        // Solve x^2 - 2 = 0 (root should be sqrt(2))
        let f = |x: f64| x * x - 2.0;
        let root = solver
            .solve(f, 1.0)
            .expect("Root finding should succeed in test");

        assert!((root - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_brent_solver() {
        let solver = BrentSolver::new();

        // Solve x^3 - x - 1 = 0 (has root around 1.32)
        let f = |x: f64| x * x * x - x - 1.0;
        let root = solver
            .solve(f, 1.0)
            .expect("Root finding should succeed in test");

        assert!(f(root).abs() < 1e-10);
        assert!((root - 1.3247179572447).abs() < 1e-6);
    }

    #[test]
    fn test_brent_solver_adaptive_bracket() {
        // Test with large initial guess to verify adaptive bracketing
        let solver = BrentSolver::new();

        // Solve x - 100 = 0 (root at x = 100)
        let f = |x: f64| x - 100.0;
        let root = solver
            .solve(f, 95.0)
            .expect("Root finding should succeed in test"); // Start near the root

        assert!(f(root).abs() < 1e-10);
        assert!((root - 100.0).abs() < 1e-6);

        // Test with configurable bracket size
        let solver_custom = BrentSolver::new().with_initial_bracket_size(Some(5.0));
        let root2 = solver_custom
            .solve(f, 95.0)
            .expect("Root finding should succeed in test");
        assert!(f(root2).abs() < 1e-10);
    }

    // ===== Phase 1 Robustness Tests =====

    #[test]
    fn test_newton_scale_robustness() {
        // Test solver across magnitude scales 10^-6 to 10^6
        let solver = NewtonSolver::new();

        for exp in -6..=6 {
            let target = 10f64.powi(exp);
            let f = |x: f64| x * x - target;
            let root = solver
                .solve(f, target.sqrt() * 0.9)
                .unwrap_or_else(|_| panic!("Root finding should succeed at scale 10^{}", exp));

            // Use scale-aware tolerance accounting for floating point precision
            // For small targets, absolute error dominates; for large targets, relative error
            let abs_tolerance: f64 = 1e-12; // Absolute tolerance near machine precision
            let rel_tolerance = 1e-10 * target; // Relative tolerance
            let tolerance = abs_tolerance.max(rel_tolerance);

            assert!(
                (f(root)).abs() < tolerance,
                "Failed at scale 10^{}: residual {} (tolerance {})",
                exp,
                f(root),
                tolerance
            );
        }
    }

    #[test]
    fn test_newton_shallow_slope() {
        // Test Newton solver with shallow derivative but valid root
        let solver = NewtonSolver::new();

        // f(x) = x^4 - 1e-8, root at x ≈ 0.01
        // At x=0.01: f'(x) = 4x^3 = 4e-6 (was rejected by legacy guard)
        let f = |x: f64| x.powi(4) - 1e-8;
        let root = solver
            .solve(f, 0.02)
            .expect("Should solve function with shallow derivative");

        assert!((f(root)).abs() < 1e-12, "Residual: {}", f(root));
        assert!((root - 0.01).abs() < 1e-6, "Root: {}", root);
    }

    #[test]
    fn test_brent_overflow_protection() {
        // Test that Brent solver doesn't overflow on pathological functions
        let solver = BrentSolver::new();

        // Function with no roots (always positive)
        let f = |x: f64| x * x + 1.0;
        let result = solver.solve(f, 0.0);

        // Should fail gracefully, not panic or return NaN
        assert!(result.is_err(), "Should fail to find root of x^2 + 1");
    }

    #[test]
    fn test_brent_pathological_functions() {
        let solver = BrentSolver::new();

        // Flat function (derivative = 0 everywhere)
        let flat = |_x: f64| 1.0;
        assert!(
            solver.solve(flat, 0.0).is_err(),
            "Should reject flat function"
        );

        // Discontinuous function with root at 0
        let step = |x: f64| if x >= 0.0 { 1.0 } else { -1.0 };
        let root = solver
            .solve(step, 0.5)
            .expect("Should find root at discontinuity");
        assert!(root.abs() < 1e-6, "Root: {}", root);
    }

    #[test]
    fn test_newton_adaptive_fd_step() {
        // Verify adaptive FD step prevents cancellation errors
        let solver = NewtonSolver::new();

        // Large-scale problem: x^2 = 1,000,000
        let f_large = |x: f64| x * x - 1_000_000.0;
        let root_large = solver
            .solve(f_large, 900.0)
            .expect("Should solve large-scale problem");
        assert!((root_large - 1000.0).abs() < 1e-6);

        // Small-scale problem: x^2 = 0.000001
        let f_small = |x: f64| x * x - 1e-6;
        let root_small = solver
            .solve(f_small, 0.0009)
            .expect("Should solve small-scale problem");
        assert!((root_small - 0.001).abs() < 1e-9);
    }

    #[test]
    fn test_solver_convergence_comparison() {
        // Compare Newton vs Brent on well-behaved function
        let newton = NewtonSolver::new();
        let brent = BrentSolver::new();

        let f = |x: f64| x * x * x - x - 1.0; // Root ≈ 1.3247

        let root_newton = newton.solve(f, 1.0).expect("Newton should converge");
        let root_brent = brent.solve(f, 1.0).expect("Brent should converge");

        // Both should find the same root
        assert!((root_newton - root_brent).abs() < 1e-6);
        assert!((f(root_newton)).abs() < 1e-10);
    }

    #[test]
    fn test_configurable_min_derivative() {
        // Test that min_derivative threshold is configurable
        let _solver_strict = NewtonSolver::new().with_min_derivative(1e-10);
        let solver_permissive = NewtonSolver::new().with_min_derivative(1e-16);

        // Function with very small derivative
        let f = |x: f64| x.powi(5) - 1e-12;

        // Permissive solver should succeed where strict might fail
        let root = solver_permissive
            .solve(f, 0.001)
            .expect("Permissive solver should handle shallow slopes");
        assert!((f(root)).abs() < 1e-12);
    }

    #[test]
    fn test_solve_with_derivative_quadratic() {
        // Test analytic derivative on simple quadratic
        let solver = NewtonSolver::new();

        // Solve x^2 - 4 = 0 (root at x = 2)
        let f = |x: f64| x * x - 4.0;
        let f_prime = |x: f64| 2.0 * x;

        let root = solver
            .solve_with_derivative(f, f_prime, 1.0)
            .expect("Root finding with analytic derivative should succeed");

        assert!((root - 2.0).abs() < 1e-10);
        assert!((f(root)).abs() < 1e-10);
    }

    #[test]
    fn test_solve_with_derivative_vs_finite_diff() {
        // Compare analytic derivative to finite difference on same function
        let solver = NewtonSolver::new();

        // Cubic function: x^3 - 2x - 5 = 0
        let f = |x: f64| x.powi(3) - 2.0 * x - 5.0;
        let f_prime = |x: f64| 3.0 * x.powi(2) - 2.0;

        let root_analytic = solver
            .solve_with_derivative(f, f_prime, 2.0)
            .expect("Analytic derivative should succeed");

        let root_fd = solver
            .solve(f, 2.0)
            .expect("Finite difference should succeed");

        // Both should converge to same root
        assert!(
            (root_analytic - root_fd).abs() < 1e-9,
            "Analytic and FD roots differ: {} vs {}",
            root_analytic,
            root_fd
        );
        assert!((f(root_analytic)).abs() < 1e-10);
    }

    #[test]
    fn test_solve_with_derivative_exponential() {
        // Test on transcendental equation: e^x - 3x = 0
        let solver = NewtonSolver::new();

        let f = |x: f64| x.exp() - 3.0 * x;
        let f_prime = |x: f64| x.exp() - 3.0;

        let root = solver
            .solve_with_derivative(f, f_prime, 1.0)
            .expect("Should solve exponential equation");

        assert!((f(root)).abs() < 1e-10);
        // One root is around x ≈ 0.619 (there's also one near 1.512)
        assert!(root > 0.0 && root < 2.0);
    }
}
