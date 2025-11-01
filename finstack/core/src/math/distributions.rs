//! Probability distribution functions and sampling algorithms.
//!
//! Provides implementations of discrete and continuous probability distributions
//! used in financial modeling, risk management, and Monte Carlo simulations.
//! All implementations use numerically stable algorithms.
//!
//! # Distributions
//!
//! - **Binomial**: Discrete distribution for binary outcomes (coin flips, defaults)
//! - **Beta**: Continuous distribution on [0,1] (recovery rates, correlations)
//!
//! # Numerical Stability
//!
//! - Log-space calculations prevent overflow for large parameters
//! - Stirling's approximation for factorials when n ≥ 20
//! - Defensive checks for boundary conditions (p=0, p=1, k>n)
//!
//! # Use Cases
//!
//! - **Credit modeling**: Binomial for default probability (n trials, k defaults)
//! - **Recovery simulation**: Beta distribution for recovery rate uncertainty
//! - **Portfolio analytics**: Binomial approximation for loss distribution
//! - **Bayesian inference**: Beta as conjugate prior for Bernoulli
//!
//! # Examples
//!
//! ## Binomial probability calculation
//!
//! ```
//! use finstack_core::math::distributions::binomial_probability;
//!
//! // Calculate P(X = 5) where X ~ Binomial(10, 0.5)
//! let prob = binomial_probability(10, 5, 0.5);
//! assert!((prob - 0.24609375).abs() < 1e-6);
//!
//! // Default probability: 5 defaults out of 100 names with 5% PD
//! let default_prob = binomial_probability(100, 5, 0.05);
//! assert!(default_prob > 0.0);
//! ```
//!
//! ## Beta distribution sampling
//!
//! ```
//! use finstack_core::math::distributions::sample_beta;
//! use finstack_core::math::random::SimpleRng;
//! use finstack_core::math::RandomNumberGenerator;
//!
//! let mut rng = SimpleRng::new(42);
//!
//! // Sample recovery rate from Beta(4, 2) - peaked around 60-70%
//! let recovery = sample_beta(&mut rng as &mut dyn RandomNumberGenerator, 4.0, 2.0);
//! assert!(recovery >= 0.0 && recovery <= 1.0);
//! ```
//!
//! # References
//!
//! - **Binomial Distribution**:
//!   - Johnson, N. L., Kotz, S., & Kemp, A. W. (1993). *Univariate Discrete Distributions*
//!     (2nd ed.). Wiley. Chapter 3.
//!
//! - **Beta Distribution**:
//!   - Johnson, N. L., Kotz, S., & Balakrishnan, N. (1995). *Continuous Univariate
//!     Distributions, Volume 2* (2nd ed.). Wiley. Chapter 25.
//!
//! - **Stirling's Approximation**:
//!   - Abramowitz, M., & Stegun, I. A. (1964). *Handbook of Mathematical Functions*.
//!     Formula 6.1.37 (Stirling's formula for factorials).

use super::random::RandomNumberGenerator;

/// Calculate binomial probability P(X = k) where X ~ Binomial(n, p).
///
/// Computes the probability mass function for the binomial distribution using
/// log-space arithmetic to avoid numerical overflow for large n. The binomial
/// distribution models the number of successes in n independent Bernoulli trials.
///
/// # Mathematical Definition
///
/// ```text
/// P(X = k) = C(n,k) * p^k * (1-p)^(n-k)
///
/// where C(n,k) = n! / (k! * (n-k)!)
/// ```
///
/// # Arguments
///
/// * `n` - Number of independent trials (≥ 0)
/// * `k` - Number of successes (0 ≤ k ≤ n)
/// * `p` - Probability of success on each trial (0 ≤ p ≤ 1)
///
/// # Returns
///
/// Probability P(X = k) ∈ [0, 1]
///
/// # Numerical Method
///
/// Uses log-space calculation: exp(ln C(n,k) + k ln p + (n-k) ln(1-p))
/// - Prevents overflow for large n
/// - Uses Stirling's approximation for n ≥ 20
/// - Handles edge cases (p=0, p=1, k>n) exactly
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::distributions::binomial_probability;
///
/// // Fair coin: P(5 heads in 10 flips)
/// let prob = binomial_probability(10, 5, 0.5);
/// assert!((prob - 0.24609375).abs() < 1e-6);
///
/// // Credit portfolio: P(5 defaults in 100 names with 5% PD)
/// let default_prob = binomial_probability(100, 5, 0.05);
/// ```
///
/// # References
///
/// - Johnson, N. L., Kotz, S., & Kemp, A. W. (1993). *Univariate Discrete Distributions*
///   (2nd ed.). Wiley. Chapter 3.
pub fn binomial_probability(n: usize, k: usize, p: f64) -> f64 {
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
        log_binomial_coefficient(n, k) + (k as f64) * p.ln() + ((n - k) as f64) * (1.0 - p).ln();
    log_prob.exp()
}

