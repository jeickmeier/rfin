//! Python bindings for portfolio scenario integration.

use crate::core::config::extract_config_or_default;
use crate::core::market_data::context::PyMarketContext;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::positions::{extract_portfolio, PyPortfolio};
use crate::portfolio::valuation::PyPortfolioValuation;
use crate::scenarios::reports::PyApplicationReport;
use crate::scenarios::spec::PyScenarioSpec;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};
use pyo3::Bound;

use finstack_portfolio::scenarios::{apply_and_revalue, apply_scenario};

/// Apply a scenario to a portfolio.
///
/// Transforms the portfolio by applying scenario operations. The original portfolio
/// is not modified; a new portfolio with transformed positions is returned along
/// with the stressed market context and an application report.
///
/// Args:
///     portfolio: Portfolio to transform.
///     scenario: Scenario specification to apply.
///     market_context: Market data context.
///
/// Returns:
///     tuple[Portfolio, MarketContext, ApplicationReport]: Transformed portfolio,
///         stressed market context, and application report.
///
/// Raises:
///     RuntimeError: If scenario application fails.
///
/// Examples:
///     >>> from finstack.portfolio import apply_scenario
///     >>> from finstack.scenarios import ScenarioSpec
///     >>> portfolio, market, report = apply_scenario(portfolio, scenario, market_context)
#[pyfunction]
#[pyo3(signature = (portfolio, scenario, market_context))]
fn py_apply_scenario(
    portfolio: &Bound<'_, PyAny>,
    scenario: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
) -> PyResult<(PyPortfolio, PyMarketContext, PyApplicationReport)> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let scenario_inner = scenario.extract::<PyRef<PyScenarioSpec>>()?.inner.clone();
    let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;

    let (transformed, stressed_market, report) =
        apply_scenario(&portfolio_inner, &scenario_inner, &market_ctx.inner)
            .map_err(portfolio_to_py)?;

    Ok((
        PyPortfolio::new(transformed),
        PyMarketContext {
            inner: stressed_market,
        },
        PyApplicationReport::new(report),
    ))
}

/// Apply a scenario to a portfolio and revalue it.
///
/// Convenience function that applies a scenario and then values the resulting portfolio.
/// Equivalent to calling apply_scenario followed by value_portfolio.
///
/// Args:
///     portfolio: Portfolio to transform and value.
///     scenario: Scenario specification to apply.
///     market_context: Market data context.
///     config: Finstack configuration (optional, uses default if not provided).
///
/// Returns:
///     tuple[PortfolioValuation, ApplicationReport]: Portfolio valuation results
///         and application report.
///
/// Raises:
///     RuntimeError: If scenario application or valuation fails.
///
/// Examples:
///     >>> from finstack.portfolio import apply_and_revalue
///     >>> from finstack.scenarios import ScenarioSpec
///     >>> valuation, report = apply_and_revalue(portfolio, scenario, market_context)
///     >>> valuation.total_base_ccy
///     Money(USD, 9500000.0)
#[pyfunction]
#[pyo3(signature = (portfolio, scenario, market_context, config=None))]
fn py_apply_and_revalue(
    portfolio: &Bound<'_, PyAny>,
    scenario: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
    config: Option<&Bound<'_, PyAny>>,
) -> PyResult<(PyPortfolioValuation, PyApplicationReport)> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let scenario_inner = scenario.extract::<PyRef<PyScenarioSpec>>()?.inner.clone();
    let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;

    let cfg = extract_config_or_default(config)?;

    let (valuation, report) =
        apply_and_revalue(&portfolio_inner, &scenario_inner, &market_ctx.inner, &cfg)
            .map_err(portfolio_to_py)?;

    Ok((
        PyPortfolioValuation::new(valuation),
        PyApplicationReport::new(report),
    ))
}

/// Register scenarios module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    let wrapped_apply = wrap_pyfunction!(py_apply_scenario, parent)?;
    parent.add("apply_scenario", wrapped_apply)?;

    let wrapped_revalue = wrap_pyfunction!(py_apply_and_revalue, parent)?;
    parent.add("apply_and_revalue", wrapped_revalue)?;

    Ok(vec![
        "apply_scenario".to_string(),
        "apply_and_revalue".to_string(),
    ])
}
