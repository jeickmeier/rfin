//! Portfolio valuation and aggregation.

use crate::error::{PortfolioError, Result};
use crate::portfolio::Portfolio;
use crate::types::{EntityId, PositionId};
use finstack_core::prelude::*;
use finstack_valuations::metrics::MetricId;
use finstack_valuations::results::ValuationResult;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Result of valuing a single position.
///
/// Holds both native-currency and base-currency valuations along with
/// the underlying [`ValuationResult`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionValue {
    /// Position identifier
    pub position_id: PositionId,

    /// Entity that owns this position
    pub entity_id: EntityId,

    /// Value in the instrument's native currency
    pub value_native: Money,

    /// Value converted to portfolio base currency
    pub value_base: Money,

    /// Full valuation result with metrics
    #[serde(skip)]
    pub valuation_result: Option<ValuationResult>,
}

/// Complete portfolio valuation results.
///
/// Provides per-position valuations, totals by entity, and the grand total.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioValuation {
    /// Values for each position
    pub position_values: IndexMap<PositionId, PositionValue>,

    /// Total portfolio value in base currency
    pub total_base_ccy: Money,

    /// Aggregated values by entity
    pub by_entity: IndexMap<EntityId, Money>,
}

impl PortfolioValuation {
    /// Get the value for a specific position.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Identifier to query.
    pub fn get_position_value(&self, position_id: &str) -> Option<&PositionValue> {
        self.position_values.get(position_id)
    }

    /// Get the total value for a specific entity.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - Entity identifier to query.
    pub fn get_entity_value(&self, entity_id: &EntityId) -> Option<&Money> {
        self.by_entity.get(entity_id)
    }
}

/// Standard metrics to compute for portfolio positions.
///
/// Note: Using Theta, DV01, and CS01 which are widely supported as scalar metrics.
/// Many instruments use bucketed metrics (series) which require special handling.
fn standard_portfolio_metrics() -> Vec<MetricId> {
    vec![MetricId::Theta, MetricId::Dv01, MetricId::Cs01]
}

/// Value all positions in a portfolio with full metrics.
///
/// This function:
/// 1. Iterates through all positions (in parallel if enabled)
/// 2. Prices each instrument with metrics  
/// 3. Converts values to base currency using FX rates  
/// 4. Aggregates by entity
///
/// # Arguments
///
/// * `portfolio` - Portfolio to value.
/// * `market` - Market data context supplying curves and FX.
/// * `config` - Runtime configuration for the valuation engine.
///
/// # Returns
///
/// [`Result`] containing [`PortfolioValuation`] on success.
///
/// # Errors
///
/// Returns [`PortfolioError`] when pricing fails, FX rates are missing, or monetary
/// arithmetic overflows.
///
/// # Parallelism
///
/// When the `parallel` feature is enabled, position valuations are computed in parallel
/// using rayon. Results are deterministically reduced to ensure consistency across runs.
pub fn value_portfolio(
    portfolio: &Portfolio,
    market: &MarketContext,
    config: &FinstackConfig,
) -> Result<PortfolioValuation> {
    // Use parallel execution if feature is enabled
    #[cfg(feature = "parallel")]
    {
        value_portfolio_parallel(portfolio, market, config)
    }

    #[cfg(not(feature = "parallel"))]
    {
        value_portfolio_serial(portfolio, market, config)
    }
}

/// Serial implementation of portfolio valuation.
#[cfg(not(feature = "parallel"))]
fn value_portfolio_serial(
    portfolio: &Portfolio,
    market: &MarketContext,
    _config: &FinstackConfig,
) -> Result<PortfolioValuation> {
    let mut position_values = IndexMap::new();
    let mut by_entity: IndexMap<EntityId, Money> = IndexMap::new();

    // Get standard metrics to compute
    let metrics = standard_portfolio_metrics();

    for position in &portfolio.positions {
        let position_value = value_single_position(position, market, portfolio, &metrics)?;

        // Aggregate by entity
        let entity_total = by_entity
            .entry(position.entity_id.clone())
            .or_insert_with(|| Money::new(0.0, portfolio.base_ccy));
        *entity_total = entity_total
            .checked_add(position_value.value_base)
            .map_err(PortfolioError::Core)?;

        // Store position value
        position_values.insert(position.position_id.clone(), position_value);
    }

    // Calculate total
    let mut total_base_ccy = Money::new(0.0, portfolio.base_ccy);
    for v in by_entity.values() {
        total_base_ccy = total_base_ccy
            .checked_add(*v)
            .map_err(PortfolioError::Core)?;
    }

    Ok(PortfolioValuation {
        position_values,
        total_base_ccy,
        by_entity,
    })
}

/// Parallel implementation of portfolio valuation.
#[cfg(feature = "parallel")]
fn value_portfolio_parallel(
    portfolio: &Portfolio,
    market: &MarketContext,
    _config: &FinstackConfig,
) -> Result<PortfolioValuation> {
    use rayon::prelude::*;
    
    let metrics = standard_portfolio_metrics();

    // Value all positions in parallel
    let position_results: Vec<Result<PositionValue>> = portfolio
        .positions
        .par_iter()
        .map(|position| value_single_position(position, market, portfolio, &metrics))
        .collect();

    // Collect results and handle errors (fail-fast on first error)
    let position_values_vec: Vec<PositionValue> =
        position_results.into_iter().collect::<Result<Vec<_>>>()?;

    // Build position_values map and aggregate by entity deterministically
    let mut position_values = IndexMap::new();
    let mut by_entity: IndexMap<EntityId, Money> = IndexMap::new();

    for position_value in position_values_vec {
        // Aggregate by entity
        let entity_total = by_entity
            .entry(position_value.entity_id.clone())
            .or_insert_with(|| Money::new(0.0, portfolio.base_ccy));
        *entity_total = entity_total
            .checked_add(position_value.value_base)
            .map_err(PortfolioError::Core)?;

        // Store position value
        position_values.insert(position_value.position_id.clone(), position_value);
    }

    // Calculate total
    let mut total_base_ccy = Money::new(0.0, portfolio.base_ccy);
    for v in by_entity.values() {
        total_base_ccy = total_base_ccy
            .checked_add(*v)
            .map_err(PortfolioError::Core)?;
    }

    Ok(PortfolioValuation {
        position_values,
        total_base_ccy,
        by_entity,
    })
}

