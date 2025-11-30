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
//! use finstack_core::math::random::TestRng;
//! use finstack_core::math::RandomNumberGenerator;
//!
//! let mut rng = TestRng::new(42);
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

/// Generate the complete binomial distribution P(X=k) for k = 0, 1, ..., n.
///
/// Returns a normalized probability vector where `dist[k]` = P(X = k).
/// Uses log-space arithmetic to prevent overflow for large n.
///
/// # Mathematical Definition
///
/// ```text
/// dist[k] = P(X = k) = C(n,k) * p^k * (1-p)^(n-k)
/// ```
///
/// # Arguments
///
/// * `n` - Number of independent trials (≥ 0)
/// * `p` - Probability of success on each trial (0 ≤ p ≤ 1)
///
/// # Returns
///
/// Vector of probabilities `[P(X=0), P(X=1), ..., P(X=n)]` with length n+1.
/// The vector sums to 1.0 (normalized).
///
/// # Use Cases
///
/// - **Credit modeling**: Loss distribution for homogeneous pool of n obligors
/// - **Portfolio analytics**: Number of defaults given conditional default probability
/// - **Structured credit**: Default distribution for CDO/CLO tranches
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::distributions::binomial_distribution;
///
/// // Fair coin: distribution of heads in 10 flips
/// let dist = binomial_distribution(10, 0.5);
/// assert_eq!(dist.len(), 11); // P(X=0), P(X=1), ..., P(X=10)
/// assert!((dist[5] - 0.24609375).abs() < 1e-6); // P(X=5)
///
/// // Credit portfolio: default distribution with 5% PD
/// let loss_dist = binomial_distribution(100, 0.05);
/// assert_eq!(loss_dist.len(), 101);
/// // Most probability mass around 5 defaults
/// assert!(loss_dist[5] > loss_dist[0]);
/// assert!(loss_dist[5] > loss_dist[20]);
/// ```
///
/// # References
///
/// - Johnson, N. L., Kotz, S., & Kemp, A. W. (1993). *Univariate Discrete Distributions*
///   (2nd ed.). Wiley. Chapter 3.
pub fn binomial_distribution(n: usize, p: f64) -> Vec<f64> {
    let mut dist = Vec::with_capacity(n + 1);
    for k in 0..=n {
        dist.push(binomial_probability(n, k, p));
    }
    // Normalize (should already sum to ~1, but defensive for numerical edge cases)
    let sum: f64 = dist.iter().sum();
    if sum > 0.0 && (sum - 1.0).abs() > 1e-10 {
        for prob in &mut dist {
            *prob /= sum;
        }
    }
    dist
}

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

/// Sample from Beta(α, β) distribution using the gamma ratio method.
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
/// # Algorithm
///
/// Uses the gamma ratio method (Devroye, 1986):
/// If X ~ Gamma(α, 1) and Y ~ Gamma(β, 1), then X/(X+Y) ~ Beta(α, β)
///
/// Gamma samples are generated using Marsaglia & Tsang's method for shape ≥ 1,
/// with Ahrens-Dieter transformation for shape < 1.
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
/// use finstack_core::math::random::TestRng;
/// use finstack_core::math::RandomNumberGenerator;
///
/// let mut rng = TestRng::new(42);
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
///   Chapter 9 (Beta distribution sampling via gamma ratio).
/// - Marsaglia, G., & Tsang, W. W. (2000). "A Simple Method for Generating Gamma
///   Variables." *ACM Transactions on Mathematical Software*, 26(3), 363-372.
pub fn sample_beta(rng: &mut dyn RandomNumberGenerator, alpha: f64, beta: f64) -> f64 {
    // Special case: Beta(1, 1) = Uniform[0, 1]
    if alpha == 1.0 && beta == 1.0 {
        return rng.uniform();
    }

    // Use gamma ratio method: X/(X+Y) ~ Beta(α, β) where X ~ Gamma(α), Y ~ Gamma(β)
    let x = sample_gamma(rng, alpha);
    let y = sample_gamma(rng, beta);

    // Guard against division by zero (extremely rare, but defensive)
    if x + y == 0.0 {
        return 0.5; // Fallback to mean for degenerate case
    }
    x / (x + y)
}

