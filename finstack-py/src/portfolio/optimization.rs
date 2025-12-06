//! Python bindings for portfolio optimization (preview API).
//!
//! This module exposes a focused optimization helper:
//! `optimize_max_yield_with_ccc_limit`, which mirrors the Rust example
//! and the finance‑realistic integration test. It is intended as a
//! first Python entrypoint into the new Rust optimization engine.

use crate::core::config::PyFinstackConfig;
use crate::core::market_data::context::PyMarketContext;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::portfolio::extract_portfolio;
use finstack_portfolio::optimization::optimize_max_yield_with_ccc_limit;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};
use pyo3::Bound;

/// Optimize a bond portfolio to maximize value‑weighted YTM with a CCC exposure limit.
///
/// This helper mirrors the Rust example and the integration test:
///
/// - Objective: maximize value‑weighted average yield (`MetricId::Ytm`)
/// - Constraint: rating = "CCC" exposure (by weight) <= `ccc_limit`
/// - Budget: implicit `sum_i w_i == 1` via the default problem constructor
///
/// The portfolio should:
/// - Be USD‑denominated (base_ccy = USD)
/// - Contain bond‑like instruments that expose `MetricId::Ytm`
/// - Tag high‑yield positions with `rating="CCC"` for the constraint
#[pyfunction]
#[pyo3(
    name = "optimize_max_yield_with_ccc_limit",
    signature = (portfolio, market_context, ccc_limit=0.20, strict_risk=false, config=None)
)]
fn py_optimize_max_yield_with_ccc_limit(
    py: Python<'_>,
    portfolio: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
    ccc_limit: f64,
    strict_risk: bool,
    config: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyObject> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;

    let cfg = if let Some(config_obj) = config {
        config_obj
            .extract::<PyRef<PyFinstackConfig>>()?
            .inner
            .clone()
    } else {
        finstack_core::config::FinstackConfig::default()
    };

    let result = optimize_max_yield_with_ccc_limit(
        &portfolio_inner,
        &market_ctx.inner,
        &cfg,
        ccc_limit,
        strict_risk,
    )
    .map_err(portfolio_to_py)?;

    let out = PyDict::new(py);

    out.set_item("label", result.label.clone())?;
    out.set_item("status", result.status_label.clone())?;
    out.set_item("objective_value", result.objective_value)?;
    out.set_item("ccc_weight", result.ccc_weight)?;

    // Optimal weights: { position_id: weight }.
    out.set_item("optimal_weights", map_weights(py, &result.optimal_weights)?)?;

    // Current weights and deltas can be useful for trade generation.
    out.set_item("current_weights", map_weights(py, &result.current_weights)?)?;
    out.set_item("weight_deltas", map_weights(py, &result.weight_deltas)?)?;

    Ok(out.into())
}

fn map_weights(
    py: Python<'_>,
    weights: &indexmap::IndexMap<finstack_portfolio::types::PositionId, f64>,
) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    for (pos_id, weight) in weights {
        dict.set_item(pos_id.as_str(), *weight)?;
    }
    Ok(dict.into())
}

/// Register optimization helpers.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    let func = wrap_pyfunction!(py_optimize_max_yield_with_ccc_limit, parent)?;
    parent.add_function(func)?;

    Ok(vec!["optimize_max_yield_with_ccc_limit".to_string()])
}
