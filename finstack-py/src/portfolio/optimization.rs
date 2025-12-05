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
use finstack_core::prelude::*;
use finstack_portfolio::{
    Constraint, DefaultLpOptimizer, MetricExpr, MissingMetricPolicy, Objective, PerPositionMetric,
    PortfolioOptimizationProblem, PortfolioOptimizer, WeightingScheme,
};
use finstack_valuations::metrics::MetricId;
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
#[pyo3(signature = (portfolio, market_context, ccc_limit=0.20, strict_risk=false, config=None))]
fn optimize_max_yield_with_ccc_limit(
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
        FinstackConfig::default()
    };

    // Objective: maximize value‑weighted average yield (YTM).
    let objective = Objective::Maximize(MetricExpr::ValueWeightedAverage {
        metric: PerPositionMetric::Metric(MetricId::Ytm),
    });

    let mut problem = PortfolioOptimizationProblem::new(portfolio_inner, objective);
    problem.weighting = WeightingScheme::ValueWeight;
    problem.missing_metric_policy = if strict_risk {
        MissingMetricPolicy::Strict
    } else {
        MissingMetricPolicy::Zero
    };
    problem.label = Some("max_yield_with_ccc_limit".to_string());

    // CCC exposure constraint.
    problem = problem.with_constraint(Constraint::TagExposureLimit {
        label: Some("ccc_limit".to_string()),
        tag_key: "rating".to_string(),
        tag_value: "CCC".to_string(),
        max_share: ccc_limit,
    });

    let optimizer = DefaultLpOptimizer::default();
    let result = optimizer
        .optimize(&problem, &market_ctx.inner, &cfg)
        .map_err(portfolio_to_py)?;

    // Compute CCC exposure from optimal weights and tags.
    let mut ccc_weight = 0.0_f64;
    let portfolio_ref = &result.problem.portfolio;
    for (pos_id, &w) in &result.optimal_weights {
        if let Some(position) = portfolio_ref.get_position(pos_id.as_str()) {
            if position.tags.get("rating").map(String::as_str) == Some("CCC") {
                ccc_weight += w;
            }
        }
    }

    // Build a simple Python dict result.
    let out = PyDict::new(py);

    out.set_item("label", result.problem.label.clone())?;
    out.set_item("status", format!("{:?}", result.status))?;
    out.set_item("objective_value", result.objective_value)?;
    out.set_item("ccc_weight", ccc_weight)?;

    // Optimal weights: { position_id: weight }.
    let weights = PyDict::new(py);
    for (pos_id, w) in &result.optimal_weights {
        weights.set_item(pos_id.as_str(), *w)?;
    }
    out.set_item("optimal_weights", weights)?;

    // Current weights and deltas can be useful for trade generation.
    let current = PyDict::new(py);
    for (pos_id, w) in &result.current_weights {
        current.set_item(pos_id.as_str(), *w)?;
    }
    out.set_item("current_weights", current)?;

    let deltas = PyDict::new(py);
    for (pos_id, dw) in &result.weight_deltas {
        deltas.set_item(pos_id.as_str(), *dw)?;
    }
    out.set_item("weight_deltas", deltas)?;

    Ok(out.into())
}

/// Register optimization helpers.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    let func = wrap_pyfunction!(optimize_max_yield_with_ccc_limit, parent)?;
    parent.add_function(func)?;

    Ok(vec!["optimize_max_yield_with_ccc_limit".to_string()])
}
