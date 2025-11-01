//! Generic solver interfaces for 1D root finding.
//!
//! This module provides unified interfaces for 1D root finding algorithms
//! commonly used in financial computations such as implied volatility calculation,
//! yield-to-maturity solving, and internal rate of return computation.
//!
//! # Algorithms
//!
//! - [`NewtonSolver`]: Newton-Raphson method with finite difference derivatives
//! - [`BrentSolver`]: Brent's method (robust bracketing method)
//! - [`HybridSolver`]: Automatic fallback from Newton to Brent
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
//! let root = solver.solve(f, 1.0).unwrap();
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
//! let root = solver.solve(f, 1.5).unwrap();
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
/// where `f'(x)` is approximated using central differences:
/// ```text
/// f'(x) ≈ (f(x + h) - f(x - h)) / (2h)
/// ```
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
/// let implied_vol = solver.solve(price_error, 0.2).unwrap();
/// assert!((price_error(implied_vol)).abs() < 1e-6);
/// ```
///
/// # References
///
/// - Press, W. H., et al. (2007). *Numerical Recipes* (3rd ed.). Section 9.4.
/// - Ralston, A., & Rabinowitz, P. (2001). *A First Course in Numerical Analysis*
///   (2nd ed.). Dover. Chapter 8.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NewtonSolver {
    /// Convergence tolerance
    pub tolerance: f64,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Finite difference step for derivative estimation
    pub fd_step: f64,
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
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
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
    fn solve<Func>(&self, f: Func, initial_guess: f64) -> Result<f64>
    where
        Func: Fn(f64) -> f64,
    {
        // Use automatic differentiation via finite differences
        let derivative = |x: f64| -> f64 {
            let f_plus = f(x + self.fd_step);
            let f_minus = f(x - self.fd_step);
            (f_plus - f_minus) / (2.0 * self.fd_step)
        };

        self.newton_method(&f, derivative, initial_guess)
    }
}

impl NewtonSolver {
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

            // Avoid division by zero
            if fpx.abs() < 1e-10 {
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
/// let root = solver.solve(f, 2.0).unwrap();
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
#[derive(Clone, Debug)]
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

    /// Find bracket around the root starting from initial guess.
    fn find_bracket<Func>(&self, f: &Func, initial_guess: f64) -> Result<(f64, f64)>
    where
        Func: Fn(f64) -> f64,
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

/// Hybrid solver that tries Newton first, falls back to Brent.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
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
    fn solve<Func>(&self, f: Func, initial_guess: f64) -> Result<f64>
    where
        Func: Fn(f64) -> f64,
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
        let f = |x: f64| x * x - 2.0;
        let root = solver.solve(f, 1.0).unwrap();

        assert!((root - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_brent_solver() {
        let solver = BrentSolver::new();

        // Solve x^3 - x - 1 = 0 (has root around 1.32)
        let f = |x: f64| x * x * x - x - 1.0;
        let root = solver.solve(f, 1.0).unwrap();

        assert!(f(root).abs() < 1e-10);
        assert!((root - 1.3247179572447).abs() < 1e-6);
    }

    #[test]
    fn test_hybrid_solver_fallback() {
        let solver = HybridSolver::new();

        // Function with discontinuous derivative (Newton may fail)
        let f = |x: f64| if x > 0.0 { x - 1.0 } else { -x - 1.0 };
        let root = solver.solve(f, 0.5).unwrap();

        assert!(f(root).abs() < 1e-10);
    }

    #[test]
    fn test_brent_solver_adaptive_bracket() {
        // Test with large initial guess to verify adaptive bracketing
        let solver = BrentSolver::new();

        // Solve x - 100 = 0 (root at x = 100)
        let f = |x: f64| x - 100.0;
        let root = solver.solve(f, 95.0).unwrap(); // Start near the root

        assert!(f(root).abs() < 1e-10);
        assert!((root - 100.0).abs() < 1e-6);

        // Test with configurable bracket size
        let solver_custom = BrentSolver::new().with_initial_bracket_size(Some(5.0));
        let root2 = solver_custom.solve(f, 95.0).unwrap();
        assert!(f(root2).abs() < 1e-10);
    }
}
