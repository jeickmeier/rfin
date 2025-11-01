//! Random number generation for Monte Carlo simulations.
//!
//! Provides trait-based interface for random number generators with deterministic
//! seed-based RNG for testing and basic simulations. For production Monte Carlo,
//! implement the trait with more sophisticated generators (PCG, Mersenne Twister).
//!
//! # Components
//!
//! - [`RandomNumberGenerator`]: Trait for pluggable RNG implementations
//! - [`SimpleRng`]: Linear congruential generator for testing
//! - [`box_muller_transform`], [`box_muller_polar`]: Normal variate generation
//!
//! # References
//!
//! - **Box-Muller Transform**:
//!   - Box, G. E. P., & Muller, M. E. (1958). "A Note on the Generation of Random
//!     Normal Deviates." *The Annals of Mathematical Statistics*, 29(2), 610-611.
//!
//! - **Polar Method**:
//!   - Marsaglia, G., & Bray, T. A. (1964). "A Convenient Method for Generating
//!     Normal Variables." *SIAM Review*, 6(3), 260-264.
//!
//! - **Linear Congruential Generators**:
//!   - Press, W. H., et al. (2007). *Numerical Recipes* (3rd ed.). Section 7.1.

/// Random number generator trait for statistical sampling.
///
/// This trait provides the basic interface needed for Monte Carlo simulations
/// and stochastic sampling algorithms.
pub trait RandomNumberGenerator {
    /// Generate uniform random number in [0, 1)
    fn uniform(&mut self) -> f64;

    /// Generate normal random number with specified mean and standard deviation
    fn normal(&mut self, mean: f64, std_dev: f64) -> f64;

    /// Generate Bernoulli random boolean with probability p
    fn bernoulli(&mut self, p: f64) -> bool;
}

/// Simple deterministic RNG for testing and basic simulations.
///
/// Uses a linear congruential generator for reproducible results.
/// For production Monte Carlo, use more sophisticated generators.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SimpleRng {
    state: u64,
    cached_normal: Option<f64>, // Instance-based cache for Box-Muller
}

impl SimpleRng {
    /// Create a new RNG with the given seed
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1), // Avoid zero state
            cached_normal: None,
        }
    }

    /// Generate next random bits using LCG
    fn next_u64(&mut self) -> u64 {
        // Simple LCG parameters (from Numerical Recipes)
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
}

impl RandomNumberGenerator for SimpleRng {
    fn uniform(&mut self) -> f64 {
        // Convert to [0, 1) using upper bits for better quality
        let bits = self.next_u64() >> 11; // Use upper 53 bits for double precision
        (bits as f64) / (1u64 << 53) as f64
    }

    fn normal(&mut self, mean: f64, std_dev: f64) -> f64 {
        // Box-Muller transform with instance-based cache
        if let Some(cached) = self.cached_normal.take() {
            return mean + std_dev * cached;
        }

        let u1 = self.uniform();
        let u2 = self.uniform();

        let (z0, z1) = box_muller_transform(u1, u2);
        self.cached_normal = Some(z1);
        mean + std_dev * z0
    }

    fn bernoulli(&mut self, p: f64) -> bool {
        self.uniform() < p
    }
}

