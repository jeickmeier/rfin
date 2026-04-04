//! Structured metric types for WASM.
//!
//! Provides 2D and 3D containers for matrix/tensor metric results,
//! plus the DV01 computation mode enum.

use crate::core::error::js_error;
use finstack_valuations::metrics::{Structured2D, Structured3D};
use wasm_bindgen::prelude::*;

// =============================================================================
// Structured2D
// =============================================================================

/// A 2D labeled matrix of metric values (rows x columns).
///
/// Used for vega surfaces, bucketed DV01 results, etc.
#[wasm_bindgen(js_name = Structured2D)]
pub struct JsStructured2D {
    inner: Structured2D,
}

#[wasm_bindgen(js_class = Structured2D)]
impl JsStructured2D {
    /// Create a Structured2D from labeled rows, columns, and a flat value array.
    ///
    /// Values should be in row-major order: `values[r * cols.length + c]`.
    #[wasm_bindgen(constructor)]
    pub fn new(
        rows: Vec<String>,
        cols: Vec<String>,
        flat_values: Vec<f64>,
    ) -> Result<JsStructured2D, JsValue> {
        let n_rows = rows.len();
        let n_cols = cols.len();
        if flat_values.len() != n_rows * n_cols {
            return Err(js_error(format!(
                "Expected {} values ({}x{}), got {}",
                n_rows * n_cols,
                n_rows,
                n_cols,
                flat_values.len()
            )));
        }
        let values: Vec<Vec<f64>> = flat_values.chunks(n_cols).map(|c| c.to_vec()).collect();
        let inner = Structured2D { rows, cols, values };
        if !inner.validate_shape() {
            return Err(js_error("Invalid matrix shape"));
        }
        Ok(JsStructured2D { inner })
    }

    /// Row labels.
    #[wasm_bindgen(getter)]
    pub fn rows(&self) -> Vec<String> {
        self.inner.rows.clone()
    }

    /// Column labels.
    #[wasm_bindgen(getter)]
    pub fn cols(&self) -> Vec<String> {
        self.inner.cols.clone()
    }

    /// Number of rows.
    #[wasm_bindgen(getter, js_name = numRows)]
    pub fn num_rows(&self) -> usize {
        self.inner.rows.len()
    }

    /// Number of columns.
    #[wasm_bindgen(getter, js_name = numCols)]
    pub fn num_cols(&self) -> usize {
        self.inner.cols.len()
    }

    /// Get a single value by row and column index.
    #[wasm_bindgen(js_name = getValue)]
    pub fn get_value(&self, row: usize, col: usize) -> Result<f64, JsValue> {
        self.inner
            .values
            .get(row)
            .and_then(|r| r.get(col))
            .copied()
            .ok_or_else(|| js_error(format!("Index out of bounds: [{}, {}]", row, col)))
    }

    /// Get all values as a flat Float64Array (row-major).
    #[wasm_bindgen(js_name = flatValues)]
    pub fn flat_values(&self) -> js_sys::Float64Array {
        let flat: Vec<f64> = self
            .inner
            .values
            .iter()
            .flat_map(|r| r.iter().copied())
            .collect();
        js_sys::Float64Array::from(flat.as_slice())
    }

    /// Serialize to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let obj = serde_json::json!({
            "rows": self.inner.rows,
            "cols": self.inner.cols,
            "values": self.inner.values,
        });
        serde_wasm_bindgen::to_value(&obj)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }
}

#[allow(dead_code)]
impl JsStructured2D {
    pub(crate) fn from_inner(inner: Structured2D) -> Self {
        Self { inner }
    }
}

// =============================================================================
// Structured3D
// =============================================================================

/// A 3D labeled tensor of metric values (axis A x B x C).
///
/// Used for 3D bucketed vegas (expiry x tenor x strike), etc.
#[wasm_bindgen(js_name = Structured3D)]
pub struct JsStructured3D {
    inner: Structured3D,
}

#[wasm_bindgen(js_class = Structured3D)]
impl JsStructured3D {
    /// Axis A labels.
    #[wasm_bindgen(getter, js_name = axisA)]
    pub fn axis_a(&self) -> Vec<String> {
        self.inner.a.clone()
    }

    /// Axis B labels.
    #[wasm_bindgen(getter, js_name = axisB)]
    pub fn axis_b(&self) -> Vec<String> {
        self.inner.b.clone()
    }

    /// Axis C labels.
    #[wasm_bindgen(getter, js_name = axisC)]
    pub fn axis_c(&self) -> Vec<String> {
        self.inner.c.clone()
    }

    /// Get a single value by indices (a, b, c).
    #[wasm_bindgen(js_name = getValue)]
    pub fn get_value(&self, a: usize, b: usize, c: usize) -> Result<f64, JsValue> {
        self.inner
            .values
            .get(a)
            .and_then(|plane| plane.get(b))
            .and_then(|row| row.get(c))
            .copied()
            .ok_or_else(|| js_error(format!("Index out of bounds: [{}, {}, {}]", a, b, c)))
    }

    /// Serialize to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let obj = serde_json::json!({
            "a": self.inner.a,
            "b": self.inner.b,
            "c": self.inner.c,
            "values": self.inner.values,
        });
        serde_wasm_bindgen::to_value(&obj)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }
}

#[allow(dead_code)]
impl JsStructured3D {
    pub(crate) fn from_inner(inner: Structured3D) -> Self {
        Self { inner }
    }
}

// =============================================================================
// Dv01ComputationMode
// =============================================================================

/// DV01 calculation mode controlling how sensitivities are computed.
#[wasm_bindgen(js_name = Dv01ComputationMode)]
#[derive(Clone, Copy)]
pub struct JsDv01ComputationMode {
    mode: Dv01Mode,
}

#[derive(Clone, Copy)]
enum Dv01Mode {
    ParallelCombined,
    ParallelPerCurve,
    KeyRateTriangular,
}

#[wasm_bindgen(js_class = Dv01ComputationMode)]
impl JsDv01ComputationMode {
    /// Single scalar from parallel bump of all curves together.
    #[wasm_bindgen(js_name = parallelCombined)]
    pub fn parallel_combined() -> Self {
        Self {
            mode: Dv01Mode::ParallelCombined,
        }
    }

    /// Per-curve parallel bump (each curve bumped independently).
    #[wasm_bindgen(js_name = parallelPerCurve)]
    pub fn parallel_per_curve() -> Self {
        Self {
            mode: Dv01Mode::ParallelPerCurve,
        }
    }

    /// Key-rate DV01 with triangular bump profiles at standard tenors.
    #[wasm_bindgen(js_name = keyRateTriangular)]
    pub fn key_rate_triangular() -> Self {
        Self {
            mode: Dv01Mode::KeyRateTriangular,
        }
    }

    /// Get the mode name as a string.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        match self.mode {
            Dv01Mode::ParallelCombined => "parallel_combined".to_string(),
            Dv01Mode::ParallelPerCurve => "parallel_per_curve".to_string(),
            Dv01Mode::KeyRateTriangular => "key_rate_triangular".to_string(),
        }
    }
}
