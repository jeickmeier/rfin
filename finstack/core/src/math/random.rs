//! Random number generation for Monte Carlo simulations.
//!
//! Provides trait-based interface for random number generators with deterministic
//! seed-based RNG for testing and basic simulations.
//!
//! # Components
//!
//! - [`RandomNumberGenerator`]: Trait for pluggable RNG implementations
//! - [`TestRng`]: Linear congruential generator for **testing only** (NOT for production)
//! - [`box_muller_transform`], [`box_muller_polar`]: Normal variate generation
//!
//! # Production Use
//!
//! **WARNING**: [`TestRng`] uses a simple LCG algorithm that is **NOT suitable for
//! production Monte Carlo simulations**. For production use:
//!
//! 1. Implement [`RandomNumberGenerator`] with a cryptographically secure or
//!    statistically robust RNG from a crate like `rand`:
//!    - **PCG64**: Fast, statistically excellent (recommended for simulations)
//!    - **ChaCha8/12/20**: Cryptographically secure when needed
//!    - **Xoshiro256++**: Fast, good statistical properties
//!
//! 2. Example production implementation:
//!
//! ```ignore
//! use rand::prelude::*;
//! use rand_pcg::Pcg64;
//!
//! struct ProductionRng(Pcg64);
//!
//! impl ProductionRng {
//!     fn new(seed: u64) -> Self {
//!         Self(Pcg64::seed_from_u64(seed))
//!     }
//! }
//!
//! impl RandomNumberGenerator for ProductionRng {
//!     fn uniform(&mut self) -> f64 { self.0.gen() }
//!     fn normal(&mut self, mean: f64, std_dev: f64) -> f64 {
//!         use rand_distr::{Distribution, Normal};
//!         Normal::new(mean, std_dev).unwrap().sample(&mut self.0)
//!     }
//!     fn bernoulli(&mut self, p: f64) -> bool { self.0.gen::<f64>() < p }
//! }
//! ```
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
//! - **Production RNGs**:
//!   - O'Neill, M. E. (2014). "PCG: A Family of Simple Fast Space-Efficient
//!     Statistically Good Algorithms for Random Number Generation."
//!   - L'Ecuyer, P. (2017). "Random Number Generation." In *Handbook of
//!     Computational Statistics* (2nd ed.). Springer.

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

/// Deterministic RNG for **testing only** — NOT for production Monte Carlo.
///
/// Uses a simple linear congruential generator (LCG) that provides:
/// - **Deterministic**: Same seed → same sequence (reproducible tests)
/// - **Fast**: Minimal overhead for unit tests
/// - **Portable**: No external dependencies
///
/// # ⚠️ WARNING: Not for Production Use
///
/// This LCG has **poor statistical properties** and is unsuitable for:
/// - Monte Carlo simulations (VaR, CVA, option pricing)
/// - Any risk-sensitive computation
/// - Large-scale sampling (>10⁶ samples)
///
/// **Issues with LCG for production**:
/// - Short period (2³² in 32-bit variant)
/// - Correlated low-order bits
/// - Fails many statistical tests (TestU01, PractRand)
///
/// For production, implement [`RandomNumberGenerator`] with PCG64, ChaCha, or
/// Xoshiro256++. See module documentation for example.
///
/// # Examples
///
/// ```rust
/// use finstack_core::math::random::{TestRng, RandomNumberGenerator};
///
/// // For unit tests only
/// let mut rng = TestRng::new(42);
/// let u = rng.uniform();
/// assert!((0.0..1.0).contains(&u));
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TestRng {
    state: u64,
    cached_normal: Option<f64>, // Instance-based cache for Box-Muller
}

/// Type alias for backwards compatibility (deprecated).
#[deprecated(
    since = "0.2.0",
    note = "Use TestRng instead; SimpleRng implies production-readiness"
)]
pub type SimpleRng = TestRng;

impl TestRng {
    /// Create a new RNG with the given seed.
    ///
    /// The same seed will always produce the same sequence of random numbers,
    /// making tests deterministic and reproducible.
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1), // Avoid zero state
            cached_normal: None,
        }
    }

    /// Generate next random bits using LCG.
    fn next_u64(&mut self) -> u64 {
        // Simple LCG parameters (from Numerical Recipes)
        // Note: These parameters are chosen for speed, not quality
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }
}

impl RandomNumberGenerator for TestRng {
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
/// # Numerical Details
///
/// Uses `f64::MIN_POSITIVE` (~2.2e-308) for clamping to allow extreme tail
/// values up to ±37σ while preventing -∞ from ln(0). The clamp at 1-ε
/// prevents numerical issues when u1 is very close to 1.
///
/// Achievable range: |z| ≤ √(-2 ln(2.2e-308)) ≈ 37.7
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

    // Use smallest positive f64 to allow extreme tail sampling (up to ~37σ)
    // while preventing -inf from ln(0). The 1.0 - EPS upper bound prevents
    // numerical issues when u1 rounds to exactly 1.0.
    //
    // With EPS = MIN_POSITIVE (~2.2e-308):
    //   -2 * ln(2.2e-308) ≈ 1418, so sqrt ≈ 37.7
    //   This allows sampling ~37σ events (probability ~1e-300)
    const EPS: f64 = f64::MIN_POSITIVE;
    let u1_safe = u1.clamp(EPS, 1.0 - f64::EPSILON);

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
    fn test_test_rng_uniform() {
        let mut rng = TestRng::new(42);

        // Test that values are in [0, 1)
        for _ in 0..100 {
            let val = rng.uniform();
            assert!((0.0..1.0).contains(&val));
        }
    }

    #[test]
    fn test_test_rng_deterministic() {
        let mut rng1 = TestRng::new(42);
        let mut rng2 = TestRng::new(42);

        // Same seed should produce same sequence
        for _ in 0..10 {
            assert_eq!(rng1.uniform(), rng2.uniform());
        }
    }

    #[test]
    fn test_test_rng_normal() {
        let mut rng = TestRng::new(42);

        // Test basic properties
        let sample = rng.normal(0.0, 1.0);
        assert!(sample.is_finite());

        // Test mean and std_dev parameters
        let sample_shifted = rng.normal(5.0, 2.0);
        assert!(sample_shifted.is_finite());
    }

    #[test]
    fn test_test_rng_bernoulli() {
        let mut rng = TestRng::new(42);

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
        let mut rng = TestRng::new(42);
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
        let mut rng = TestRng::new(42);
        let gen_u01 = || rng.uniform();

        let (z1, z2) = box_muller_polar(gen_u01);
        assert!(z1.is_finite());
        assert!(z2.is_finite());
    }
}