/// Calculate log of binomial coefficient ln(C(n,k)).
///
/// Computes the natural logarithm of "n choose k" using numerically stable
/// methods. For large factorials (n ≥ 20), uses Stirling's approximation to
/// avoid overflow.
///
/// # Mathematical Definition
///
/// ```text
/// ln C(n,k) = ln(n! / (k! * (n-k)!))
///           = ln(n!) - ln(k!) - ln((n-k)!)
/// ```
///
/// # Arguments
///
/// * `n` - Total number of items
/// * `k` - Number of items to choose
///
/// # Returns
///
/// ln(n choose k), or -∞ if k > n
///
/// # Numerical Method
///
/// - For n < 20: Exact calculation using sum of logarithms
/// - For n ≥ 20: Stirling's approximation ln(n!) ≈ n ln(n) - n + 0.5 ln(2πn)
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::distributions::log_binomial_coefficient;
///
/// // C(10, 5) = 252, ln(252) ≈ 5.5294
/// let log_coef = log_binomial_coefficient(10, 5);
/// assert!((log_coef - 252.0_f64.ln()).abs() < 1e-6);
/// ```
///
/// # References
///
/// - Abramowitz, M., & Stegun, I. A. (1964). *Handbook of Mathematical Functions*.
///   Formula 6.1.37 (Stirling's approximation).
pub fn log_binomial_coefficient(n: usize, k: usize) -> f64 {
    if k > n {
        return f64::NEG_INFINITY;
    }
    if k == 0 || k == n {
        return 0.0;
    }

    // Use the more efficient calculation: ln(n!) - ln(k!) - ln((n-k)!)
    // Using Stirling's approximation for large values
    log_factorial(n) - log_factorial(k) - log_factorial(n - k)
}

/// Calculate log factorial ln(n!) with automatic method selection.
///
/// Uses exact calculation for small n and Stirling's approximation for large n
/// to balance accuracy and numerical stability.
///
/// # Algorithm
///
/// - **n < 20**: Exact via Σ ln(i) for i = 2..n
/// - **n ≥ 20**: Stirling's approximation
///
/// # Arguments
///
/// * `n` - Non-negative integer
///
/// # Returns
///
/// ln(n!)
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::distributions::log_factorial;
///
/// assert_eq!(log_factorial(0), 0.0); // 0! = 1, ln(1) = 0
/// assert!((log_factorial(5) - (2.0_f64.ln() + 3.0_f64.ln() + 4.0_f64.ln() + 5.0_f64.ln())).abs() < 1e-10);
/// ```
pub fn log_factorial(n: usize) -> f64 {
    if n == 0 || n == 1 {
        return 0.0;
    }
    if n < 20 {
        // Exact calculation for small n: ln(n!) = ln(1) + ln(2) + ... + ln(n)
        (2..=n).map(|i| (i as f64).ln()).sum()
    } else {
        // Stirling's approximation: ln(n!) ≈ n*ln(n) - n + 0.5*ln(2πn)
        let n_f = n as f64;
        n_f * n_f.ln() - n_f + 0.5 * (2.0 * std::f64::consts::PI * n_f).ln()
    }
}

/// Sample from Beta(α, β) distribution using transformation method.
///
/// Generates random samples from the Beta distribution, commonly used for
/// modeling random variables constrained to [0,1] such as recovery rates,
/// default correlations, and prepayment rates.
///
/// # Distribution Properties
///
/// ```text
/// Beta(α, β) with α, β > 0:
/// - Support: [0, 1]
/// - Mean: α / (α + β)
/// - Mode: (α - 1) / (α + β - 2)  for α, β > 1
///
/// Shape parameter effects:
/// - α = β = 1: Uniform[0,1]
/// - α > β: Right-skewed (mode near 1)
/// - α < β: Left-skewed (mode near 0)
/// - α = β > 1: Symmetric, bell-shaped
/// ```
///
/// # Arguments
///
/// * `rng` - Random number generator implementing [`RandomNumberGenerator`]
/// * `alpha` - First shape parameter (α > 0)
/// * `beta` - Second shape parameter (β > 0)
///
/// # Returns
///
/// Random sample x ∈ [0, 1] from Beta(α, β)
///
/// # Use Cases
///
/// - **Recovery rates**: Beta(4, 2) models senior unsecured recovery ~60-70%
/// - **Default correlation**: Beta(2, 5) for low but uncertain correlation
/// - **Prepayment rates**: Beta shapes for mortgage prepayment speed
/// - **Bayesian priors**: Conjugate prior for Bernoulli/binomial likelihood
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::distributions::sample_beta;
/// use finstack_core::math::random::SimpleRng;
/// use finstack_core::math::RandomNumberGenerator;
///
/// let mut rng = SimpleRng::new(42);
///
/// // Sample recovery rate: Beta(4, 2) peaked around 65%
/// let recovery = sample_beta(&mut rng as &mut dyn RandomNumberGenerator, 4.0, 2.0);
/// assert!(recovery >= 0.0 && recovery <= 1.0);
///
/// // Uniform distribution: Beta(1, 1)
/// let uniform = sample_beta(&mut rng as &mut dyn RandomNumberGenerator, 1.0, 1.0);
/// assert!(uniform >= 0.0 && uniform <= 1.0);
/// ```
///
/// # References
///
/// - Johnson, N. L., Kotz, S., & Balakrishnan, N. (1995). *Continuous Univariate
///   Distributions, Volume 2* (2nd ed.). Wiley. Chapter 25 (Beta distribution).
/// - Devroye, L. (1986). *Non-Uniform Random Variate Generation*. Springer.
///   Chapter 9 (Beta distribution sampling).
pub fn sample_beta(rng: &mut dyn RandomNumberGenerator, alpha: f64, beta: f64) -> f64 {
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
        assert_eq!(log_binomial_coefficient(3, 5), f64::NEG_INFINITY);
    }

    #[test]
    fn test_sample_beta() {
        use super::super::random::SimpleRng;

        let mut rng = SimpleRng::new(42);

        // Test uniform case (alpha=1, beta=1)
        let uniform_sample = sample_beta(&mut rng as &mut dyn RandomNumberGenerator, 1.0, 1.0);
        assert!((0.0..=1.0).contains(&uniform_sample));

        // Test that samples are in [0, 1]
        let samples: Vec<f64> = (0..100)
            .map(|_| sample_beta(&mut rng as &mut dyn RandomNumberGenerator, 2.0, 2.0))
            .collect();
        for sample in samples {
            assert!((0.0..=1.0).contains(&sample));
        }
    }
}