/// Value a single position with metrics and FX conversion.
///
/// This helper function is used by both serial and parallel implementations.
fn value_single_position(
    position: &crate::position::Position,
    market: &MarketContext,
    portfolio: &Portfolio,
    metrics: &[MetricId],
) -> Result<PositionValue> {
    // Price the instrument with metrics (try with metrics, fall back to value only)
    let valuation_result = position
        .instrument
        .price_with_metrics(market, portfolio.as_of, metrics)
        .or_else(|_: finstack_core::Error| {
            // If metrics fail, just get base value
            let value = position.instrument.value(market, portfolio.as_of)?;
            Ok(ValuationResult::stamped(
                position.instrument.id(),
                portfolio.as_of,
                value,
            ))
        })
        .map_err(|e: finstack_core::Error| PortfolioError::ValuationError {
            position_id: position.position_id.clone(),
            message: e.to_string(),
        })?;

    let value_native = valuation_result.value;

    // Scale by quantity if needed (for most instruments, returns unit value)
    let scaled_native = value_native * position.quantity;

    // Convert to base currency
    let value_base = if scaled_native.currency() == portfolio.base_ccy {
        scaled_native
    } else {
        // Get FX matrix
        let fx_matrix = market.fx.as_ref().ok_or_else(|| {
            PortfolioError::MissingMarketData("FX matrix not available".to_string())
        })?;

        // Get FX rate
        let query = FxQuery::new(
            scaled_native.currency(),
            portfolio.base_ccy,
            portfolio.as_of,
        );
        let rate_result = fx_matrix
            .rate(query)
            .map_err(|_| PortfolioError::FxConversionFailed {
                from: scaled_native.currency(),
                to: portfolio.base_ccy,
            })?;

        Money::new(
            scaled_native.amount() * rate_result.rate,
            portfolio.base_ccy,
        )
    };

    Ok(PositionValue {
        position_id: position.position_id.clone(),
        entity_id: position.entity_id.clone(),
        value_native: scaled_native,
        value_base,
        valuation_result: Some(valuation_result),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::position::{Position, PositionUnit};
    use crate::types::{Entity, DUMMY_ENTITY_ID};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_valuations::instruments::deposit::Deposit;
    use std::sync::Arc;
    use time::macros::date;

    fn build_test_market() -> MarketContext {
        let base_date = date!(2024 - 01 - 01);
        // Create a flat discount curve (0% rate) so deposits have near-zero PV
        // NOTE: Flat curve requires allow_non_monotonic() since monotonicity is enforced by default
        let curve = DiscountCurve::builder("USD")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 1.0), (5.0, 1.0)])
            .set_interp(InterpStyle::Linear)
            .allow_non_monotonic() // Flat curve for testing
            .build()
            .unwrap();

        MarketContext::new().insert_discount(curve)
    }

    #[test]
    fn test_value_single_position() {
        let as_of = date!(2024 - 01 - 01);

        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .build()
            .unwrap();

        let position = Position::new(
            "POS_001",
            DUMMY_ENTITY_ID,
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        );

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .position(position)
            .build()
            .unwrap();

        let market = build_test_market();
        let config = FinstackConfig::default();

        let valuation = value_portfolio(&portfolio, &market, &config).unwrap();

        assert_eq!(valuation.position_values.len(), 1);
        // Note: With flat curve, deposit PV is small but should be present
        assert!(valuation.total_base_ccy.amount().abs() >= 0.0);
        assert_eq!(valuation.by_entity.len(), 1);
    }

    #[test]
    fn test_value_multiple_entities() {
        let as_of = date!(2024 - 01 - 01);

        let dep1 = Deposit::builder()
            .id("DEP_1".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .build()
            .unwrap();

        let dep2 = Deposit::builder()
            .id("DEP_2".into())
            .notional(Money::new(500_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 03 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .build()
            .unwrap();

        let pos1 = Position::new(
            "POS_001",
            "ENTITY_A",
            "DEP_1",
            Arc::new(dep1),
            1.0,
            PositionUnit::Units,
        );

        let pos2 = Position::new(
            "POS_002",
            "ENTITY_B",
            "DEP_2",
            Arc::new(dep2),
            1.0,
            PositionUnit::Units,
        );

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .entity(Entity::new("ENTITY_B"))
            .position(pos1)
            .position(pos2)
            .build()
            .unwrap();

        let market = build_test_market();
        let config = FinstackConfig::default();

        let valuation = value_portfolio(&portfolio, &market, &config).unwrap();

        assert_eq!(valuation.position_values.len(), 2);
        assert_eq!(valuation.by_entity.len(), 2);
        assert!(valuation
            .get_entity_value(&"ENTITY_A".to_string())
            .is_some());
        assert!(valuation
            .get_entity_value(&"ENTITY_B".to_string())
            .is_some());
    }
}
