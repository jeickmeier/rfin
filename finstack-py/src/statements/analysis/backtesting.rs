//! Forecast backtesting bindings.

use crate::statements::error::stmt_to_py;
use finstack_statements_analytics::analysis::{backtest_forecast as rs_backtest_forecast, ForecastMetrics};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{wrap_pyfunction, Bound};

/// Forecast accuracy metrics.
#[pyclass(
    module = "finstack.statements.analysis",
    name = "ForecastMetrics",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyForecastMetrics {
    inner: ForecastMetrics,
}

#[pymethods]
impl PyForecastMetrics {
    #[getter]
    /// Mean Absolute Error.
    fn mae(&self) -> f64 {
        self.inner.mae
    }

    #[getter]
    /// Mean Absolute Percentage Error.
    fn mape(&self) -> f64 {
        self.inner.mape
    }

    #[getter]
    /// Root Mean Squared Error.
    fn rmse(&self) -> f64 {
        self.inner.rmse
    }

    #[getter]
    /// Number of data points.
    fn n(&self) -> usize {
        self.inner.n
    }

    /// Format metrics as a human-readable summary.
    ///
    /// Returns
    /// -------
    /// str
    ///     Summary string
    fn summary(&self) -> String {
        self.inner.summary()
    }

    fn __repr__(&self) -> String {
        format!(
            "ForecastMetrics(mae={:.4}, mape={:.4}%, rmse={:.4}, n={})",
            self.inner.mae, self.inner.mape, self.inner.rmse, self.inner.n
        )
    }
}

#[pyfunction]
#[pyo3(signature = (actual, forecast), name = "backtest_forecast")]
/// Compute forecast error metrics by comparing actual vs forecast values.
///
/// Parameters
/// ----------
/// actual : list[float]
///     Actual observed values
/// forecast : list[float]
///     Forecasted/predicted values
///
/// Returns
/// -------
/// ForecastMetrics
///     Metrics containing MAE, MAPE, and RMSE
fn py_backtest_forecast(actual: Vec<f64>, forecast: Vec<f64>) -> PyResult<PyForecastMetrics> {
    let metrics = rs_backtest_forecast(&actual, &forecast).map_err(stmt_to_py)?;
    Ok(PyForecastMetrics { inner: metrics })
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyForecastMetrics>()?;
    module.add_function(wrap_pyfunction!(py_backtest_forecast, module)?)?;
    Ok(vec!["ForecastMetrics", "backtest_forecast"])
}
