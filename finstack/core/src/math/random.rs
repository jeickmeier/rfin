//! Random number generation trait and basic implementations.
//!
//! This module provides a clean interface for random number generation
//! used in Monte Carlo simulations and stochastic algorithms.
//!
//! For production use with advanced generators (PCG, etc.), implement
//! the RandomNumberGenerator trait in the consuming crates.

use crate::F;

/// Random number generator trait for statistical sampling.
///
/// This trait provides the basic interface needed for Monte Carlo simulations
/// and stochastic sampling algorithms.
pub trait RandomNumberGenerator {
    /// Generate uniform random number in [0, 1)
    fn uniform(&mut self) -> F;
    
    /// Generate normal random number with specified mean and standard deviation
    fn normal(&mut self, mean: F, std_dev: F) -> F;
    
    /// Generate Bernoulli random boolean with probability p
    fn bernoulli(&mut self, p: F) -> bool;
}

/// Simple deterministic RNG for testing and basic simulations.
///
/// Uses a linear congruential generator for reproducible results.
/// For production Monte Carlo, use more sophisticated generators.
#[derive(Clone, Debug)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    /// Create a new RNG with the given seed
    pub fn new(seed: u64) -> Self {
        Self { 
            state: seed.wrapping_add(1) // Avoid zero state
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
    fn uniform(&mut self) -> F {
        // Convert to [0, 1) using upper bits for better quality
        let bits = self.next_u64() >> 11; // Use upper 53 bits for double precision
        (bits as F) / (1u64 << 53) as F
    }

    fn normal(&mut self, mean: F, std_dev: F) -> F {
        // Box-Muller transform
        static mut CACHED: Option<F> = None;
        static mut HAS_CACHED: bool = false;

        unsafe {
            if HAS_CACHED {
                HAS_CACHED = false;
                return mean + std_dev * CACHED.unwrap();
            }
        }

        let u1 = self.uniform();
        let u2 = self.uniform();

        let mag = std_dev * (-2.0 * u1.ln()).sqrt();
        let z0 = mag * (2.0 * std::f64::consts::PI * u2).cos();
        let z1 = mag * (2.0 * std::f64::consts::PI * u2).sin();

        unsafe {
            CACHED = Some(z1);
            HAS_CACHED = true;
        }

        mean + z0
    }

    fn bernoulli(&mut self, p: F) -> bool {
        self.uniform() < p
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
}
