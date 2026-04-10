//! Factor-model sensitivity engine types for WASM.
//!
//! Wraps `finstack_valuations::factor_model::sensitivity` types: the sensitivity
//! matrix and scenario grid used by delta-based and full-repricing engines.

use finstack_valuations::factor_model::sensitivity::{
    FactorPnlProfile, ScenarioGrid, SensitivityMatrix,
};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// SensitivityMatrix
// ---------------------------------------------------------------------------

/// Positions x factors sensitivity matrix stored in row-major order.
///
/// Each row corresponds to a portfolio position and each column to a risk
/// factor. Values represent the per-unit P&L sensitivity to a one-unit
/// factor move (in the configured bump units).
#[wasm_bindgen(js_name = SensitivityMatrix)]
#[derive(Clone)]
pub struct JsSensitivityMatrix {
    inner: SensitivityMatrix,
}

impl JsSensitivityMatrix {
    pub(crate) fn from_inner(inner: SensitivityMatrix) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> &SensitivityMatrix {
        &self.inner
    }
}

#[wasm_bindgen(js_class = SensitivityMatrix)]
impl JsSensitivityMatrix {
    /// Number of positions (rows).
    #[wasm_bindgen(getter, js_name = nPositions)]
    pub fn n_positions(&self) -> usize {
        self.inner.n_positions()
    }

    /// Number of factors (columns).
    #[wasm_bindgen(getter, js_name = nFactors)]
    pub fn n_factors(&self) -> usize {
        self.inner.n_factors()
    }

    /// Ordered position identifiers.
    #[wasm_bindgen(getter, js_name = positionIds)]
    pub fn position_ids(&self) -> Vec<String> {
        self.inner.position_ids().to_vec()
    }

    /// Ordered factor identifiers.
    #[wasm_bindgen(getter, js_name = factorIds)]
    pub fn factor_ids(&self) -> Vec<String> {
        self.inner
            .factor_ids()
            .iter()
            .map(|id| id.as_str().to_string())
            .collect()
    }

    /// Read a single sensitivity element.
    pub fn delta(&self, position_idx: usize, factor_idx: usize) -> f64 {
        self.inner.delta(position_idx, factor_idx)
    }

    /// Get the sensitivity row for a position (one value per factor).
    #[wasm_bindgen(js_name = positionDeltas)]
    pub fn position_deltas(&self, position_idx: usize) -> Vec<f64> {
        self.inner.position_deltas(position_idx).to_vec()
    }

    /// Get the sensitivity column for a factor (one value per position).
    #[wasm_bindgen(js_name = factorDeltas)]
    pub fn factor_deltas(&self, factor_idx: usize) -> Vec<f64> {
        self.inner.factor_deltas(factor_idx)
    }
}

// ---------------------------------------------------------------------------
// ScenarioGrid
// ---------------------------------------------------------------------------

/// Symmetric grid of scenario shifts used by the full repricing engine.
///
/// @example
/// ```javascript
/// const grid = new ScenarioGrid(5);  // [-2, -1, 0, 1, 2]
/// console.log(grid.shifts);
/// ```
#[wasm_bindgen(js_name = ScenarioGrid)]
#[derive(Clone)]
pub struct JsScenarioGrid {
    inner: ScenarioGrid,
}

#[wasm_bindgen(js_class = ScenarioGrid)]
impl JsScenarioGrid {
    /// Create a grid centered on zero with `nPoints` shift levels.
    #[wasm_bindgen(constructor)]
    pub fn new(n_points: usize) -> Self {
        Self {
            inner: ScenarioGrid::new(n_points),
        }
    }

    /// Ordered shift coordinates.
    #[wasm_bindgen(getter)]
    pub fn shifts(&self) -> Vec<f64> {
        self.inner.shifts().to_vec()
    }
}

// ---------------------------------------------------------------------------
// FactorPnlProfile
// ---------------------------------------------------------------------------

/// P&L profile for one factor across a scenario grid.
///
/// Contains the shifted P&L vectors for each scenario point.
#[wasm_bindgen(js_name = FactorPnlProfile)]
#[derive(Clone)]
pub struct JsFactorPnlProfile {
    inner: FactorPnlProfile,
}

#[wasm_bindgen(js_class = FactorPnlProfile)]
impl JsFactorPnlProfile {
    /// Identifier of the shocked factor.
    #[wasm_bindgen(getter, js_name = factorId)]
    pub fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }

    /// Scenario shift coordinates.
    #[wasm_bindgen(getter)]
    pub fn shifts(&self) -> Vec<f64> {
        self.inner.shifts.clone()
    }
}
