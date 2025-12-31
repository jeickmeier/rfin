//! Joint probability utilities for WASM bindings.

use finstack_core::math::probability::{correlation_bounds, joint_probabilities, CorrelatedBernoulli};
use wasm_bindgen::prelude::*;

/// Compute joint probabilities for two correlated Bernoulli random variables.
///
/// Given marginal probabilities p1 and p2 with correlation ρ, returns
/// the four joint probabilities [p11, p10, p01, p00] where:
/// - p11 = P(X₁=1, X₂=1)
/// - p10 = P(X₁=1, X₂=0)
/// - p01 = P(X₁=0, X₂=1)
/// - p00 = P(X₁=0, X₂=0)
///
/// The correlation is automatically clamped to feasible Fréchet-Hoeffding bounds.
///
/// @param {number} p1 - Marginal probability P(X₁=1)
/// @param {number} p2 - Marginal probability P(X₂=1)
/// @param {number} correlation - Correlation between X₁ and X₂
/// @returns {Float64Array} [p11, p10, p01, p00]
///
/// @example
/// ```javascript
/// const probs = jointProbabilities(0.6, 0.4, 0.3);
/// // probs[0] + probs[1] + probs[2] + probs[3] = 1.0
/// // probs[0] + probs[1] = 0.6 (marginal p1)
/// // probs[0] + probs[2] = 0.4 (marginal p2)
/// ```
#[wasm_bindgen(js_name = jointProbabilities)]
pub fn joint_probabilities_js(p1: f64, p2: f64, correlation: f64) -> Vec<f64> {
    let (p11, p10, p01, p00) = joint_probabilities(p1, p2, correlation);
    vec![p11, p10, p01, p00]
}

/// Compute the achievable correlation bounds for given marginal probabilities.
///
/// Returns the Fréchet-Hoeffding bounds [ρ_min, ρ_max] that constrain feasible correlations.
///
/// @param {number} p1 - Marginal probability P(X₁=1)
/// @param {number} p2 - Marginal probability P(X₂=1)
/// @returns {Float64Array} [ρ_min, ρ_max]
///
/// @example
/// ```javascript
/// const [rhoMin, rhoMax] = correlationBounds(0.5, 0.5);
/// // For equal probabilities: rhoMin ≈ -1, rhoMax ≈ 1
/// ```
#[wasm_bindgen(js_name = correlationBounds)]
pub fn correlation_bounds_js(p1: f64, p2: f64) -> Vec<f64> {
    let (rho_min, rho_max) = correlation_bounds(p1, p2);
    vec![rho_min, rho_max]
}

/// Correlated Bernoulli distribution for scenario generation.
///
/// Provides methods for working with correlated binary outcomes,
/// useful for tree-based pricing and analytical calculations.
///
/// @example
/// ```javascript
/// const dist = new CorrelatedBernoulliDist(0.5, 0.5, 0.5);
///
/// // Sample outcomes
/// const [x1, x2] = dist.sampleFromUniform(0.3);
///
/// // Access probabilities
/// const p11 = dist.jointP11();  // P(both happen)
/// const condP = dist.conditionalP2GivenX1();  // P(X2=1 | X1=1)
/// ```
#[wasm_bindgen(js_name = CorrelatedBernoulliDist)]
pub struct JsCorrelatedBernoulli {
    inner: CorrelatedBernoulli,
}

#[wasm_bindgen(js_class = CorrelatedBernoulliDist)]
impl JsCorrelatedBernoulli {
    /// Create a correlated Bernoulli distribution.
    ///
    /// @param {number} p1 - Marginal probability of first event
    /// @param {number} p2 - Marginal probability of second event
    /// @param {number} correlation - Correlation between events
    #[wasm_bindgen(constructor)]
    pub fn new(p1: f64, p2: f64, correlation: f64) -> JsCorrelatedBernoulli {
        JsCorrelatedBernoulli {
            inner: CorrelatedBernoulli::new(p1, p2, correlation),
        }
    }

    /// Sample a pair of correlated binary outcomes.
    ///
    /// @param {number} u - Uniform random value in [0, 1]
    /// @returns {Uint8Array} [x1, x2] where each is 0 or 1
    #[wasm_bindgen(js_name = sampleFromUniform)]
    pub fn sample_from_uniform(&self, u: f64) -> Vec<u8> {
        let (x1, x2) = self.inner.sample_from_uniform(u);
        vec![x1, x2]
    }

    /// Get the marginal probability of event 1.
    #[wasm_bindgen(getter)]
    pub fn p1(&self) -> f64 {
        self.inner.p1()
    }

    /// Get the marginal probability of event 2.
    #[wasm_bindgen(getter)]
    pub fn p2(&self) -> f64 {
        self.inner.p2()
    }

    /// Get the correlation.
    #[wasm_bindgen(getter)]
    pub fn correlation(&self) -> f64 {
        self.inner.correlation()
    }

    /// Get P(X₁=1, X₂=1).
    #[wasm_bindgen(js_name = jointP11)]
    pub fn joint_p11(&self) -> f64 {
        self.inner.joint_p11()
    }

    /// Get P(X₁=1, X₂=0).
    #[wasm_bindgen(js_name = jointP10)]
    pub fn joint_p10(&self) -> f64 {
        self.inner.joint_p10()
    }

    /// Get P(X₁=0, X₂=1).
    #[wasm_bindgen(js_name = jointP01)]
    pub fn joint_p01(&self) -> f64 {
        self.inner.joint_p01()
    }

    /// Get P(X₁=0, X₂=0).
    #[wasm_bindgen(js_name = jointP00)]
    pub fn joint_p00(&self) -> f64 {
        self.inner.joint_p00()
    }

    /// Get all four joint probabilities.
    ///
    /// @returns {Float64Array} [p11, p10, p01, p00]
    #[wasm_bindgen(js_name = jointProbabilities)]
    pub fn joint_probabilities(&self) -> Vec<f64> {
        let (p11, p10, p01, p00) = self.inner.joint_probabilities();
        vec![p11, p10, p01, p00]
    }

    /// Get P(X₂=1 | X₁=1).
    #[wasm_bindgen(js_name = conditionalP2GivenX1)]
    pub fn conditional_p2_given_x1(&self) -> f64 {
        self.inner.conditional_p2_given_x1()
    }

    /// Get P(X₁=1 | X₂=1).
    #[wasm_bindgen(js_name = conditionalP1GivenX2)]
    pub fn conditional_p1_given_x2(&self) -> f64 {
        self.inner.conditional_p1_given_x2()
    }
}
