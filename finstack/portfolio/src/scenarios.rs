//! Scenario application for portfolios.
//!
//! This module is only available when the `scenarios` feature is enabled.
//! It provides helpers to apply scenario specifications to portfolios and
//! optionally re-value them using the modified market data.

#[cfg(feature = "scenarios")]
use crate::error::{PortfolioError, Result};
#[cfg(feature = "scenarios")]
use crate::portfolio::Portfolio;
#[cfg(feature = "scenarios")]
use finstack_core::market_data::context::MarketContext;
#[cfg(feature = "scenarios")]
use finstack_scenarios::engine::{ApplicationReport, ExecutionContext, ScenarioEngine};
#[cfg(feature = "scenarios")]
use finstack_scenarios::spec::ScenarioSpec;
#[cfg(feature = "scenarios")]
use finstack_statements::types::FinancialModelSpec;
#[cfg(feature = "scenarios")]
use finstack_valuations::instruments::Instrument;
#[cfg(feature = "scenarios")]
use std::sync::Arc;

/// Apply a scenario to a portfolio.
///
/// This function:
/// 1. Clones the portfolio (scenarios create modified copies)  
/// 2. Extracts instruments into a mutable vector for the scenario engine  
/// 3. Applies the scenario using the engine  
/// 4. Returns the modified portfolio and market data
///
/// The original portfolio and market are left untouched.
///
/// # Arguments
///
/// * `portfolio` - Portfolio to clone and mutate within the scenario engine.
/// * `scenario` - Scenario specification describing desired transformations.
/// * `market` - Market data context subject to the scenario operations.
///
/// # Returns
///
/// [`Result`] containing the modified portfolio, market, and application report.
///
/// # Errors
///
/// Returns [`PortfolioError::ScenarioError`] when the scenario engine reports a failure.
#[cfg(feature = "scenarios")]
pub fn apply_scenario(
    portfolio: &Portfolio,
    scenario: &ScenarioSpec,
    market: &MarketContext,
) -> Result<(Portfolio, MarketContext, ApplicationReport)> {
    let mut market_copy = market.clone();
    let mut portfolio_copy = portfolio.clone();

    // Extract instruments into a mutable vector
    let mut instruments: Vec<Box<dyn Instrument>> = portfolio_copy
        .positions
        .iter()
        .map(|pos| {
            // Clone the instrument via its Arc
            pos.instrument.clone_box()
        })
        .collect();

    // Create a dummy financial model for the execution context
    let mut model = FinancialModelSpec::new("portfolio_dummy", vec![]);

    // Build execution context
    let mut ctx = ExecutionContext {
        market: &mut market_copy,
        model: &mut model,
        instruments: Some(&mut instruments),
        rate_bindings: None,
        calendar: None,
        as_of: portfolio.as_of,
    };

    // Apply scenario
    let engine = ScenarioEngine::default();
    let report = engine
        .apply(scenario, &mut ctx)
        .map_err(|e| PortfolioError::ScenarioError(e.to_string()))?;

    // Update portfolio positions with modified instruments
    for (i, position) in portfolio_copy.positions.iter_mut().enumerate() {
        if let Some(modified_inst) = instruments.get(i) {
            position.instrument = Arc::from(modified_inst.clone_box());
        }
    }

    Ok((portfolio_copy, market_copy, report))
}

/// Apply a scenario and re-value the portfolio.
///
/// Convenience function that applies a scenario and immediately
/// re-values the portfolio with the modified market data.
///
/// # Arguments
///
/// * `portfolio` - Original portfolio used as the base case.
/// * `scenario` - Scenario specification to apply.
/// * `market` - Market data context to mutate.
/// * `config` - Configuration forwarded to [`value_portfolio`](crate::valuation::value_portfolio).
///
/// # Returns
///
/// [`Result`] containing the re-valued [`PortfolioValuation`](crate::valuation::PortfolioValuation)
/// along with the scenario [`ApplicationReport`].
///
/// # Errors
///
/// Propagates errors from [`apply_scenario`] and [`value_portfolio`](crate::valuation::value_portfolio).
#[cfg(feature = "scenarios")]
pub fn apply_and_revalue(
    portfolio: &Portfolio,
    scenario: &ScenarioSpec,
    market: &MarketContext,
    config: &finstack_core::config::FinstackConfig,
) -> Result<(crate::valuation::PortfolioValuation, ApplicationReport)> {
    let (modified_portfolio, modified_market, report) =
        apply_scenario(portfolio, scenario, market)?;

    let valuation =
        crate::valuation::value_portfolio(&modified_portfolio, &modified_market, config)?;

    Ok((valuation, report))
}

#[cfg(test)]
#[cfg(feature = "scenarios")]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::position::{Position, PositionUnit};
    use crate::test_utils::build_test_market;
    use crate::types::Entity;
    use finstack_core::config::FinstackConfig;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use finstack_scenarios::spec::{CurveKind, OperationSpec};
    use finstack_valuations::instruments::rates::deposit::Deposit;
    use std::sync::Arc;
    use time::macros::date;

    #[test]
    fn test_apply_scenario_basic() {
        let as_of = date!(2024 - 01 - 01);

        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .quote_rate_opt(Some(0.045))
            .build()
            .expect("test should succeed");

        let position = Position::new(
            "POS_001",
            "ENTITY_A",
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let market = build_test_market();

        let scenario = ScenarioSpec {
            id: "test_scenario".to_string(),
            name: Some("Test Scenario".to_string()),
            description: None,
            operations: vec![OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD".to_string(),
                bp: 50.0,
            }],
            priority: 0,
        };

        let result = apply_scenario(&portfolio, &scenario, &market);
        assert!(result.is_ok());

        let (_modified_portfolio, _modified_market, report) = result.expect("test should succeed");
        assert!(report.operations_applied > 0);
    }

    #[test]
    fn test_apply_and_revalue() {
        let as_of = date!(2024 - 01 - 01);

        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .quote_rate_opt(Some(0.045))
            .build()
            .expect("test should succeed");

        let position = Position::new(
            "POS_001",
            "ENTITY_A",
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .expect("test should succeed");

        let market = build_test_market();
        let config = FinstackConfig::default();

        let scenario = ScenarioSpec {
            id: "test_scenario".to_string(),
            name: None,
            description: None,
            operations: vec![],
            priority: 0,
        };

        let result = apply_and_revalue(&portfolio, &scenario, &market, &config);
        assert!(result.is_ok());
    }
}
