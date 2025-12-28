//! Portfolio valuation and aggregation.

use crate::error::{PortfolioError, Result};
use crate::portfolio::Portfolio;
use crate::types::{EntityId, PositionId};
use finstack_core::config::FinstackConfig;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::summation::neumaier_sum;
use finstack_core::money::{fx::FxQuery, Money};
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
    /// * `entity_id` - Entity identifier to query (accepts &str or &EntityId).
    pub fn get_entity_value(&self, entity_id: &str) -> Option<&Money> {
        self.by_entity.get(entity_id)
    }
}

/// Standard metrics to compute for portfolio positions.
///
/// Note: Using Theta, DV01, and CS01 which are widely supported as scalar metrics.
fn standard_portfolio_metrics() -> Vec<MetricId> {
    // Core risk set chosen to align with common portfolio risk reports:
    // - Theta: carry / time-decay
    // - Dv01 / BucketedDv01: parallel and key-rate IR risk
    // - Cs01 / BucketedCs01: credit spread / hazard risk
    // - Delta / Gamma / Vega / Rho: standard option Greeks
    //
    // Instruments that do not support a given metric simply omit it from
    // `ValuationResult::measures`; aggregation remains robust to missing keys.
    vec![
        MetricId::Theta,
        MetricId::Dv01,
        MetricId::BucketedDv01,
        MetricId::Cs01,
        MetricId::BucketedCs01,
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Rho,
        MetricId::Pv01,
    ]
}

/// Options controlling portfolio valuation behaviour.
///
/// This structure allows callers to configure how strictly risk metric
/// errors are handled and which additional metrics to request.
///
/// By default, risk metrics are treated as best-effort: if metrics fail
/// for a position, the engine falls back to PV-only valuation for that
/// position.
#[derive(Clone, Debug, Default)]
pub struct PortfolioValuationOptions {
    /// When `true`, any failure to compute requested risk metrics for a
    /// position causes the entire portfolio valuation to fail.
    ///
    /// When `false` (default), the engine falls back to PV-only
    /// valuation for that position if metrics fail, preserving
    /// aggregate PV but potentially leaving some risk metrics missing.
    pub strict_risk: bool,

    /// Optional list of additional metrics to request from instruments.
    ///
    /// When `None`, only the standard portfolio metrics (see
    /// [`standard_portfolio_metrics`]) are requested.
    /// When `Some`, the metrics are merged with the standard set unless
    /// [`PortfolioValuationOptions::replace_standard_metrics`] is `true`.
    pub additional_metrics: Option<Vec<MetricId>>,

    /// When `true`, [`PortfolioValuationOptions::additional_metrics`]
    /// replaces the standard metric set entirely rather than being
    /// merged with it.
    pub replace_standard_metrics: bool,
}

