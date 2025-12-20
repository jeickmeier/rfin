use super::config::PyCalibrationConfig;
use super::quote::PyMarketQuote;
use super::report::PyCalibrationReport;
use crate::core::market_data::context::PyMarketContext;
use crate::statements::utils::py_to_json;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::api::engine as calib_engine_v2;
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelope, CalibrationPlan, CalibrationStep, CALIBRATION_SCHEMA,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyModule;
use pyo3::Bound;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;

fn step_from_py(value: &Bound<'_, PyAny>) -> PyResult<CalibrationStep> {
    let json_value = py_to_json(value)?;
    serde_json::from_value(json_value).map_err(|e| {
        PyValueError::new_err(format!(
            "Invalid v2 calibration step. Expected a dict matching CalibrationStep schema: {e}"
        ))
    })
}

fn quote_sets_from_py(
    py: Python<'_>,
    quote_sets: HashMap<String, Vec<Py<PyMarketQuote>>>,
) -> PyResult<HashMap<String, Vec<finstack_valuations::market::quotes::market_quote::MarketQuote>>>
{
    let mut out = HashMap::with_capacity(quote_sets.len());
    for (k, quotes) in quote_sets {
        let mut v = Vec::with_capacity(quotes.len());
        for q in quotes {
            let qref = q.bind(py).try_borrow().map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "MarketQuote is already mutably borrowed; cannot read inner quote: {e}"
                ))
            })?;
            v.push(qref.inner.clone());
        }
        out.insert(k, v);
    }
    Ok(out)
}

/// Execute a v2 (plan-driven) calibration.
///
/// Parameters
/// ----------
/// plan_id : str
///     Identifier for this plan execution.
/// quote_sets : dict[str, list[MarketQuote]]
///     Named collections of quotes; steps reference these by name.
/// steps : list[dict]
///     Each step is a JSON-like dict matching the v2 schema for `CalibrationStep`.
/// settings : CalibrationConfig | None
///     Optional global calibration settings (tolerances, bounds, etc.).
/// initial_market : MarketContext | None
///     Optional initial market context; if omitted, calibration starts from an empty context.
/// description : str | None
///     Optional plan description.
///
/// Returns
/// -------
/// tuple[MarketContext, CalibrationReport, dict[str, CalibrationReport]]
///     The calibrated market context, merged report, and per-step reports.
#[pyfunction]
#[pyo3(signature = (plan_id, quote_sets, steps, settings=None, initial_market=None, description=None))]
fn execute_calibration_v2(
    py: Python<'_>,
    plan_id: String,
    quote_sets: HashMap<String, Vec<Py<PyMarketQuote>>>,
    steps: Vec<Bound<'_, PyAny>>,
    settings: Option<PyRef<'_, PyCalibrationConfig>>,
    initial_market: Option<PyRef<'_, PyMarketContext>>,
    description: Option<String>,
) -> PyResult<Py<PyAny>> {
    let quote_sets = quote_sets_from_py(py, quote_sets)?;
    let mut rust_steps = Vec::with_capacity(steps.len());
    for step in steps {
        rust_steps.push(step_from_py(&step)?);
    }

    let plan = CalibrationPlan {
        id: plan_id,
        description,
        quote_sets,
        steps: rust_steps,
        settings: settings
            .as_ref()
            .map(|s| s.inner.clone())
            .unwrap_or_default(),
    };

    let initial_market = initial_market.as_ref().map(|ctx| (&ctx.inner).into());

    let envelope = CalibrationEnvelope {
        schema: CALIBRATION_SCHEMA.to_string(),
        plan,
        initial_market,
    };

    // Release GIL for compute-heavy calibration work
    let result = py
        .detach(|| calib_engine_v2::execute(&envelope))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let market = MarketContext::try_from(result.result.final_market.clone()).map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!(
            "Failed to build MarketContext from calibration result: {e}"
        ))
    })?;

    let step_reports = PyDict::new(py);
    for (k, v) in result.result.step_reports.iter() {
        step_reports.set_item(k, PyCalibrationReport::new(v.clone()))?;
    }

    (
        PyMarketContext { inner: market },
        PyCalibrationReport::new(result.result.report.clone()),
        step_reports,
    )
        .into_py_any(py)
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add("CALIBRATION_SCHEMA", CALIBRATION_SCHEMA)?;
    module.add_function(wrap_pyfunction!(execute_calibration_v2, module)?)?;
    Ok(vec!["CALIBRATION_SCHEMA", "execute_calibration_v2"])
}