/// Sample from Gamma(shape, 1) distribution using Marsaglia-Tsang method.
///
/// This is an internal helper for Beta sampling. Uses rejection sampling with
/// Marsaglia & Tsang's "squeeze" method for shape ≥ 1, and Ahrens-Dieter
/// transformation for shape < 1.
///
/// # Arguments
///
/// * `rng` - Random number generator
/// * `shape` - Shape parameter (α > 0)
///
/// # Returns
///
/// Random sample from Gamma(shape, 1)
///
/// # References
///
/// - Marsaglia, G., & Tsang, W. W. (2000). "A Simple Method for Generating Gamma
///   Variables." *ACM Transactions on Mathematical Software*, 26(3), 363-372.
fn sample_gamma(rng: &mut dyn RandomNumberGenerator, shape: f64) -> f64 {
    if shape < 1.0 {
        // Ahrens-Dieter transformation for shape < 1:
        // If X ~ Gamma(shape + 1), then X * U^(1/shape) ~ Gamma(shape)
        let u = rng.uniform();
        // Clamp u away from 0 to prevent ln(0) issues
        let u_safe = u.max(1e-300);
        return sample_gamma(rng, shape + 1.0) * u_safe.powf(1.0 / shape);
    }

    // Marsaglia-Tsang method for shape >= 1
    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();

    loop {
        // Generate normal variate using Box-Muller
        let x = rng.normal(0.0, 1.0);
        let v = 1.0 + c * x;

        if v > 0.0 {
            let v = v * v * v; // v^3
            let u = rng.uniform();
            let x2 = x * x;

            // Squeeze test (fast accept)
            if u < 1.0 - 0.0331 * x2 * x2 {
                return d * v;
            }

            // Full rejection test
            // Clamp u and v away from 0 to prevent ln(0)
            let u_safe = u.max(1e-300);
            let v_safe = v.max(1e-300);
            if u_safe.ln() < 0.5 * x2 + d * (1.0 - v_safe + v_safe.ln()) {
                return d * v;
            }
        }
        // Reject and retry
    }
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
        use super::super::random::TestRng;

        let mut rng = TestRng::new(42);

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

    #[test]
    fn test_sample_beta_statistics() {
        use super::super::random::TestRng;

        let mut rng = TestRng::new(12345);
        let n_samples = 10_000;

        // Test Beta(4, 2) - expected mean = 4/(4+2) = 0.6667
        let alpha: f64 = 4.0;
        let beta_param: f64 = 2.0;
        let expected_mean = alpha / (alpha + beta_param);
        let expected_var =
            (alpha * beta_param) / ((alpha + beta_param).powi(2) * (alpha + beta_param + 1.0));

        let samples: Vec<f64> = (0..n_samples)
            .map(|_| {
                sample_beta(
                    &mut rng as &mut dyn RandomNumberGenerator,
                    alpha,
                    beta_param,
                )
            })
            .collect();

        let sample_mean = samples.iter().sum::<f64>() / n_samples as f64;
        let sample_var = samples
            .iter()
            .map(|x| (x - sample_mean).powi(2))
            .sum::<f64>()
            / (n_samples - 1) as f64;

        // Allow 5% relative error for mean (statistical tolerance)
        assert!(
            (sample_mean - expected_mean).abs() < 0.05 * expected_mean,
            "Beta(4,2) mean: expected {:.4}, got {:.4}",
            expected_mean,
            sample_mean
        );

        // Allow 20% relative error for variance (higher tolerance due to sampling variance)
        assert!(
            (sample_var - expected_var).abs() < 0.20 * expected_var,
            "Beta(4,2) variance: expected {:.4}, got {:.4}",
            expected_var,
            sample_var
        );
    }

    #[test]
    fn test_sample_beta_small_shape() {
        use super::super::random::TestRng;

        // Test with shape parameters < 1 (uses Ahrens-Dieter transformation)
        let mut rng = TestRng::new(9999);
        let samples: Vec<f64> = (0..1000)
            .map(|_| sample_beta(&mut rng as &mut dyn RandomNumberGenerator, 0.5, 0.5))
            .collect();

        // All samples should be in [0, 1]
        for sample in &samples {
            assert!(
                (0.0..=1.0).contains(sample),
                "Beta(0.5, 0.5) sample {} out of bounds",
                sample
            );
        }

        // Beta(0.5, 0.5) is the arcsine distribution with mean = 0.5
        let sample_mean = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(
            (sample_mean - 0.5).abs() < 0.1,
            "Beta(0.5, 0.5) mean: expected ~0.5, got {:.4}",
            sample_mean
        );
    }

    #[test]
    fn test_binomial_distribution() {
        // Test basic distribution
        let dist = binomial_distribution(10, 0.5);
        assert_eq!(dist.len(), 11);

        // Test P(X=5) for fair coin
        assert!(
            (dist[5] - 0.24609375).abs() < 1e-6,
            "P(X=5) = {}, expected 0.24609375",
            dist[5]
        );

        // Test normalization
        let sum: f64 = dist.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-10,
            "Distribution sum = {}, expected 1.0",
            sum
        );

        // Test symmetry for p=0.5
        for k in 0..=5 {
            assert!(
                (dist[k] - dist[10 - k]).abs() < 1e-10,
                "P({}) = {} should equal P({}) = {}",
                k,
                dist[k],
                10 - k,
                dist[10 - k]
            );
        }
    }

    #[test]
    fn test_binomial_distribution_edge_cases() {
        // p = 0: all probability on k=0
        let dist_zero = binomial_distribution(5, 0.0);
        assert!((dist_zero[0] - 1.0).abs() < 1e-10);
        for val in dist_zero.iter().skip(1) {
            assert!(*val < 1e-10);
        }

        // p = 1: all probability on k=n
        let dist_one = binomial_distribution(5, 1.0);
        assert!((dist_one[5] - 1.0).abs() < 1e-10);
        for val in dist_one.iter().take(5) {
            assert!(*val < 1e-10);
        }

        // n = 0: single element
        let dist_n0 = binomial_distribution(0, 0.5);
        assert_eq!(dist_n0.len(), 1);
        assert!((dist_n0[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_binomial_distribution_credit_portfolio() {
        // Typical credit portfolio: 100 names with 5% PD
        let dist = binomial_distribution(100, 0.05);
        assert_eq!(dist.len(), 101);

        // Expected number of defaults = n * p = 5
        // Most probability mass should be around 5
        let expected_mean: f64 = (0..=100).map(|k| k as f64 * dist[k]).sum();
        assert!(
            (expected_mean - 5.0).abs() < 0.01,
            "Mean = {}, expected ~5.0",
            expected_mean
        );

        // Variance = n * p * (1-p) = 4.75
        let expected_var: f64 = (0..=100)
            .map(|k| (k as f64 - expected_mean).powi(2) * dist[k])
            .sum();
        assert!(
            (expected_var - 4.75).abs() < 0.01,
            "Variance = {}, expected ~4.75",
            expected_var
        );
    }
}
