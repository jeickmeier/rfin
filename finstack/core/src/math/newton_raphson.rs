//! Newton-Raphson root finding algorithm with automatic fallback.
//!
//! Provides a robust implementation of the Newton-Raphson method for finding
//! roots of functions, with automatic fallback to Brent's method when
//! convergence is poor.

use crate::{F, Result};
use crate::error::InputError;

/// Configuration for Newton-Raphson solver
#[derive(Clone, Debug)]
pub struct NewtonRaphsonConfig {
    /// Maximum number of iterations
    pub max_iterations: usize,
    /// Absolute tolerance for convergence
    pub tolerance: F,
    /// Relative tolerance for convergence
    pub relative_tolerance: F,
    /// Maximum allowed step size
    pub max_step: F,
    /// Minimum allowed step size (to detect convergence issues)
    pub min_step: F,
    /// Whether to automatically fallback to Brent's method
    pub auto_fallback: bool,
}

impl Default for NewtonRaphsonConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            tolerance: 1e-12,
            relative_tolerance: 1e-10,
            max_step: 1.0,
            min_step: 1e-15,
            auto_fallback: true,
        }
    }
}

/// Result of Newton-Raphson solver
#[derive(Clone, Debug)]
pub struct NewtonRaphsonResult {
    /// The found root
    pub root: F,
    /// Number of iterations taken
    pub iterations: usize,
    /// Final error estimate
    pub error: F,
    /// Whether fallback was used
    pub used_fallback: bool,
}

/// Newton-Raphson solver with automatic fallback
///
/// Finds a root of the equation f(x) = 0 using Newton-Raphson iteration
/// with automatic fallback to Brent's method if convergence is poor.
///
/// # Arguments
/// * `f` - The function to find the root of
/// * `df` - The derivative of the function
/// * `initial_guess` - Starting point for iteration
/// * `config` - Solver configuration
///
/// # Returns
/// * `Ok(NewtonRaphsonResult)` - The solution and metadata
/// * `Err(MathError)` - If no root could be found
///
/// # Example
/// ```rust
/// use finstack_core::math::newton_raphson::{newton_raphson_with_fallback, NewtonRaphsonConfig};
///
/// // Find root of x^2 - 2 = 0 (sqrt(2))
/// let f = |x: f64| x * x - 2.0;
/// let df = |x: f64| 2.0 * x;
///
/// let config = NewtonRaphsonConfig::default();
/// let result = newton_raphson_with_fallback(f, df, 1.5, &config).unwrap();
/// assert!((result.root - 1.41421356237).abs() < 1e-10);
/// ```
pub fn newton_raphson_with_fallback<F, D>(
    f: F,
    df: D,
    initial_guess: f64,
    config: &NewtonRaphsonConfig,
) -> Result<NewtonRaphsonResult>
where
    F: Fn(f64) -> f64,
    D: Fn(f64) -> f64,
{
    let mut x = initial_guess;
    let mut iterations = 0;
    let mut last_error = f64::INFINITY;
    let mut convergence_failures = 0;
    
    // Try Newton-Raphson first
    while iterations < config.max_iterations {
        let fx = f(x);
        
        // Check for convergence
        if fx.abs() < config.tolerance {
            return Ok(NewtonRaphsonResult {
                root: x,
                iterations,
                error: fx.abs(),
                used_fallback: false,
            });
        }
        
        // Check relative convergence
        if iterations > 0 && (fx.abs() / initial_guess.abs()).abs() < config.relative_tolerance {
            return Ok(NewtonRaphsonResult {
                root: x,
                iterations,
                error: fx.abs(),
                used_fallback: false,
            });
        }
        
        let dfx = df(x);
        
        // Check for zero derivative
        if dfx.abs() < 1e-15 {
            if config.auto_fallback {
                break; // Switch to fallback
            } else {
                return Err(InputError::Invalid.into());
            }
        }
        
        // Calculate Newton step
        let step = fx / dfx;
        
        // Check for convergence issues
        if step.abs() < config.min_step {
            return Ok(NewtonRaphsonResult {
                root: x,
                iterations,
                error: fx.abs(),
                used_fallback: false,
            });
        }
        
        // Limit step size
        let limited_step = if step.abs() > config.max_step {
            config.max_step * step.signum()
        } else {
            step
        };
        
        // Update x
        x -= limited_step;
        
        // Check for divergence
        if fx.abs() > last_error {
            convergence_failures += 1;
            if convergence_failures > 3 && config.auto_fallback {
                break; // Switch to fallback
            }
        } else {
            convergence_failures = 0;
        }
        
        last_error = fx.abs();
        iterations += 1;
    }
    
    // Fallback to Brent's method if enabled
    if config.auto_fallback {
        use super::root_finding::brent;
        
        // Determine bracket for Brent's method
        let (a, b) = determine_bracket(&f, initial_guess);
        
        match brent(f, a, b, config.tolerance, config.max_iterations) {
            Ok(root) => Ok(NewtonRaphsonResult {
                root,
                iterations: config.max_iterations, // Approximate
                error: config.tolerance,
                used_fallback: true,
            }),
            Err(_) => Err(InputError::Invalid.into()),
        }
    } else {
        Err(InputError::Invalid.into())
    }
}

