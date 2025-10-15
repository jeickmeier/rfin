//! Python bindings for portfolio scenario integration.

use crate::core::config::PyFinstackConfig;
use crate::core::market_data::context::PyMarketContext;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::portfolio::{extract_portfolio, PyPortfolio};
use crate::portfolio::valuation::PyPortfolioValuation;
use crate::scenarios::spec::PyScenarioSpec;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};
use pyo3::Bound;

#[cfg(feature = "scenarios")]
use finstack_portfolio::scenarios::{apply_and_revalue, apply_scenario};

/// Apply a scenario to a portfolio.
///
/// Transforms the portfolio by applying scenario operations. The original portfolio
/// is not modified; a new portfolio with transformed positions is returned.
///
/// Args:
///     portfolio: Portfolio to transform.
///     scenario: Scenario specification to apply.
///     market_context: Market data context.
///
/// Returns:
///     Portfolio: Transformed portfolio.
///
/// Raises:
///     RuntimeError: If scenario application fails.
///
/// Examples:
///     >>> from finstack.portfolio import apply_scenario
///     >>> from finstack.scenarios import ScenarioSpec
///     >>> transformed = apply_scenario(portfolio, scenario, market_context)
#[pyfunction]
#[pyo3(signature = (portfolio, scenario, market_context))]
#[cfg(feature = "scenarios")]
fn py_apply_scenario(
    portfolio: &Bound<'_, PyAny>,
    scenario: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
) -> PyResult<PyPortfolio> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let scenario_inner = scenario.extract::<PyRef<PyScenarioSpec>>()?.inner.clone();
    let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;

    let (transformed, _market, _report) = apply_scenario(&portfolio_inner, &scenario_inner, &market_ctx.inner)
        .map_err(portfolio_to_py)?;

    Ok(PyPortfolio::new(transformed))
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
///     PortfolioValuation: Portfolio valuation results.
///
/// Raises:
///     RuntimeError: If scenario application or valuation fails.
///
/// Examples:
///     >>> from finstack.portfolio import apply_and_revalue
///     >>> from finstack.scenarios import ScenarioSpec
///     >>> valuation = apply_and_revalue(portfolio, scenario, market_context)
///     >>> valuation.total_base_ccy
///     Money(USD, 9500000.0)
#[pyfunction]
#[pyo3(signature = (portfolio, scenario, market_context, config=None))]
#[cfg(feature = "scenarios")]
fn py_apply_and_revalue(
    portfolio: &Bound<'_, PyAny>,
    scenario: &Bound<'_, PyAny>,
    market_context: &Bound<'_, PyAny>,
    config: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyPortfolioValuation> {
    let portfolio_inner = extract_portfolio(portfolio)?;
    let scenario_inner = scenario.extract::<PyRef<PyScenarioSpec>>()?.inner.clone();
    let market_ctx = market_context.extract::<PyRef<PyMarketContext>>()?;

    let cfg = if let Some(config_obj) = config {
        config_obj.extract::<PyRef<PyFinstackConfig>>()?.inner.clone()
    } else {
        finstack_core::config::FinstackConfig::default()
    };

    let (valuation, _report) = apply_and_revalue(&portfolio_inner, &scenario_inner, &market_ctx.inner, &cfg)
        .map_err(portfolio_to_py)?;

    Ok(PyPortfolioValuation::new(valuation))
}

/// Register scenarios module exports.
pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    #[cfg(feature = "scenarios")]
    {
        let wrapped_apply = wrap_pyfunction!(py_apply_scenario, parent)?;
        parent.add("apply_scenario", wrapped_apply)?;
        
        let wrapped_revalue = wrap_pyfunction!(py_apply_and_revalue, parent)?;
        parent.add("apply_and_revalue", wrapped_revalue)?;
        
        Ok(vec![
            "apply_scenario".to_string(),
            "apply_and_revalue".to_string(),
        ])
    }

    #[cfg(not(feature = "scenarios"))]
    {
        Ok(vec![])
    }
}

