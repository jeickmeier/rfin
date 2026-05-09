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
use finstack_valuations::calibration::api::errors::EnvelopeError;
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelope, CalibrationResultEnvelope,
};
use finstack_valuations::calibration::api::validate as validate_api;
use pyo3::create_exception;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

create_exception!(
    finstack.valuations,
    CalibrationEnvelopeError,
    PyRuntimeError,
    "Raised when a calibration envelope fails validation or solving.\n\n\
     Inherits from RuntimeError, so existing `except RuntimeError` callers \
     continue to catch it. Carries `kind`, `step_id`, and `details` \
     attributes for programmatic handling."
);

/// Build a `CalibrationEnvelopeError` from a structured `EnvelopeError`,
/// attaching `kind`, `step_id`, and pretty-printed `details` attributes.
fn envelope_error_to_py(py: Python<'_>, err: &EnvelopeError) -> PyErr {
    let exc = CalibrationEnvelopeError::new_err(err.to_string());
    let value = exc.value(py);
    let _ = value.setattr("kind", err.kind_str());
    let _ = value.setattr("details", err.to_json());
    let _ = value.setattr("step_id", err.step_id().map(|s| s.to_string()));
    exc
}

/// Map a finstack_core::Error back into a CalibrationEnvelopeError when it
/// originated from an EnvelopeError, otherwise fall through to display_to_py.
///
/// `From<EnvelopeError> for finstack_core::Error` produces an
/// `Error::Calibration` whose `category` matches the EnvelopeError's
/// `kind_str`. We surface those fields back to Python so callers can pattern-
/// match on `exc.kind`.
fn calibration_error_to_py(py: Python<'_>, err: finstack_core::Error) -> PyErr {
    if let finstack_core::Error::Calibration { message, category } = &err {
        let exc = CalibrationEnvelopeError::new_err(message.clone());
        let value = exc.value(py);
        let _ = value.setattr("kind", category.as_str());
        let _ = value.setattr("details", message.clone());
        let _ = value.setattr("step_id", Option::<String>::None);
        return exc;
    }
    display_to_py(err)
}

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
fn validate_calibration_json(py: Python<'_>, json: &str) -> PyResult<String> {
    let parsed: CalibrationEnvelope = serde_json::from_str(json).map_err(|e| {
        envelope_error_to_py(
            py,
            &EnvelopeError::JsonParse {
                message: e.to_string(),
                line: Some(e.line() as u32),
                col: Some(e.column() as u32),
            },
        )
    })?;
    serde_json::to_string_pretty(&parsed).map_err(display_to_py)
}

/// Pre-flight envelope validation without invoking the solver.
///
/// Parameters
/// ----------
/// json : str
///     JSON-serialized ``CalibrationEnvelope``.
///
/// Returns
/// -------
/// str
///     Pretty-printed JSON ``ValidationReport`` with all errors found in a
///     single pass plus the dependency graph.
///
/// Raises
/// ------
/// CalibrationEnvelopeError
///     If the envelope JSON is malformed.
#[pyfunction]
fn dry_run(py: Python<'_>, json: &str) -> PyResult<String> {
    validate_api::dry_run(json).map_err(|e| envelope_error_to_py(py, &e))
}

/// Dump the static dependency graph of a calibration plan.
///
/// Parameters
/// ----------
/// json : str
///     JSON-serialized ``CalibrationEnvelope``.
///
/// Returns
/// -------
/// str
///     Pretty-printed JSON ``DependencyGraph`` with ``initial_ids`` and
///     ``nodes`` (per-step reads/writes in declared order).
///
/// Raises
/// ------
/// CalibrationEnvelopeError
///     If the envelope JSON is malformed.
#[pyfunction]
fn dependency_graph_json(py: Python<'_>, json: &str) -> PyResult<String> {
    validate_api::dependency_graph_json(json).map_err(|e| envelope_error_to_py(py, &e))
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
fn calibrate(py: Python<'_>, json: &str) -> PyResult<PyCalibrationResult> {
    let envelope: CalibrationEnvelope = serde_json::from_str(json).map_err(|e| {
        envelope_error_to_py(
            py,
            &EnvelopeError::JsonParse {
                message: e.to_string(),
                line: Some(e.line() as u32),
                col: Some(e.column() as u32),
            },
        )
    })?;
    // Release the GIL for the duration of the solver: calibration can run for seconds.
    let result = py
        .detach(|| engine::execute(&envelope))
        .map_err(|e| calibration_error_to_py(py, e))?;
    Ok(PyCalibrationResult { inner: result })
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Register calibration functions and types on the valuations submodule.
pub fn register(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCalibrationResult>()?;
    m.add(
        "CalibrationEnvelopeError",
        py.get_type::<CalibrationEnvelopeError>(),
    )?;
    m.add_function(pyo3::wrap_pyfunction!(validate_calibration_json, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(calibrate, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(dry_run, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(dependency_graph_json, m)?)?;
    Ok(())
}
