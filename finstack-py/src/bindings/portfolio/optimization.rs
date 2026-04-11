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

/// Register optimization functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(optimize_portfolio, m)?)?;
    Ok(())
}
