//! Portfolio metrics aggregation.
//!
//! Provides utilities to determine which valuation metrics are summable and
//! to consolidate per-position measures into portfolio-level analytics.

use crate::error::Result;
use crate::types::{EntityId, PositionId};
use crate::valuation::PortfolioValuation;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Aggregated metric across the portfolio.
///
/// Contains portfolio-wide totals as well as breakdowns by entity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregatedMetric {
    /// Metric identifier
    pub metric_id: String,

    /// Total value across all positions (for summable metrics)
    pub total: f64,

    /// Aggregated values by entity
    pub by_entity: IndexMap<EntityId, f64>,
}

/// Complete portfolio metrics results.
///
/// Holds both aggregated metrics and per-position values returned
/// by [`aggregate_metrics`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortfolioMetrics {
    /// Aggregated metrics (summable only)
    pub aggregated: IndexMap<String, AggregatedMetric>,

    /// Raw metrics by position (all metrics)
    pub by_position: IndexMap<PositionId, IndexMap<String, f64>>,
}

impl PortfolioMetrics {
    /// Get an aggregated metric by identifier.
    ///
    /// # Arguments
    ///
    /// * `metric_id` - Identifier of the metric to look up.
    pub fn get_metric(&self, metric_id: &str) -> Option<&AggregatedMetric> {
        self.aggregated.get(metric_id)
    }

    /// Get metrics for a specific position.
    ///
    /// # Arguments
    ///
    /// * `position_id` - Identifier of the position to query.
    pub fn get_position_metrics(&self, position_id: &str) -> Option<&IndexMap<String, f64>> {
        self.by_position.get(position_id)
    }

    /// Get the total value of a specific metric across the portfolio.
    ///
    /// # Arguments
    ///
    /// * `metric_id` - Identifier of the metric.
    pub fn get_total(&self, metric_id: &str) -> Option<f64> {
        self.aggregated.get(metric_id).map(|m| m.total)
    }
}

/// Metrics that can be meaningfully summed across positions.
///
/// These metrics scale linearly with position size and can be aggregated.
const SUMMABLE_METRICS: &[&str] = &[
    "theta",
    "dv01",
    "cs01",
    "delta",
    "gamma",
    "vega",
    "rho",
    "pv01",
    "ir01",
    "hazard_cs01",
    "index_delta",
    "bucketed_dv01",
    "bucketed_cs01",
    "accrued_interest",
    "pv_fixed",
    "pv_float",
    "pv_primary",
    "pv_reference",
    "duration_mac",
    "duration_mod",
    "spread_duration",
    "real_duration",
];

/// Check if a metric can be summed across positions.
///
/// # Arguments
///
/// * `metric_id` - Metric identifier to test.
pub fn is_summable(metric_id: &str) -> bool {
    SUMMABLE_METRICS.contains(&metric_id)
}

/// Aggregate metrics from portfolio valuation results.
///
/// This function:
/// 1. Collects all metrics from position valuations  
/// 2. Aggregates summable metrics by entity and portfolio total  
/// 3. Stores non-summable metrics by position only
///
/// # Arguments
///
/// * `valuation` - Portfolio valuation containing per-position valuation results.
///
/// # Returns
///
/// [`Result`] with a populated [`PortfolioMetrics`] structure.
pub fn aggregate_metrics(valuation: &PortfolioValuation) -> Result<PortfolioMetrics> {
    let mut by_position: IndexMap<PositionId, IndexMap<String, f64>> = IndexMap::new();
    let mut aggregated: IndexMap<String, AggregatedMetric> = IndexMap::new();

    // Phase 1: Collect metrics from each position
    for (position_id, position_value) in &valuation.position_values {
        if let Some(val_result) = &position_value.valuation_result {
            let metrics = val_result.measures.clone();
            by_position.insert(position_id.clone(), metrics.clone());

            // Phase 2: Aggregate summable metrics
            for (metric_id, value) in metrics {
                if is_summable(&metric_id) {
                    let agg =
                        aggregated
                            .entry(metric_id.clone())
                            .or_insert_with(|| AggregatedMetric {
                                metric_id: metric_id.clone(),
                                total: 0.0,
                                by_entity: IndexMap::new(),
                            });

                    // Add to total
                    agg.total += value;

                    // Add to entity
                    *agg.by_entity
                        .entry(position_value.entity_id.clone())
                        .or_insert(0.0) += value;
                }
            }
        }
    }

    Ok(PortfolioMetrics {
        aggregated,
        by_position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::position::{Position, PositionUnit};
    use crate::types::Entity;
    use crate::valuation::value_portfolio;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::prelude::*;
    use finstack_valuations::instruments::deposit::Deposit;
    use std::sync::Arc;
    use time::macros::date;

    fn build_test_market() -> MarketContext {
        let base_date = date!(2024 - 01 - 01);
        // Flat curve for testing - requires allow_non_monotonic()
        let curve = DiscountCurve::builder("USD")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 1.0), (5.0, 1.0)])
            .set_interp(InterpStyle::Linear)
            .allow_non_monotonic()
            .build()
            .unwrap();

        MarketContext::new().insert_discount(curve)
    }

    #[test]
    fn test_is_summable() {
        assert!(is_summable("dv01"));
        assert!(is_summable("cs01"));
        assert!(is_summable("delta"));
        assert!(!is_summable("ytm"));
        assert!(!is_summable("duration"));
    }

    #[test]
    fn test_aggregate_metrics_basic() {
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
            "ENTITY_A",
            "DEP_1M",
            Arc::new(deposit),
            1.0,
            PositionUnit::Units,
        );

        let portfolio = PortfolioBuilder::new("TEST")
            .base_ccy(Currency::USD)
            .as_of(as_of)
            .entity(Entity::new("ENTITY_A"))
            .position(position)
            .build()
            .unwrap();

        let market = build_test_market();
        let config = FinstackConfig::default();

        let valuation = value_portfolio(&portfolio, &market, &config).unwrap();
        let metrics = aggregate_metrics(&valuation).unwrap();

        // Should have position-level metrics
        assert_eq!(valuation.position_values.len(), 1);
        assert!(
            !metrics.by_position.is_empty(),
            "Should have position metrics"
        );
    }
}