/// Simple Newton-Raphson without fallback
///
/// # Example
/// ```rust
/// use finstack_core::math::newton_raphson::newton_raphson;
///
/// let f = |x: f64| x * x - 2.0;
/// let df = |x: f64| 2.0 * x;
///
/// let root = newton_raphson(f, df, 1.5, 1e-10, 50).unwrap();
/// assert!((root - 1.41421356237).abs() < 1e-10);
/// ```
pub fn newton_raphson<F, D>(
    f: F,
    df: D,
    initial_guess: f64,
    tolerance: f64,
    max_iterations: usize,
) -> Result<f64>
where
    F: Fn(f64) -> f64,
    D: Fn(f64) -> f64,
{
    let config = NewtonRaphsonConfig {
        max_iterations,
        tolerance,
        auto_fallback: false,
        ..Default::default()
    };
    
    newton_raphson_with_fallback(f, df, initial_guess, &config)
        .map(|result| result.root)
}

/// Determine a bracket for root finding
fn determine_bracket<F>(f: &F, initial: f64) -> (f64, f64)
where
    F: Fn(f64) -> f64,
{
    let _f_initial = f(initial);
    
    // Try to find a bracket around the initial guess
    let mut a = initial - 1.0;
    let mut b = initial + 1.0;
    
    // Expand bracket if needed
    for _ in 0..10 {
        let fa = f(a);
        let fb = f(b);
        
        if fa * fb < 0.0 {
            return (a, b);
        }
        
        if fa.abs() < fb.abs() {
            a -= b - a;
        } else {
            b += b - a;
        }
    }
    
    // Default bracket if we can't find a sign change
    (-10.0, 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_newton_raphson_quadratic() {
        // Find root of x^2 - 2 = 0
        let f = |x: f64| x * x - 2.0;
        let df = |x: f64| 2.0 * x;
        
        let root = newton_raphson(f, df, 1.5, 1e-10, 50).unwrap();
        assert!((root - 2.0_f64.sqrt()).abs() < 1e-10);
    }
    
    #[test]
    fn test_newton_raphson_with_fallback() {
        // Function with poor convergence properties
        let f = |x: f64| x.powi(3) - 2.0 * x - 5.0;
        let df = |x: f64| 3.0 * x.powi(2) - 2.0;
        
        let config = NewtonRaphsonConfig::default();
        let result = newton_raphson_with_fallback(f, df, 0.0, &config).unwrap();
        
        // Check that we found a root
        assert!(f(result.root).abs() < 1e-10);
    }
    
    #[test]
    fn test_newton_raphson_zero_derivative() {
        // Function with zero derivative at root: f(x) = x³
        // This is a challenging case because df(x) = 3x² → 0 as x → 0
        let f = |x: f64| x.powi(3);
        let df = |x: f64| 3.0 * x.powi(2);
        
        let config = NewtonRaphsonConfig::default();
        let result = newton_raphson_with_fallback(f, df, 0.1, &config);
        
        // Should find a point where f(x) ≈ 0, even if x isn't exactly 0
        match result {
            Ok(res) => {
                // For functions with zero derivative at root, Newton-Raphson
                // may not converge exactly to the root, but should find where f(x) ≈ 0
                assert!(res.error < 1e-10, "Function value {} should be near zero", res.error);
                // For this specific function, check that we're reasonably close to the true root
                assert!(res.root.abs() < 1e-3, "Root {} should be reasonably close to 0", res.root);
            }
            Err(e) => {
                panic!("Failed to find root: {:?}", e);
            }
        }
    }
}
