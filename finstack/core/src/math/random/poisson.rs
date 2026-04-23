//! Poisson distribution sampling for jump processes.
//!
//! Provides methods to sample from Poisson distribution for modeling
//! jump arrivals in jump-diffusion processes.

/// Sample from Poisson distribution using inverse CDF method.
///
/// # Arguments
///
/// * `lambda` - Mean number of events (λ)
/// * `u` - Uniform random variable in [0, 1)
///
/// # Returns
///
/// Number of Poisson events
///
/// # Algorithm
///
/// Uses inverse CDF: finds smallest k such that P(N ≤ k) ≥ u
///
/// For small λ, uses direct summation.
/// For large λ, uses normal approximation.
pub fn poisson_inverse_cdf(lambda: f64, u: f64) -> usize {
    if lambda <= 0.0 {
        return 0;
    }

    // Threshold 30.0: normal approximation skewness = 1/√λ < 0.18
    if lambda < 30.0 {
        let mut p = (-lambda).exp(); // P(N = 0)
        let mut cdf = p;
        let mut k = 0;

        // Cap at 200 to prevent infinite loops for extreme u values
        while cdf < u && k < 200 {
            k += 1;
            p *= lambda / k as f64;
            cdf += p;
        }

        k
    } else {
        // For large lambda, use normal approximation
        // N ~ Poisson(λ) ≈ N(λ, λ) for large λ
        use crate::math::special_functions::standard_normal_inv_cdf;

        let std_dev = lambda.sqrt();
        let z = standard_normal_inv_cdf(u);
        let n_approx = lambda + std_dev * z;

        n_approx.round().max(0.0) as usize
    }
}

/// Sample from Poisson using standard normal input.
///
/// Converts a standard normal variate to Poisson via CDF transform.
///
/// # Arguments
///
/// * `lambda` - Mean number of events
/// * `z` - Standard normal variate
///
/// # Returns
///
/// Number of Poisson events
pub fn poisson_from_normal(lambda: f64, z: f64) -> usize {
    use crate::math::special_functions::norm_cdf;

    let u = norm_cdf(z);
    poisson_inverse_cdf(lambda, u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poisson_zero_lambda() {
        assert_eq!(poisson_inverse_cdf(0.0, 0.5), 0);
        assert_eq!(poisson_inverse_cdf(0.0, 0.9), 0);
    }

    #[test]
    fn test_poisson_small_lambda() {
        // For λ = 1, P(N=0) = e^{-1} ≈ 0.368
        let lambda = 1.0;

        // u < e^{-1} should give 0
        assert_eq!(poisson_inverse_cdf(lambda, 0.3), 0);

        // u > e^{-1} should give 1 or more
        assert!(poisson_inverse_cdf(lambda, 0.5) >= 1);
    }

    #[test]
    fn test_poisson_mean() {
        // Test that empirical mean approaches lambda
        let lambda = 3.0;
        let n_samples = 1000;

        let mut sum = 0;
        for i in 0..n_samples {
            let u = (i as f64 + 0.5) / n_samples as f64; // Uniform grid
            let k = poisson_inverse_cdf(lambda, u);
            sum += k;
        }

        let empirical_mean = sum as f64 / n_samples as f64;

        // Should be close to lambda
        assert!((empirical_mean - lambda).abs() / lambda < 0.2);
    }

    #[test]
    fn test_poisson_from_normal() {
        let lambda = 2.0;

        // z = 0 (median) should give around lambda
        let k = poisson_from_normal(lambda, 0.0);
        assert!(k <= 4); // Should be close to 2

        // Very negative z should give 0 or low value
        let k_low = poisson_from_normal(lambda, -3.0);
        assert!(k_low <= 2);

        // Very positive z should give higher value
        let k_high = poisson_from_normal(lambda, 3.0);
        assert!(k_high >= 2);
    }
}
