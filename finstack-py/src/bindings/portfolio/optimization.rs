//! Python bindings for portfolio optimization.
//!
//! Exposes the LP-based optimizer through JSON-friendly entry points that
//! follow the same pattern as the other portfolio pipeline functions.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn opt_to_py(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Optimize a portfolio from a JSON specification.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioOptimizationSpec`` containing the portfolio,
///     objective, constraints, and weighting scheme.
/// market_json : str
///     JSON-serialized ``MarketContext``.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``PortfolioOptimizationResultJson`` with optimal weights,
///     trade list, dual values, and diagnostics.
#[pyfunction]
fn optimize_portfolio(spec_json: &str, market_json: &str) -> PyResult<String> {
    let spec: finstack_portfolio::PortfolioOptimizationSpec =
        serde_json::from_str(spec_json).map_err(opt_to_py)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(opt_to_py)?;
    let config = finstack_core::config::FinstackConfig::default();
    let result =
        finstack_portfolio::optimize_from_spec(&spec, &market, &config).map_err(opt_to_py)?;
    serde_json::to_string_pretty(&result).map_err(opt_to_py)
}

/// Optimize a portfolio to maximize value-weighted YTM with a CCC exposure cap.
///
/// A convenience wrapper around the general optimizer for a common
/// fixed-income use case.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioSpec``.
/// market_json : str
///     JSON-serialized ``MarketContext``.
/// ccc_limit : float
///     Maximum allowable weight in positions tagged ``rating="CCC"``,
///     expressed as a fraction in ``[0, 1]``.
/// strict_risk : bool
///     If ``True``, fail when any required risk metric is missing.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``MaxYieldWithCccLimitResult``.
#[pyfunction]
#[pyo3(signature = (spec_json, market_json, ccc_limit=0.10, strict_risk=false))]
fn optimize_max_yield(
    spec_json: &str,
    market_json: &str,
    ccc_limit: f64,
    strict_risk: bool,
) -> PyResult<String> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(opt_to_py)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(opt_to_py)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(opt_to_py)?;
    let config = finstack_core::config::FinstackConfig::default();
    let result = finstack_portfolio::optimize_max_yield_with_ccc_limit(
        &portfolio,
        &market,
        &config,
        ccc_limit,
        strict_risk,
    )
    .map_err(opt_to_py)?;
    serde_json::to_string_pretty(&result).map_err(opt_to_py)
}

/// Register optimization functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(optimize_portfolio, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(optimize_max_yield, m)?)?;
    Ok(())
}
