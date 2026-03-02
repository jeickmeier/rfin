//! Covenant analysis bindings for financial statement forecasts.
//!
//! Wraps `finstack_statements::analysis::covenants` functions to expose
//! `forecast_covenant`, `forecast_covenants`, and `forecast_breaches`
//! through the `finstack.statements.analysis` Python module.

use crate::core::dates::periods::PyPeriodId;
use crate::statements::evaluator::PyStatementResult;
use crate::statements::types::model::PyFinancialModelSpec;
use crate::valuations::covenants::{
    PyCovenantForecast, PyCovenantForecastConfig, PyCovenantSpec, PyFutureBreach,
};
use finstack_statements::analysis::covenants as rs_covenants;
use finstack_valuations::covenants::CovenantEngine;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{wrap_pyfunction, Bound};

/// Forecast a single covenant's future compliance using statement results.
///
/// Parameters
/// ----------
/// covenant : CovenantSpec
///     Covenant specification to forecast.
/// model : FinancialModelSpec
///     Financial model with period definitions.
/// base_case : StatementResult
///     Evaluated statement results (time-series of metrics).
/// periods : list[PeriodId]
///     Periods to project over.
/// config : CovenantForecastConfig, optional
///     Forecasting configuration. Uses defaults when omitted.
///
/// Returns
/// -------
/// CovenantForecast
///     Projected covenant compliance including headroom, breach dates, etc.
#[pyfunction]
#[pyo3(
    signature = (covenant, model, base_case, periods, config=None),
    name = "forecast_covenant"
)]
fn py_forecast_covenant(
    covenant: &PyCovenantSpec,
    model: &PyFinancialModelSpec,
    base_case: &PyStatementResult,
    periods: Vec<PyPeriodId>,
    config: Option<&PyCovenantForecastConfig>,
) -> PyResult<PyCovenantForecast> {
    let rs_periods: Vec<finstack_core::dates::PeriodId> = periods.iter().map(|p| p.inner).collect();
    let cfg = config.map(|c| c.inner.clone()).unwrap_or_default();
    let result = rs_covenants::forecast_covenant(
        &covenant.inner,
        &model.inner,
        &base_case.inner,
        &rs_periods,
        cfg,
    )
    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(PyCovenantForecast { inner: result })
}

/// Forecast multiple covenants with shared statement inputs.
///
/// Parameters
/// ----------
/// covenants : list[CovenantSpec]
///     Covenant specifications to forecast.
/// model : FinancialModelSpec
///     Financial model with period definitions.
/// base_case : StatementResult
///     Evaluated statement results (time-series of metrics).
/// periods : list[PeriodId]
///     Periods to project over.
/// config : CovenantForecastConfig, optional
///     Forecasting configuration. Uses defaults when omitted.
///
/// Returns
/// -------
/// list[CovenantForecast]
///     Projected covenant compliance for each specification.
#[pyfunction]
#[pyo3(
    signature = (covenants, model, base_case, periods, config=None),
    name = "forecast_covenants"
)]
fn py_forecast_covenants(
    covenants: Vec<PyCovenantSpec>,
    model: &PyFinancialModelSpec,
    base_case: &PyStatementResult,
    periods: Vec<PyPeriodId>,
    config: Option<&PyCovenantForecastConfig>,
) -> PyResult<Vec<PyCovenantForecast>> {
    let rs_periods: Vec<finstack_core::dates::PeriodId> = periods.iter().map(|p| p.inner).collect();
    let cfg = config.map(|c| c.inner.clone()).unwrap_or_default();
    let specs: Vec<finstack_valuations::covenants::CovenantSpec> =
        covenants.iter().map(|c| c.inner.clone()).collect();
    let results =
        rs_covenants::forecast_covenants(&specs, &model.inner, &base_case.inner, &rs_periods, cfg)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(results
        .into_iter()
        .map(|r| PyCovenantForecast { inner: r })
        .collect())
}

/// Forecast covenant breaches based on statement results.
///
/// Constructs a ``CovenantEngine`` from the supplied specs and delegates to
/// `finstack_statements::analysis::covenants::forecast_breaches`.
///
/// Parameters
/// ----------
/// results : StatementResult
///     Evaluated statement results containing metric time-series.
/// covenant_specs : list[CovenantSpec]
///     Covenant specifications to check for breaches.
/// model : FinancialModelSpec, optional
///     Financial model for precise period end dates. When ``None``, dates are
///     approximated from the period identifier.
/// config : CovenantForecastConfig, optional
///     Forecasting configuration. Uses defaults when omitted.
///
/// Returns
/// -------
/// list[FutureBreach]
///     Projected breaches across all covenants and periods.
#[pyfunction]
#[pyo3(
    signature = (results, covenant_specs, model=None, config=None),
    name = "forecast_breaches"
)]
fn py_forecast_breaches(
    results: &PyStatementResult,
    covenant_specs: Vec<PyCovenantSpec>,
    model: Option<&PyFinancialModelSpec>,
    config: Option<&PyCovenantForecastConfig>,
) -> PyResult<Vec<PyFutureBreach>> {
    let cfg = config.map(|c| c.inner.clone()).unwrap_or_default();
    let mut engine = CovenantEngine::new();
    for spec in &covenant_specs {
        engine.add_spec(spec.inner.clone());
    }
    let breaches =
        rs_covenants::forecast_breaches(&results.inner, &engine, model.map(|m| &m.inner), cfg)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
    Ok(breaches
        .into_iter()
        .map(|b| PyFutureBreach { inner: b })
        .collect())
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_function(wrap_pyfunction!(py_forecast_covenant, module)?)?;
    module.add_function(wrap_pyfunction!(py_forecast_covenants, module)?)?;
    module.add_function(wrap_pyfunction!(py_forecast_breaches, module)?)?;
    Ok(vec![
        "forecast_covenant",
        "forecast_covenants",
        "forecast_breaches",
    ])
}
