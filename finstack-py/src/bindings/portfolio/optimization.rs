//! Python bindings for portfolio optimization.
//!
//! Exposes the LP-based optimizer through JSON-friendly entry points that
//! follow the same pattern as the other portfolio pipeline functions.

use crate::bindings::extract::extract_market;
use crate::errors::display_to_py;
use pyo3::prelude::*;

/// Optimize a portfolio from a JSON specification.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioOptimizationSpec`` containing the portfolio,
///     objective, constraints, and weighting scheme.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``PortfolioOptimizationResultJson`` with optimal weights,
///     trade list, dual values, and diagnostics.
#[pyfunction]
fn optimize_portfolio(spec_json: &str, market: &Bound<'_, PyAny>) -> PyResult<String> {
    let spec: finstack_portfolio::PortfolioOptimizationSpec =
        serde_json::from_str(spec_json).map_err(display_to_py)?;
    let market = extract_market(market)?;
    let config = finstack_core::config::FinstackConfig::default();
    let result =
        finstack_portfolio::optimize_from_spec(&spec, &market, &config).map_err(display_to_py)?;
    serde_json::to_string_pretty(&result).map_err(display_to_py)
}

/// Register optimization functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(optimize_portfolio, m)?)?;
    Ok(())
}
