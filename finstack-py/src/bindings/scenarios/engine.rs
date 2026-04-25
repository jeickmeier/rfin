//! Python wrappers for scenario engine application.

use crate::bindings::extract::{extract_market, extract_model};
use crate::errors::display_to_py;
use pyo3::prelude::*;
use pyo3::types::PyDict;

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
///     model), ``operations_applied`` (int), ``user_operations`` (int, count of
///     user-provided operations before hierarchy expansion), ``expanded_operations``
///     (int, count after expansion), and ``warnings`` (list[str]).
#[pyfunction]
fn apply_scenario<'py>(
    py: Python<'py>,
    scenario_json: &str,
    market: &Bound<'py, PyAny>,
    model: &Bound<'py, PyAny>,
    as_of: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let spec: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(display_to_py)?;
    let mut market = extract_market(market)?;
    let mut model = extract_model(model)?;
    let date = super::parse_date(as_of)?;

    let engine = finstack_scenarios::ScenarioEngine::new();

    // Release the GIL for scenario application: shifts + re-pricing can run for seconds.
    let (report, market, model) = py.detach(|| {
        let mut ctx = finstack_scenarios::ExecutionContext {
            market: &mut market,
            model: &mut model,
            instruments: None,
            rate_bindings: None,
            calendar: None,
            as_of: date,
        };
        let report = engine.apply(&spec, &mut ctx);
        (report, market, model)
    });
    let report = report.map_err(display_to_py)?;

    let dict = PyDict::new(py);
    dict.set_item(
        "market_json",
        serde_json::to_string(&market).map_err(display_to_py)?,
    )?;
    dict.set_item(
        "model_json",
        serde_json::to_string(&model).map_err(display_to_py)?,
    )?;
    dict.set_item("operations_applied", report.operations_applied)?;
    dict.set_item("user_operations", report.user_operations)?;
    dict.set_item("expanded_operations", report.expanded_operations)?;
    // Warnings are surfaced under two keys:
    //   - `warnings`:       `list[str]` (rendered Display form, useful for
    //                       logs and human-readable summaries).
    //   - `warnings_json`:  `str` (JSON-encoded `Vec<Warning>` matching the
    //                       WASM binding; parse with `json.loads(...)` for
    //                       structured pattern-matching on `kind`).
    // Both views describe the same warnings; choose whichever fits the
    // caller's needs.
    let warning_strs: Vec<String> = report.warnings.iter().map(ToString::to_string).collect();
    dict.set_item("warnings", warning_strs)?;
    let warnings_json = serde_json::to_string(&report.warnings).map_err(display_to_py)?;
    dict.set_item("warnings_json", warnings_json)?;

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
///     ``user_operations``, ``expanded_operations``, and ``warnings``.
#[pyfunction]
fn apply_scenario_to_market<'py>(
    py: Python<'py>,
    scenario_json: &str,
    market: &Bound<'py, PyAny>,
    as_of: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let spec: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(display_to_py)?;
    let mut market = extract_market(market)?;
    let mut model = finstack_statements::FinancialModelSpec::new("__scenario_temp__", vec![]);
    let date = super::parse_date(as_of)?;

    let engine = finstack_scenarios::ScenarioEngine::new();

    let (report, market) = py.detach(|| {
        let mut ctx = finstack_scenarios::ExecutionContext {
            market: &mut market,
            model: &mut model,
            instruments: None,
            rate_bindings: None,
            calendar: None,
            as_of: date,
        };
        let report = engine.apply(&spec, &mut ctx);
        (report, market)
    });
    let report = report.map_err(display_to_py)?;

    let dict = PyDict::new(py);
    dict.set_item(
        "market_json",
        serde_json::to_string(&market).map_err(display_to_py)?,
    )?;
    dict.set_item("operations_applied", report.operations_applied)?;
    dict.set_item("user_operations", report.user_operations)?;
    dict.set_item("expanded_operations", report.expanded_operations)?;
    // Warnings are surfaced under two keys:
    //   - `warnings`:       `list[str]` (rendered Display form, useful for
    //                       logs and human-readable summaries).
    //   - `warnings_json`:  `str` (JSON-encoded `Vec<Warning>` matching the
    //                       WASM binding; parse with `json.loads(...)` for
    //                       structured pattern-matching on `kind`).
    // Both views describe the same warnings; choose whichever fits the
    // caller's needs.
    let warning_strs: Vec<String> = report.warnings.iter().map(ToString::to_string).collect();
    dict.set_item("warnings", warning_strs)?;
    let warnings_json = serde_json::to_string(&report.warnings).map_err(display_to_py)?;
    dict.set_item("warnings_json", warnings_json)?;

    Ok(dict)
}

/// Register engine functions.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(apply_scenario, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(apply_scenario_to_market, m)?)?;
    Ok(())
}
