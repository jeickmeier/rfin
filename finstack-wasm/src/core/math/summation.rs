//! Numerically stable summation algorithms for WASM bindings.

use finstack_core::math::summation::{kahan_sum, neumaier_sum, NeumaierAccumulator};
use wasm_bindgen::prelude::*;

/// Kahan compensated summation for improved numerical stability.
///
/// Best for sequences where all values have the same sign. For mixed-sign values,
/// prefer `neumaierSum` which handles magnitude differences better.
///
/// @param {Float64Array} values - Array of values to sum
/// @returns {number} Sum with reduced floating-point error
///
/// @example
/// ```javascript
/// // Summing many small values where naive summation loses precision
/// const values = new Float64Array(1_000_000).fill(0.1);
/// const sum = kahanSum(values);  // ≈ 100000.0 (more accurate than simple sum)
/// ```
#[wasm_bindgen(js_name = kahanSum)]
pub fn kahan_sum_js(values: &[f64]) -> f64 {
    kahan_sum(values.iter().copied())
}

/// Neumaier compensated summation – handles both positive and negative values.
///
/// Improves upon Kahan summation by better handling cases where values have
/// similar magnitudes but opposite signs. **Recommended for financial calculations.**
///
/// @param {Float64Array} values - Array of values to sum
/// @returns {number} Sum with reduced floating-point error
///
/// @example
/// ```javascript
/// // Mixed-sign values where naive summation loses precision
/// const values = new Float64Array([1e16, 1.0, -1e16]);
/// const sum = neumaierSum(values);  // ≈ 1.0 (accurate despite large cancellation)
/// ```
#[wasm_bindgen(js_name = neumaierSum)]
pub fn neumaier_sum_js(values: &[f64]) -> f64 {
    neumaier_sum(values.iter().copied())
}

/// Incremental Neumaier compensated summation accumulator.
///
/// Useful for streaming summation without allocating a temporary array.
/// Handles both same-sign and mixed-sign values correctly.
///
/// @example
/// ```javascript
/// const acc = new SumAccumulator();
/// acc.add(1e16);
/// acc.add(1.0);
/// acc.add(-1e16);
/// const sum = acc.total();  // ≈ 1.0
/// ```
#[wasm_bindgen(js_name = SumAccumulator)]
pub struct JsNeumaierAccumulator {
    inner: NeumaierAccumulator,
}

#[wasm_bindgen(js_class = SumAccumulator)]
impl JsNeumaierAccumulator {
    /// Create a new accumulator with zero state.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsNeumaierAccumulator {
        JsNeumaierAccumulator {
            inner: NeumaierAccumulator::new(),
        }
    }

    /// Add a value to the running total.
    ///
    /// @param {number} x - Value to add
    pub fn add(&mut self, x: f64) {
        self.inner.add(x);
    }

    /// Add multiple values to the running total.
    ///
    /// @param {Float64Array} values - Array of values to add
    #[wasm_bindgen(js_name = addAll)]
    pub fn add_all(&mut self, values: &[f64]) {
        for &x in values {
            self.inner.add(x);
        }
    }

    /// Return the compensated total and reset the accumulator.
    ///
    /// @returns {number} The final sum
    pub fn total(&mut self) -> f64 {
        let result = self.inner.total();
        self.inner = NeumaierAccumulator::new();
        result
    }

    /// Return the current sum without consuming the accumulator.
    ///
    /// @returns {number} Current running sum
    pub fn current(&self) -> f64 {
        self.inner.current()
    }

    /// Reset the accumulator to zero.
    pub fn reset(&mut self) {
        self.inner = NeumaierAccumulator::new();
    }
}

impl Default for JsNeumaierAccumulator {
    fn default() -> Self {
        Self::new()
    }
}
