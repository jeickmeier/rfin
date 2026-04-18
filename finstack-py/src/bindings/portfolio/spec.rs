//! JSON round-trip helpers for portfolio specs and results.

use crate::errors::display_to_py;
use pyo3::prelude::*;

/// Parse a portfolio specification from JSON.
#[pyfunction]
pub fn parse_portfolio_spec(json_str: &str) -> PyResult<String> {
    let spec: finstack_portfolio::portfolio::PortfolioSpec =
        serde_json::from_str(json_str).map_err(display_to_py)?;
    serde_json::to_string(&spec).map_err(display_to_py)
}

/// Build a runtime portfolio from a JSON spec and round-trip the spec.
#[pyfunction]
pub fn build_portfolio_from_spec(spec_json: &str) -> PyResult<String> {
    let spec: finstack_portfolio::portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(display_to_py)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(display_to_py)?;
    let round_tripped = portfolio.to_spec();
    serde_json::to_string(&round_tripped).map_err(display_to_py)
}

/// Extract total portfolio value from a ``PortfolioResult`` JSON.
#[pyfunction]
pub fn portfolio_result_total_value(result_json: &str) -> PyResult<f64> {
    let result: finstack_portfolio::results::PortfolioResult =
        serde_json::from_str(result_json).map_err(display_to_py)?;
    Ok(result.total_value().amount())
}

/// Extract a specific metric from a ``PortfolioResult`` JSON.
#[pyfunction]
pub fn portfolio_result_get_metric(result_json: &str, metric_id: &str) -> PyResult<Option<f64>> {
    let result: finstack_portfolio::results::PortfolioResult =
        serde_json::from_str(result_json).map_err(display_to_py)?;
    Ok(result.get_metric(metric_id))
}

/// Aggregate portfolio metrics from a valuation JSON.
///
/// Parameters
/// ----------
/// valuation_json : str
///     JSON-serialized ``PortfolioValuation``.
/// base_ccy : str
///     Base currency code.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// as_of : str
///     Valuation date in ISO 8601 format.
#[pyfunction]
pub fn aggregate_metrics(
    valuation_json: &str,
    base_ccy: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
) -> PyResult<String> {
    use crate::bindings::extract::extract_market;
    let valuation: finstack_portfolio::valuation::PortfolioValuation =
        serde_json::from_str(valuation_json).map_err(display_to_py)?;
    let ccy: finstack_core::currency::Currency = base_ccy.parse().map_err(display_to_py)?;
    let market = extract_market(market)?;
    let date = super::parse_date(as_of)?;
    let metrics = finstack_portfolio::metrics::aggregate_metrics(&valuation, ccy, &market, date)
        .map_err(display_to_py)?;
    serde_json::to_string(&metrics).map_err(display_to_py)
}

/// Register spec functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(parse_portfolio_spec, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(build_portfolio_from_spec, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(portfolio_result_total_value, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(portfolio_result_get_metric, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(aggregate_metrics, m)?)?;
    Ok(())
}