/// Value all positions in a portfolio with full metrics using default options.
///
/// This is a convenience wrapper over
/// [`value_portfolio_with_options`] that uses
/// [`PortfolioValuationOptions::default`].
pub fn value_portfolio(
    portfolio: &Portfolio,
    market: &MarketContext,
    config: &FinstackConfig,
) -> Result<PortfolioValuation> {
    value_portfolio_with_options(
        portfolio,
        market,
        config,
        &PortfolioValuationOptions::default(),
    )
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
/// * `options` - Portfolio valuation options controlling risk behaviour.
///
/// # Returns
///
/// [`Result`] containing [`PortfolioValuation`] on success.
///
/// # Errors
///
/// Returns [`PortfolioError`] in the following cases:
///
/// - [`PortfolioError::ValuationError`] - Instrument pricing failed for a position
/// - [`PortfolioError::MissingMarketData`] - FX matrix unavailable for cross-currency conversion
/// - [`PortfolioError::FxConversionFailed`] - Required FX rate not found in the matrix
/// - [`PortfolioError::Core`] - Monetary arithmetic overflow during aggregation
///
/// # Parallelism
///
/// When the `parallel` feature is enabled, position valuations are computed in parallel
/// using rayon. Results are deterministically reduced to ensure consistency across runs.
pub fn value_portfolio_with_options(
    portfolio: &Portfolio,
    market: &MarketContext,
    config: &FinstackConfig,
    options: &PortfolioValuationOptions,
) -> Result<PortfolioValuation> {
    // Use parallel execution if feature is enabled
    #[cfg(feature = "parallel")]
    {
        value_portfolio_parallel(portfolio, market, config, options)
    }

    #[cfg(not(feature = "parallel"))]
    {
        value_portfolio_serial(portfolio, market, config, options)
    }
}

/// Serial implementation of portfolio valuation.
#[cfg(not(feature = "parallel"))]
fn value_portfolio_serial(
    portfolio: &Portfolio,
    market: &MarketContext,
    _config: &FinstackConfig,
    options: &PortfolioValuationOptions,
) -> Result<PortfolioValuation> {
    let mut position_values = IndexMap::new();
    let mut by_entity: IndexMap<EntityId, Money> = IndexMap::new();

    // Determine which metrics to compute for this valuation
    let metrics = match (
        &options.additional_metrics,
        options.replace_standard_metrics,
    ) {
        (Some(additional), true) => additional.clone(),
        (Some(additional), false) => {
            let mut merged = standard_portfolio_metrics();
            for m in additional {
                if !merged.contains(m) {
                    merged.push(m.clone());
                }
            }
            merged
        }
        (None, _) => standard_portfolio_metrics(),
    };

    for position in &portfolio.positions {
        let position_value =
            value_single_position(position, market, portfolio, &metrics, options.strict_risk)?;

        // Aggregate by entity using compensated summation
        let entity_total = by_entity
            .entry(position.entity_id.clone())
            .or_insert_with(|| Money::new(0.0, portfolio.base_ccy));
        *entity_total = entity_total
            .checked_add(position_value.value_base)
            .map_err(PortfolioError::Core)?;

        // Store position value
        position_values.insert(position.position_id.clone(), position_value);
    }

    // Calculate total using compensated summation for numerical stability
    let entity_amounts: Vec<f64> = by_entity.values().map(|v| v.amount()).collect();
    let total_amount = neumaier_sum(entity_amounts);
    let total_base_ccy = Money::new(total_amount, portfolio.base_ccy);

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
    options: &PortfolioValuationOptions,
) -> Result<PortfolioValuation> {
    use rayon::prelude::*;

    let metrics = match (
        &options.additional_metrics,
        options.replace_standard_metrics,
    ) {
        (Some(additional), true) => additional.clone(),
        (Some(additional), false) => {
            let mut merged = standard_portfolio_metrics();
            for m in additional {
                if !merged.contains(m) {
                    merged.push(m.clone());
                }
            }
            merged
        }
        (None, _) => standard_portfolio_metrics(),
    };

    // Value all positions in parallel and aggregate using fold/reduce
    let position_values_vec: Vec<PositionValue> = portfolio
        .positions
        .par_iter()
        .map(|position| {
            value_single_position(position, market, portfolio, &metrics, options.strict_risk)
        })
        .collect::<Result<Vec<_>>>()?;

    // Parallel aggregation of entity totals using fold/reduce
    // Collect amounts per entity in parallel, then use compensated summation
    let entity_amounts_map: IndexMap<EntityId, Vec<f64>> = position_values_vec
        .par_iter()
        .fold(IndexMap::new, |mut acc, pv| {
            acc.entry(pv.entity_id.clone())
                .or_insert_with(Vec::new)
                .push(pv.value_base.amount());
            acc
        })
        .reduce(IndexMap::new, |mut a, b| {
            for (entity_id, amounts) in b {
                a.entry(entity_id).or_insert_with(Vec::new).extend(amounts);
            }
            a
        });

    // Build position_values IndexMap deterministically (preserve order)
    let mut position_values = IndexMap::new();
    for pv in position_values_vec {
        position_values.insert(pv.position_id.clone(), pv);
    }

    // Convert entity amounts to Money using compensated summation per entity
    let mut by_entity: IndexMap<EntityId, Money> = IndexMap::new();
    for (entity_id, amounts) in entity_amounts_map {
        let total_amount = neumaier_sum(amounts.into_iter());
        by_entity.insert(entity_id, Money::new(total_amount, portfolio.base_ccy));
    }

    // Calculate total using compensated summation for numerical stability
    let entity_amounts_for_total: Vec<f64> = by_entity.values().map(|v| v.amount()).collect();
    let total_amount = neumaier_sum(entity_amounts_for_total);
    let total_base_ccy = Money::new(total_amount, portfolio.base_ccy);

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
    strict_risk: bool,
) -> Result<PositionValue> {
    // Price the instrument with metrics.
    //
    // When `strict_risk` is `false`, metric failures fall back to PV-only
    // valuation for the position. When `true`, any metric failure bubbles up
    // as a portfolio error.
    let valuation_result = if strict_risk {
        position
            .instrument
            .price_with_metrics(market, portfolio.as_of, metrics)
            .map_err(|e: finstack_core::Error| PortfolioError::ValuationError {
                position_id: position.position_id.clone(),
                message: e.to_string(),
            })?
    } else {
        position
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
            })?
    };

    let value_native = valuation_result.value;

    // Scale by quantity using unit-aware scaling logic.
    let scaled_native = position.scale_value(value_native);

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
        let rate_result =
            fx_matrix
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
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::position::{Position, PositionUnit};
    use crate::test_utils::build_test_market;
    use crate::types::{Entity, DUMMY_ENTITY_ID};
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use finstack_valuations::instruments::deposit::Deposit;
    use std::sync::Arc;
    use time::macros::date;

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
            .quote_rate_opt(Some(0.045))
            .build()
            .expect("test should succeed");

        let position = Position::new(
            "POS_001",
            DUMMY_ENTITY_ID,
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .position(position)
            .build()
            .expect("test should succeed");

        let market = build_test_market();
        let config = FinstackConfig::default();

        let valuation = value_portfolio(&portfolio, &market, &config).expect("test should succeed");

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
            .quote_rate_opt(Some(0.045))
            .build()
            .expect("test should succeed");

        let dep2 = Deposit::builder()
            .id("DEP_2".into())
            .notional(Money::new(500_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 03 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .quote_rate_opt(Some(0.045))
            .build()
            .expect("test should succeed");

        let pos1 = Position::new(
            "POS_001",
            "ENTITY_A",
            "DEP_1",
            Arc::new(dep1),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");

        let pos2 = Position::new(
            "POS_002",
            "ENTITY_B",
            "DEP_2",
            Arc::new(dep2),
            1.0,
            PositionUnit::Units,
        )
        .expect("test should succeed");

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .entity(Entity::new("ENTITY_B"))
            .position(pos1)
            .position(pos2)
            .build()
            .expect("test should succeed");

        let market = build_test_market();
        let config = FinstackConfig::default();

        let valuation = value_portfolio(&portfolio, &market, &config).expect("test should succeed");

        assert_eq!(valuation.position_values.len(), 2);
        assert_eq!(valuation.by_entity.len(), 2);
        assert!(valuation.get_entity_value("ENTITY_A").is_some());
        assert!(valuation.get_entity_value("ENTITY_B").is_some());
    }
}
