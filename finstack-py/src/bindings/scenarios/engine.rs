//! Python wrappers for scenario engine application.

use crate::bindings::extract::{extract_market, extract_model};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

fn scn_to_py(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Apply a scenario to a market context and financial model.
///
/// Parameters
/// ----------
/// scenario_json : str
///     JSON-serialized ``ScenarioSpec``.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// model : FinancialModelSpec | str
///     A ``FinancialModelSpec`` object or a JSON string.
/// as_of : str
///     Valuation date in ISO 8601 format.
///
/// Returns
/// -------
/// dict
///     Dict with ``market_json`` (modified market), ``model_json`` (modified
///     model), ``operations_applied`` (int), and ``warnings`` (list[str]).
#[pyfunction]
fn apply_scenario<'py>(
    py: Python<'py>,
    scenario_json: &str,
    market: &Bound<'py, PyAny>,
    model: &Bound<'py, PyAny>,
    as_of: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let spec: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(scn_to_py)?;
    let mut market = extract_market(market)?;
    let mut model = extract_model(model)?;
    let date = super::parse_date(as_of)?;

    let engine = finstack_scenarios::ScenarioEngine::new();
    let mut ctx = finstack_scenarios::ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: date,
    };

    let report = engine.apply(&spec, &mut ctx).map_err(scn_to_py)?;

    let dict = PyDict::new(py);
    dict.set_item(
        "market_json",
        serde_json::to_string(&market).map_err(scn_to_py)?,
    )?;
    dict.set_item(
        "model_json",
        serde_json::to_string(&model).map_err(scn_to_py)?,
    )?;
    dict.set_item("operations_applied", report.operations_applied)?;
    dict.set_item("warnings", &report.warnings)?;

    Ok(dict)
}

/// Apply a scenario to a market context only (no model).
///
/// Parameters
/// ----------
/// scenario_json : str
///     JSON-serialized ``ScenarioSpec``.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// as_of : str
///     Valuation date in ISO 8601 format.
///
/// Returns
/// -------
/// dict
///     Dict with ``market_json`` (modified market), ``operations_applied``,
///     and ``warnings``.
#[pyfunction]
fn apply_scenario_to_market<'py>(
    py: Python<'py>,
    scenario_json: &str,
    market: &Bound<'py, PyAny>,
    as_of: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let spec: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(scn_to_py)?;
    let mut market = extract_market(market)?;
    let mut model = finstack_statements::FinancialModelSpec::new("__scenario_temp__", vec![]);
    let date = super::parse_date(as_of)?;

    let engine = finstack_scenarios::ScenarioEngine::new();
    let mut ctx = finstack_scenarios::ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: date,
    };

    let report = engine.apply(&spec, &mut ctx).map_err(scn_to_py)?;

    let dict = PyDict::new(py);
    dict.set_item(
        "market_json",
        serde_json::to_string(&market).map_err(scn_to_py)?,
    )?;
    dict.set_item("operations_applied", report.operations_applied)?;
    dict.set_item("warnings", &report.warnings)?;

    Ok(dict)
}

/// Register engine functions.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(apply_scenario, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(apply_scenario_to_market, m)?)?;
    Ok(())
}
