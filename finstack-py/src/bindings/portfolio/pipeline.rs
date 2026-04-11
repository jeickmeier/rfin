//! End-to-end portfolio pipeline functions.
//!
//! Each function takes a portfolio spec + market context as JSON, builds the
//! runtime portfolio internally, performs the computation, and returns JSON.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn port_to_py(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Value a portfolio from its spec and market context.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioSpec``.
/// market_json : str
///     JSON-serialized ``MarketContext``.
/// strict_risk : bool
///     If ``True``, any risk metric failure aborts the entire valuation.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``PortfolioValuation``.
#[pyfunction]
#[pyo3(signature = (spec_json, market_json, strict_risk=false))]
fn value_portfolio(spec_json: &str, market_json: &str, strict_risk: bool) -> PyResult<String> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(port_to_py)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(port_to_py)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(port_to_py)?;
    let config = finstack_core::config::FinstackConfig::default();
    let options = finstack_portfolio::PortfolioValuationOptions {
        strict_risk,
        ..Default::default()
    };
    let valuation = finstack_portfolio::value_portfolio(&portfolio, &market, &config, &options)
        .map_err(port_to_py)?;
    serde_json::to_string(&valuation).map_err(port_to_py)
}

/// Aggregate cashflows for a portfolio from its spec and market context.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioSpec``.
/// market_json : str
///     JSON-serialized ``MarketContext``.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``PortfolioCashflows`` ladder.
#[pyfunction]
fn aggregate_cashflows(spec_json: &str, market_json: &str) -> PyResult<String> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(port_to_py)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(port_to_py)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(port_to_py)?;
    let cashflows =
        finstack_portfolio::aggregate_cashflows(&portfolio, &market).map_err(port_to_py)?;
    serde_json::to_string(&cashflows).map_err(port_to_py)
}

/// Apply a scenario to a portfolio and revalue it.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioSpec``.
/// scenario_json : str
///     JSON-serialized ``ScenarioSpec``.
/// market_json : str
///     JSON-serialized ``MarketContext``.
///
/// Returns
/// -------
/// tuple[str, str]
///     (valuation_json, report_json) — the revalued portfolio and
///     the scenario application report.
#[pyfunction]
fn apply_scenario_and_revalue(
    spec_json: &str,
    scenario_json: &str,
    market_json: &str,
) -> PyResult<(String, String)> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(port_to_py)?;
    let scenario: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(port_to_py)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(port_to_py)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(port_to_py)?;
    let config = finstack_core::config::FinstackConfig::default();
    let (valuation, report) =
        finstack_portfolio::apply_and_revalue(&portfolio, &scenario, &market, &config)
            .map_err(port_to_py)?;
    let val_json = serde_json::to_string(&valuation).map_err(port_to_py)?;
    let report_json = serde_json::to_string(&report).map_err(port_to_py)?;
    Ok((val_json, report_json))
}

/// Register pipeline functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(value_portfolio, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(aggregate_cashflows, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(apply_scenario_and_revalue, m)?)?;
    Ok(())
}