/// Box-Muller transform for generating normal random variables.
///
/// Transforms two independent uniform random variables into two independent
/// standard normal random variables using the Box-Muller method. This is the
/// classic algorithm for generating Gaussian samples in Monte Carlo simulations.
///
/// # Arguments
///
/// * `u1` - First uniform random variable in (0, 1)
/// * `u2` - Second uniform random variable in (0, 1)
///
/// # Returns
///
/// Tuple of two independent N(0,1) random variables (z₁, z₂).
///
/// # Algorithm
///
/// ```text
/// z₁ = √(-2 ln u₁) cos(2π u₂)
/// z₂ = √(-2 ln u₁) sin(2π u₂)
/// ```
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::random::box_muller_transform;
///
/// let (z1, z2) = box_muller_transform(0.5, 0.5);
/// // z1 and z2 are independent N(0,1) samples
/// ```
///
/// # References
///
/// - Box, G. E. P., & Muller, M. E. (1958). "A Note on the Generation of Random
///   Normal Deviates." *The Annals of Mathematical Statistics*, 29(2), 610-611.
#[inline]
pub fn box_muller_transform(u1: f64, u2: f64) -> (f64, f64) {
    use std::f64::consts::PI;
    // Clamp u1 away from 0 and 1 to prevent -inf or inf in log
    // This prevents NaN/inf propagation in Monte Carlo paths
    const EPS: f64 = 1e-300;
    let u1_safe = u1.clamp(EPS, 1.0 - EPS);
    let r = (-2.0 * u1_safe.ln()).sqrt();
    let theta = 2.0 * PI * u2;
    let z1 = r * theta.cos();
    let z2 = r * theta.sin();
    (z1, z2)
}

/// Polar form of Box-Muller (rejection-based, typically faster).
///
/// # Arguments
///
/// * `gen_u01` - Function that generates U(0,1) random variables
///
/// # Returns
///
/// Tuple of two independent N(0,1) random variables.
pub fn box_muller_polar<F>(mut gen_u01: F) -> (f64, f64)
where
    F: FnMut() -> f64,
{
    loop {
        let u1 = 2.0 * gen_u01() - 1.0;
        let u2 = 2.0 * gen_u01() - 1.0;
        let s = u1 * u1 + u2 * u2;

        if s > 0.0 && s < 1.0 {
            let factor = (-2.0 * s.ln() / s).sqrt();
            return (u1 * factor, u2 * factor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_rng_uniform() {
        let mut rng = SimpleRng::new(42);

        // Test that values are in [0, 1)
        for _ in 0..100 {
            let val = rng.uniform();
            assert!((0.0..1.0).contains(&val));
        }
    }

    #[test]
    fn test_simple_rng_deterministic() {
        let mut rng1 = SimpleRng::new(42);
        let mut rng2 = SimpleRng::new(42);

        // Same seed should produce same sequence
        for _ in 0..10 {
            assert_eq!(rng1.uniform(), rng2.uniform());
        }
    }

    #[test]
    fn test_simple_rng_normal() {
        let mut rng = SimpleRng::new(42);

        // Test basic properties
        let sample = rng.normal(0.0, 1.0);
        assert!(sample.is_finite());

        // Test mean and std_dev parameters
        let sample_shifted = rng.normal(5.0, 2.0);
        assert!(sample_shifted.is_finite());
    }

    #[test]
    fn test_simple_rng_bernoulli() {
        let mut rng = SimpleRng::new(42);

        // Test extreme probabilities
        assert!(!rng.bernoulli(0.0));
        assert!(rng.bernoulli(1.0));

        // Test intermediate probability
        let mut successes = 0;
        for _ in 0..1000 {
            if rng.bernoulli(0.3) {
                successes += 1;
            }
        }

        // Should be roughly 30% successes (allow wide tolerance for small sample)
        assert!(successes > 200 && successes < 400);
    }

    #[test]
    fn test_box_muller_transform() {
        let (z1, z2) = box_muller_transform(0.5, 0.5);
        assert!(z1.is_finite());
        assert!(z2.is_finite());

        // Test with many samples
        let mut rng = SimpleRng::new(42);
        let mut samples = Vec::new();
        for _ in 0..500 {
            let u1 = rng.uniform();
            let u2 = rng.uniform();
            let (z1, z2) = box_muller_transform(u1, u2);
            samples.push(z1);
            samples.push(z2);
        }

        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let var =
            samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (samples.len() - 1) as f64;

        assert!(mean.abs() < 0.1);
        assert!((var - 1.0).abs() < 0.2);
    }

    #[test]
    fn test_box_muller_polar() {
        let mut rng = SimpleRng::new(42);
        let gen_u01 = || rng.uniform();

        let (z1, z2) = box_muller_polar(gen_u01);
        assert!(z1.is_finite());
        assert!(z2.is_finite());
    }
}
