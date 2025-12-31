//! Random number generation for WASM bindings.

use finstack_core::math::random::{box_muller_transform, Pcg64Rng, RandomNumberGenerator};
use wasm_bindgen::prelude::*;

/// Production-grade random number generator backed by PCG64.
///
/// Provides excellent statistical properties suitable for Monte Carlo simulations:
/// - Period: 2^128
/// - Passes all TestU01 and PractRand statistical tests
/// - Deterministic: same seed always produces same sequence
///
/// @example
/// ```javascript
/// // Create RNG with seed for reproducibility
/// const rng = new Rng(42);
///
/// // Generate samples
/// const u = rng.uniform();     // U(0, 1)
/// const z = rng.normal(0, 1);  // N(0, 1)
/// const b = rng.bernoulli(0.05); // 5% probability
/// ```
#[wasm_bindgen(js_name = Rng)]
pub struct JsRng {
    inner: Pcg64Rng,
}

#[wasm_bindgen(js_class = Rng)]
impl JsRng {
    /// Create a new RNG with the given seed.
    ///
    /// @param {bigint} seed - 64-bit seed value
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> JsRng {
        JsRng {
            inner: Pcg64Rng::new(seed),
        }
    }

    /// Create a new RNG with seed and stream for parallel simulations.
    ///
    /// Each (seed, stream) pair produces a unique, independent sequence.
    ///
    /// @param {bigint} seed - Base seed
    /// @param {bigint} stream - Stream identifier
    #[wasm_bindgen(js_name = withStream)]
    pub fn with_stream(seed: u64, stream: u64) -> JsRng {
        JsRng {
            inner: Pcg64Rng::new_with_stream(seed, stream),
        }
    }

    /// Generate uniform random number in [0, 1).
    ///
    /// @returns {number} Uniform random value
    pub fn uniform(&mut self) -> f64 {
        self.inner.uniform()
    }

    /// Generate normal random number with specified mean and standard deviation.
    ///
    /// @param {number} mean - Mean of the distribution
    /// @param {number} stdDev - Standard deviation
    /// @returns {number} Normal random value
    pub fn normal(&mut self, mean: f64, std_dev: f64) -> f64 {
        self.inner.normal(mean, std_dev)
    }

    /// Generate Bernoulli random boolean with probability p.
    ///
    /// @param {number} p - Probability of true (0 to 1)
    /// @returns {boolean} Random boolean
    pub fn bernoulli(&mut self, p: f64) -> bool {
        self.inner.bernoulli(p)
    }

    /// Generate n uniform random numbers.
    ///
    /// @param {number} n - Number of samples
    /// @returns {Float64Array} Array of uniform random values
    #[wasm_bindgen(js_name = uniformArray)]
    pub fn uniform_array(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.inner.uniform()).collect()
    }

    /// Generate n normal random numbers.
    ///
    /// @param {number} n - Number of samples
    /// @param {number} mean - Mean of the distribution
    /// @param {number} stdDev - Standard deviation
    /// @returns {Float64Array} Array of normal random values
    #[wasm_bindgen(js_name = normalArray)]
    pub fn normal_array(&mut self, n: usize, mean: f64, std_dev: f64) -> Vec<f64> {
        (0..n).map(|_| self.inner.normal(mean, std_dev)).collect()
    }

    /// Clone the current RNG state.
    ///
    /// Useful for creating checkpoints in simulations.
    #[wasm_bindgen(js_name = clone)]
    pub fn clone_rng(&self) -> JsRng {
        JsRng {
            inner: self.inner.clone(),
        }
    }
}

/// Box-Muller transform for generating normal random variables.
///
/// Transforms two independent uniform random variables into two independent
/// standard normal random variables.
///
/// @param {number} u1 - First uniform random variable in (0, 1)
/// @param {number} u2 - Second uniform random variable in (0, 1)
/// @returns {Float64Array} Two independent N(0,1) random variables [z1, z2]
///
/// @example
/// ```javascript
/// const [z1, z2] = boxMullerTransform(0.5, 0.5);
/// // z1 and z2 are independent N(0,1) samples
/// ```
#[wasm_bindgen(js_name = boxMullerTransform)]
pub fn box_muller_transform_js(u1: f64, u2: f64) -> Vec<f64> {
    let (z1, z2) = box_muller_transform(u1, u2);
    vec![z1, z2]
}
