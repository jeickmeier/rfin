//! Forecast helper bindings.
//!
//! This module exposes forecast generation utilities directly, without requiring
//! building and evaluating a full model.

use crate::core::dates::periods::PyPeriodId;
use crate::statements::error::stmt_to_py;
use crate::statements::types::forecast::PyForecastSpec;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::{PyDict, PyModule};
use pyo3::Bound;

/// Apply a forecast method to generate values for forecast periods.
///
/// Args:
///     spec: Forecast specification with method and parameters.
///     base_value: Starting value (typically last actual value).
///     forecast_periods: List of periods to forecast.
///
/// Returns:
///     dict[PeriodId, float]: Mapping of period_id → forecasted value.
#[pyfunction(
    name = "apply_forecast",
    text_signature = "(spec, base_value, forecast_periods)"
)]
fn apply_forecast_py(
    spec: &PyForecastSpec,
    base_value: f64,
    forecast_periods: Vec<PyPeriodId>,
    py: Python<'_>,
) -> PyResult<Py<PyAny>> {
    let periods_inner: Vec<finstack_core::dates::PeriodId> =
        forecast_periods.into_iter().map(|p| p.inner).collect();

    let values =
        finstack_statements::forecast::apply_forecast(&spec.inner, base_value, &periods_inner)
            .map_err(stmt_to_py)?;

    let dict = PyDict::new(py);
    for (period_id, value) in values {
        dict.set_item(PyPeriodId::new(period_id), value)?;
    }
    Ok(dict.into())
}

/// Register forecast helper exports.
pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "forecast")?;
    module.setattr(
        "__doc__",
        "Forecast helpers mirroring finstack_statements::forecast utilities.",
    )?;

    module.add_function(wrap_pyfunction!(apply_forecast_py, &module)?)?;
    let exports = ["apply_forecast"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    parent.setattr("forecast", &module)?;
    Ok(exports.to_vec())
}
