//! Python bindings for the calibration engine.
//!
//! Wraps [`finstack_valuations::calibration::api::engine::execute`] behind
//! a JSON-in / rich-result-out API that matches the existing scenarios-engine
//! binding pattern.

use crate::bindings::core::market_data::context::PyMarketContext;
use crate::bindings::pandas_utils::dict_to_dataframe;
use crate::errors::display_to_py;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelope, CalibrationResultEnvelope,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

// ---------------------------------------------------------------------------
// CalibrationResult
// ---------------------------------------------------------------------------

/// Result of a calibration plan execution.
///
/// Provides access to the calibrated market context, per-step reports,
/// and overall success status.
#[pyclass(
    name = "CalibrationResult",
    module = "finstack.valuations",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCalibrationResult {
    inner: CalibrationResultEnvelope,
}

#[pymethods]
impl PyCalibrationResult {
    /// Deserialize from a JSON string.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: CalibrationResultEnvelope = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize to a pretty-printed JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(display_to_py)
    }

    /// Whether the overall calibration succeeded (all steps passed fitting and validation).
    #[getter]
    fn success(&self) -> bool {
        self.inner.result.report.success
    }

    /// The calibrated ``MarketContext`` containing all produced curves and surfaces.
    #[getter]
    fn market(&self) -> PyResult<PyMarketContext> {
        let ctx = MarketContext::try_from(self.inner.result.final_market.clone())
            .map_err(display_to_py)?;
        Ok(PyMarketContext::from_inner(ctx))
    }

    /// The calibrated market serialized as a JSON string.
    #[getter]
    fn market_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner.result.final_market).map_err(display_to_py)
    }

    /// The aggregated calibration report as a JSON string.
    #[getter]
    fn report_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner.result.report).map_err(display_to_py)
    }

    /// List of step identifiers that were executed.
    #[getter]
    fn step_ids(&self) -> Vec<String> {
        self.inner.result.step_reports.keys().cloned().collect()
    }

    /// Number of solver iterations across all steps.
    #[getter]
    fn iterations(&self) -> usize {
        self.inner.result.report.iterations
    }

    /// Maximum absolute residual across all steps.
    #[getter]
    fn max_residual(&self) -> f64 {
        self.inner.result.report.max_residual
    }

    /// Root mean square error across all steps.
    #[getter]
    fn rmse(&self) -> f64 {
        self.inner.result.report.rmse
    }

    /// Per-step calibration report as a JSON string.
    ///
    /// Parameters
    /// ----------
    /// step_id : str
    ///     Identifier of the calibration step.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON-serialized calibration report for the step.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If no step with the given *step_id* exists.
    fn step_report_json(&self, step_id: &str) -> PyResult<String> {
        let report = self
            .inner
            .result
            .step_reports
            .get(step_id)
            .ok_or_else(|| PyValueError::new_err(format!("No step report for '{step_id}'")))?;
        serde_json::to_string_pretty(report).map_err(display_to_py)
    }

    /// Per-step summary as a pandas ``DataFrame``.
    ///
    /// Columns: ``step_id``, ``success``, ``iterations``, ``max_residual``,
    /// ``rmse``, ``convergence_reason``.
    fn report_to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let n = self.inner.result.step_reports.len();
        let mut ids: Vec<String> = Vec::with_capacity(n);
        let mut successes: Vec<bool> = Vec::with_capacity(n);
        let mut iters: Vec<usize> = Vec::with_capacity(n);
        let mut max_res: Vec<f64> = Vec::with_capacity(n);
        let mut rmses: Vec<f64> = Vec::with_capacity(n);
        let mut reasons: Vec<String> = Vec::with_capacity(n);

        for (id, report) in &self.inner.result.step_reports {
            ids.push(id.clone());
            successes.push(report.success);
            iters.push(report.iterations);
            max_res.push(report.max_residual);
            rmses.push(report.rmse);
            reasons.push(report.convergence_reason.clone());
        }

        let data = PyDict::new(py);
        data.set_item("step_id", ids)?;
        data.set_item("success", successes)?;
        data.set_item("iterations", iters)?;
        data.set_item("max_residual", max_res)?;
        data.set_item("rmse", rmses)?;
        data.set_item("convergence_reason", reasons)?;
        dict_to_dataframe(py, &data, None)
    }

    fn __repr__(&self) -> String {
        let n = self.inner.result.step_reports.len();
        format!(
            "CalibrationResult(success={}, steps={n}, iterations={}, max_residual={:.2e})",
            self.inner.result.report.success,
            self.inner.result.report.iterations,
            self.inner.result.report.max_residual,
        )
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Validate a calibration plan JSON and return the canonical (pretty-printed) form.
///
/// Parameters
/// ----------
/// json : str
///     JSON-serialized ``CalibrationEnvelope``.
///
/// Returns
/// -------
/// str
///     Canonical pretty-printed JSON.
///
/// Raises
/// ------
/// ValueError
///     If the JSON is not a valid calibration envelope.
#[pyfunction]
fn validate_calibration_json(json: &str) -> PyResult<String> {
    let parsed: CalibrationEnvelope = serde_json::from_str(json)
        .map_err(|e| PyValueError::new_err(format!("invalid calibration JSON: {e}")))?;
    serde_json::to_string_pretty(&parsed).map_err(display_to_py)
}

/// Execute a calibration plan and return the full result.
///
/// Parameters
/// ----------
/// json : str
///     JSON-serialized ``CalibrationEnvelope`` containing the plan,
///     quote sets, and optional initial market state.
///
/// Returns
/// -------
/// CalibrationResult
///     The calibration result with calibrated market, reports, and diagnostics.
///
/// Raises
/// ------
/// ValueError
///     If the JSON is invalid or calibration fails.
#[pyfunction]
fn calibrate(json: &str) -> PyResult<PyCalibrationResult> {
    let envelope: CalibrationEnvelope = serde_json::from_str(json)
        .map_err(|e| PyValueError::new_err(format!("invalid calibration JSON: {e}")))?;
    let result = engine::execute(&envelope).map_err(display_to_py)?;
    Ok(PyCalibrationResult { inner: result })
}

/// Execute a calibration plan and return only the calibrated ``MarketContext``.
///
/// Convenience wrapper around :func:`calibrate` for the common case where
/// you only need the resulting curves.
///
/// Parameters
/// ----------
/// json : str
///     JSON-serialized ``CalibrationEnvelope``.
///
/// Returns
/// -------
/// MarketContext
///     The calibrated market context.
///
/// Raises
/// ------
/// ValueError
///     If calibration fails or the result market cannot be constructed.
#[pyfunction]
fn calibrate_to_market(json: &str) -> PyResult<PyMarketContext> {
    let envelope: CalibrationEnvelope = serde_json::from_str(json)
        .map_err(|e| PyValueError::new_err(format!("invalid calibration JSON: {e}")))?;
    let result = engine::execute(&envelope).map_err(display_to_py)?;
    let ctx = MarketContext::try_from(result.result.final_market).map_err(display_to_py)?;
    Ok(PyMarketContext::from_inner(ctx))
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Register calibration functions and types on the valuations submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCalibrationResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(validate_calibration_json, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(calibrate, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(calibrate_to_market, m)?)?;
    Ok(())
}
