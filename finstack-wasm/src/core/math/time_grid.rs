use crate::core::error::{core_to_js, js_error};
use finstack_core::math::time_grid::TimeGrid as CoreTimeGrid;
use wasm_bindgen::prelude::*;

/// Time grid for Monte Carlo simulation.
///
/// Defines discretization points in year-fraction time from t=0 to t=T.
///
/// @example
/// ```javascript
/// // Uniform grid: 1 year with 252 trading days
/// const grid = TimeGrid.uniform(1.0, 252);
///
/// // Custom grid with irregular periods
/// const grid2 = TimeGrid.fromTimes([0.0, 0.25, 0.5, 0.75, 1.0]);
/// ```
#[wasm_bindgen(js_name = TimeGrid)]
#[derive(Clone, Debug)]
pub struct JsTimeGrid {
    inner: CoreTimeGrid,
}

impl JsTimeGrid {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &CoreTimeGrid {
        &self.inner
    }
}

#[wasm_bindgen(js_class = TimeGrid)]
impl JsTimeGrid {
    /// Create a uniform time grid from 0 to tMax with numSteps steps.
    ///
    /// @param {number} tMax - Final time in years (must be > 0)
    /// @param {number} numSteps - Number of time steps (must be > 0)
    /// @returns {TimeGrid} Uniform time grid
    #[wasm_bindgen(js_name = uniform)]
    pub fn uniform(t_max: f64, num_steps: usize) -> Result<JsTimeGrid, JsValue> {
        CoreTimeGrid::uniform(t_max, num_steps)
            .map(|inner| JsTimeGrid { inner })
            .map_err(core_to_js)
    }

    /// Create a uniform base grid and merge in required event times.
    ///
    /// @param {number} tMax - Horizon in years
    /// @param {number} stepsPerYear - Target density for uniform spacing
    /// @param {number} minSteps - Minimum number of uniform steps
    /// @param {Float64Array} requiredTimes - Extra knot times to merge in
    /// @returns {TimeGrid} Merged time grid
    #[wasm_bindgen(js_name = uniformWithRequiredTimes)]
    pub fn uniform_with_required_times(
        t_max: f64,
        steps_per_year: f64,
        min_steps: usize,
        required_times: Vec<f64>,
    ) -> Result<JsTimeGrid, JsValue> {
        CoreTimeGrid::uniform_with_required_times(t_max, steps_per_year, min_steps, &required_times)
            .map(|inner| JsTimeGrid { inner })
            .map_err(core_to_js)
    }

    /// Create a custom time grid from explicit time points.
    ///
    /// @param {Float64Array} times - Monotonically increasing time points starting at 0
    /// @returns {TimeGrid} Custom time grid
    #[wasm_bindgen(js_name = fromTimes)]
    pub fn from_times(times: Vec<f64>) -> Result<JsTimeGrid, JsValue> {
        CoreTimeGrid::from_times(times)
            .map(|inner| JsTimeGrid { inner })
            .map_err(core_to_js)
    }

    /// Number of time steps.
    #[wasm_bindgen(getter, js_name = numSteps)]
    pub fn num_steps(&self) -> usize {
        self.inner.num_steps()
    }

    /// Total time span in years.
    #[wasm_bindgen(getter, js_name = tMax)]
    pub fn t_max(&self) -> f64 {
        self.inner.t_max()
    }

    /// Get time at a specific step index.
    ///
    /// @param {number} step - Step index (0-based)
    /// @returns {number} Time in years at the given step
    #[wasm_bindgen(js_name = time)]
    pub fn time(&self, step: usize) -> Result<f64, JsValue> {
        if step > self.inner.num_steps() {
            return Err(js_error(format!(
                "Step index {step} out of bounds (max: {})",
                self.inner.num_steps()
            )));
        }
        Ok(self.inner.time(step))
    }

    /// Get time step size at step i (dt[i] = t[i+1] - t[i]).
    ///
    /// @param {number} step - Step index (0-based)
    /// @returns {number} Time step size in years
    #[wasm_bindgen(js_name = dt)]
    pub fn dt(&self, step: usize) -> Result<f64, JsValue> {
        if step >= self.inner.num_steps() {
            return Err(js_error(format!(
                "Step index {step} out of bounds (max: {})",
                self.inner.num_steps().saturating_sub(1)
            )));
        }
        Ok(self.inner.dt(step))
    }

    /// Get all time points as a Float64Array.
    #[wasm_bindgen(js_name = times)]
    pub fn times(&self) -> Vec<f64> {
        self.inner.times().to_vec()
    }

    /// Get all time steps as a Float64Array.
    #[wasm_bindgen(js_name = dts)]
    pub fn dts(&self) -> Vec<f64> {
        self.inner.dts().to_vec()
    }

    /// Check if grid is uniform (all dt equal within tolerance).
    #[wasm_bindgen(getter, js_name = isUniform)]
    pub fn is_uniform(&self) -> bool {
        self.inner.is_uniform()
    }

    /// String representation.
    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        format!(
            "TimeGrid(steps={}, tMax={}{})",
            self.inner.num_steps(),
            self.inner.t_max(),
            if self.inner.is_uniform() {
                ", uniform"
            } else {
                ""
            }
        )
    }
}
