//! Statistical distribution functions and sampling methods.
//!
//! This module provides pure mathematical implementations of common statistical
//! distributions and sampling algorithms, independent of any financial context.
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::distributions::{binomial_probability, sample_beta};
//!
//! // Calculate binomial probability P(X = 5) where X ~ Binomial(10, 0.5)
//! let prob = binomial_probability(10, 5, 0.5);
//! assert!((prob - 0.24609375).abs() < 1e-6);
//! ```

use super::random::RandomNumberGenerator;
use crate::F;

/// Calculate binomial probability: P(X = k) where X ~ Binomial(n, p)
///
/// Uses log-space calculation to avoid overflow for large n.
///
/// # Arguments
/// * `n` - Number of trials
/// * `k` - Number of successes
/// * `p` - Probability of success per trial
///
/// # Returns
/// The probability P(X = k)
pub fn binomial_probability(n: usize, k: usize, p: F) -> F {
    if k > n {
        return 0.0;
    }
    if p <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    if p >= 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }

    // Use log-space calculation to avoid overflow for large n
    let log_prob =
        log_binomial_coefficient(n, k) + (k as F) * p.ln() + ((n - k) as F) * (1.0 - p).ln();
    log_prob.exp()
}

/// Calculate log of binomial coefficient: ln(n choose k)
///
/// Uses Stirling's approximation for large factorials to maintain numerical stability.
///
/// # Arguments
/// * `n` - Total items
/// * `k` - Items to choose
///
/// # Returns
/// ln(n! / (k! * (n-k)!))
pub fn log_binomial_coefficient(n: usize, k: usize) -> F {
    if k > n {
        return F::NEG_INFINITY;
    }
    if k == 0 || k == n {
        return 0.0;
    }

    // Use the more efficient calculation: ln(n!) - ln(k!) - ln((n-k)!)
    // Using Stirling's approximation for large values
    log_factorial(n) - log_factorial(k) - log_factorial(n - k)
}

/// Calculate log factorial using exact calculation for small n, Stirling's approximation for large n.
///
/// # Arguments
/// * `n` - Input value
///
/// # Returns
/// ln(n!)
pub fn log_factorial(n: usize) -> F {
    if n == 0 || n == 1 {
        return 0.0;
    }
    if n < 20 {
        // Exact calculation for small n: ln(n!) = ln(1) + ln(2) + ... + ln(n)
        (2..=n).map(|i| (i as F).ln()).sum()
    } else {
        // Stirling's approximation: ln(n!) ≈ n*ln(n) - n + 0.5*ln(2πn)
        let n_f = n as F;
        n_f * n_f.ln() - n_f + 0.5 * (2.0 * std::f64::consts::PI * n_f).ln()
    }
}

/// Sample from a beta distribution using transformation method.
///
/// This is a simplified implementation suitable for most financial applications.
/// For more sophisticated sampling needs, consider external crates.
///
/// # Arguments
/// * `rng` - Random number generator
/// * `alpha` - First shape parameter (> 0)
/// * `beta` - Second shape parameter (> 0)
///
/// # Returns
/// Random sample from Beta(alpha, beta) distribution
pub fn sample_beta(rng: &mut dyn RandomNumberGenerator, alpha: F, beta: F) -> F {
    // Use inverse transform for simple cases
    if alpha == 1.0 && beta == 1.0 {
        return rng.uniform();
    }

    // Simplified beta sampling for common cases using transformation method
    let u1 = rng.uniform();
    let u2 = rng.uniform();

    // Use transformation method for beta(alpha, beta)
    let x = u1.powf(1.0 / alpha);
    let y = u2.powf(1.0 / beta);
    x / (x + y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binomial_probability() {
        // Test known values
        assert!((binomial_probability(10, 5, 0.5) - 0.24609375).abs() < 1e-6);
        assert!((binomial_probability(5, 0, 0.1) - 0.59049).abs() < 1e-6);

        // Test edge cases
        assert_eq!(binomial_probability(10, 0, 0.0), 1.0);
        assert_eq!(binomial_probability(10, 10, 1.0), 1.0);
        assert_eq!(binomial_probability(10, 5, 0.0), 0.0);
    }

    #[test]
    fn test_log_factorial() {
        // Test small values (exact calculation)
        assert!((log_factorial(1) - 0.0).abs() < 1e-12);
        assert!(
            (log_factorial(5) - (2.0_f64.ln() + 3.0_f64.ln() + 4.0_f64.ln() + 5.0_f64.ln())).abs()
                < 1e-12
        );

        // Test large values (Stirling approximation)
        let log_100_factorial = log_factorial(100);
        assert!(log_100_factorial > 360.0 && log_100_factorial < 365.0);
    }

    #[test]
    fn test_log_binomial_coefficient() {
        // Test known values
        // C(10, 5) = 252, ln(252) ≈ 5.53039
        let actual = log_binomial_coefficient(10, 5);
        let expected = 252.0_f64.ln(); // More precise expected value
        assert!(
            (actual - expected).abs() < 1e-4,
            "Expected {}, got {}",
            expected,
            actual
        );

        // Test edge cases
        assert_eq!(log_binomial_coefficient(5, 0), 0.0);
        assert_eq!(log_binomial_coefficient(5, 5), 0.0);
        assert_eq!(log_binomial_coefficient(3, 5), F::NEG_INFINITY);
    }

    #[test]
    fn test_sample_beta() {
        use super::super::random::SimpleRng;

        let mut rng = SimpleRng::new(42);

        // Test uniform case (alpha=1, beta=1)
        let uniform_sample = sample_beta(&mut rng as &mut dyn RandomNumberGenerator, 1.0, 1.0);
        assert!((0.0..=1.0).contains(&uniform_sample));

        // Test that samples are in [0, 1]
        let samples: Vec<F> = (0..100)
            .map(|_| sample_beta(&mut rng as &mut dyn RandomNumberGenerator, 2.0, 2.0))
            .collect();
        for sample in samples {
            assert!((0.0..=1.0).contains(&sample));
        }
    }
}
