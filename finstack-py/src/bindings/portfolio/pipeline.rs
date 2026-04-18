//! End-to-end portfolio pipeline functions.
//!
//! Each function takes a portfolio spec + market context as JSON, builds the
//! runtime portfolio internally, performs the computation, and returns JSON.

use crate::bindings::extract::extract_market;
use crate::errors::display_to_py;
use pyo3::prelude::*;

/// Value a portfolio from its spec and market context.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioSpec``.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// strict_risk : bool
///     If ``True``, any risk metric failure aborts the entire valuation.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``PortfolioValuation``.
#[pyfunction]
#[pyo3(signature = (spec_json, market, strict_risk=false))]
fn value_portfolio(
    spec_json: &str,
    market: &Bound<'_, PyAny>,
    strict_risk: bool,
) -> PyResult<String> {
    let spec: finstack_portfolio::portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(display_to_py)?;
    let market = extract_market(market)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(display_to_py)?;
    let config = finstack_core::config::FinstackConfig::default();
    let options = finstack_portfolio::valuation::PortfolioValuationOptions {
        strict_risk,
        ..Default::default()
    };
    let valuation =
        finstack_portfolio::valuation::value_portfolio(&portfolio, &market, &config, &options)
            .map_err(display_to_py)?;
    serde_json::to_string(&valuation).map_err(display_to_py)
}

/// Aggregate cashflows for a portfolio from its spec and market context.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioSpec``.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``PortfolioCashflows`` ladder.
#[pyfunction]
fn aggregate_cashflows(spec_json: &str, market: &Bound<'_, PyAny>) -> PyResult<String> {
    let spec: finstack_portfolio::portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(display_to_py)?;
    let market = extract_market(market)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(display_to_py)?;
    let cashflows = finstack_portfolio::cashflows::aggregate_cashflows(&portfolio, &market)
        .map_err(display_to_py)?;
    serde_json::to_string(&cashflows).map_err(display_to_py)
}

/// Apply a scenario to a portfolio and revalue it.
///
/// Parameters
/// ----------
/// spec_json : str
///     JSON-serialized ``PortfolioSpec``.
/// scenario_json : str
///     JSON-serialized ``ScenarioSpec``.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
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
    market: &Bound<'_, PyAny>,
) -> PyResult<(String, String)> {
    let spec: finstack_portfolio::portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(display_to_py)?;
    let scenario: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(display_to_py)?;
    let market = extract_market(market)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(display_to_py)?;
    let config = finstack_core::config::FinstackConfig::default();
    let (valuation, report) =
        finstack_portfolio::scenarios::apply_and_revalue(&portfolio, &scenario, &market, &config)
            .map_err(display_to_py)?;
    let val_json = serde_json::to_string(&valuation).map_err(display_to_py)?;
    let report_json = serde_json::to_string(&report).map_err(display_to_py)?;
    Ok((val_json, report_json))
}

/// Register pipeline functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(value_portfolio, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(aggregate_cashflows, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(apply_scenario_and_revalue, m)?)?;
    Ok(())
}
