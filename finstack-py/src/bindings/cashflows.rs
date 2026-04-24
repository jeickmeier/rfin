//! Python bindings for the `finstack-cashflows` crate.

use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

#[pyfunction]
#[pyo3(signature = (spec_json, market_json = None))]
fn build_cashflow_schedule(spec_json: &str, market_json: Option<&str>) -> PyResult<String> {
    finstack_cashflows::build_cashflow_schedule_json(spec_json, market_json)
        .map_err(crate::errors::core_to_py)
}

#[pyfunction]
fn validate_cashflow_schedule(schedule_json: &str) -> PyResult<String> {
    finstack_cashflows::validate_cashflow_schedule_json(schedule_json)
        .map_err(crate::errors::core_to_py)
}

#[pyfunction]
fn dated_flows(schedule_json: &str) -> PyResult<String> {
    finstack_cashflows::dated_flows_json(schedule_json).map_err(crate::errors::core_to_py)
}

#[pyfunction]
#[pyo3(signature = (schedule_json, as_of, config_json = None))]
fn accrued_interest(schedule_json: &str, as_of: &str, config_json: Option<&str>) -> PyResult<f64> {
    finstack_cashflows::accrued_interest_json(schedule_json, as_of, config_json)
        .map_err(crate::errors::core_to_py)
}

#[pyfunction]
#[pyo3(signature = (instrument_id, schedule_json, discount_curve_id, quoted_clean = None))]
fn bond_from_cashflows(
    instrument_id: &str,
    schedule_json: &str,
    discount_curve_id: &str,
    quoted_clean: Option<f64>,
) -> PyResult<String> {
    let schedule: finstack_cashflows::builder::CashFlowSchedule =
        serde_json::from_str(schedule_json).map_err(crate::errors::display_to_py)?;
    let bond = finstack_valuations::instruments::fixed_income::bond::Bond::from_cashflows(
        instrument_id,
        schedule,
        discount_curve_id,
        quoted_clean,
    )
    .map_err(crate::errors::core_to_py)?;
    let instrument = finstack_valuations::instruments::InstrumentJson::Bond(bond);
    serde_json::to_string(&instrument).map_err(crate::errors::display_to_py)
}

/// Register the `finstack.cashflows` Python namespace.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "cashflows")?;
    m.setattr(
        "__doc__",
        "Cashflow schedule JSON construction, validation, and bond conversion.",
    )?;

    m.add_function(wrap_pyfunction!(build_cashflow_schedule, &m)?)?;
    m.add_function(wrap_pyfunction!(validate_cashflow_schedule, &m)?)?;
    m.add_function(wrap_pyfunction!(dated_flows, &m)?)?;
    m.add_function(wrap_pyfunction!(accrued_interest, &m)?)?;
    m.add_function(wrap_pyfunction!(bond_from_cashflows, &m)?)?;

    let all = PyList::new(
        py,
        [
            "build_cashflow_schedule",
            "validate_cashflow_schedule",
            "dated_flows",
            "accrued_interest",
            "bond_from_cashflows",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let parent_name: String = match parent.getattr("__name__") {
        Ok(attr) => attr
            .extract::<String>()
            .unwrap_or_else(|_| "finstack.finstack".to_string()),
        Err(_) => "finstack.finstack".to_string(),
    };
    let qual = format!("{parent_name}.cashflows");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
