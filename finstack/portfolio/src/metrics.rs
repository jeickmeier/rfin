//! Portfolio metrics aggregation.
//!
//! Provides utilities to determine which valuation metrics are summable and
//! to consolidate per-position measures into portfolio-level analytics.
//!
//! # FX Conversion
//!
//! When aggregating risk metrics across a multi-currency portfolio, position-level
//! sensitivities must be converted to the portfolio's base currency before summation.
//! For example, a EUR position's DV01 (reported in EUR) and a USD position's DV01
//! (reported in USD) cannot be meaningfully summed without FX conversion.
//!
//! [`aggregate_metrics`] performs this conversion automatically using the FX rate
//! implied by each position's native and base-currency valuations. Positions with
//! zero native PV fall back to the FX matrix in the
//! [`finstack_core::market_data::context::MarketContext`].

use crate::error::Result;
use crate::types::{EntityId, PositionId};
use crate::valuation::PortfolioValuation;
use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::summation::neumaier_sum;
use finstack_core::money::fx::FxQuery;
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

impl Default for PortfolioMetrics {
    fn default() -> Self {
        Self {
            aggregated: IndexMap::new(),
            by_position: IndexMap::new(),
        }
    }
}

/// Metrics that can be meaningfully summed across positions.
///
/// These metrics scale linearly with position size and can be aggregated.
///
/// Bucketed metrics (e.g. key-rate DV01) are stored in `ValuationResult::measures`
/// using composite keys of the form:
///
/// - `bucketed_dv01::2y`
/// - `bucketed_cs01::5y`
///
/// To support portfolio-level aggregation of these series, `is_summable` performs
/// a prefix match on the base metric ID rather than requiring an exact key match.
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
];

/// Check if a metric can be summed across positions.
///
/// This treats both base IDs (e.g. `bucketed_dv01`) and structured
/// composite keys (e.g. `bucketed_dv01::2y`) as summable so that
/// key-rate / bucketed series aggregate correctly.
///
/// # Arguments
///
/// * `metric_id` - Metric identifier to test.
pub fn is_summable(metric_id: &str) -> bool {
    if SUMMABLE_METRICS.contains(&metric_id) {
        return true;
    }

    // Handle composite keys produced by `MetricContext::default_composite_key`,
    // which uses the pattern `base::label[::sub_label...]`.
    if let Some((base, _rest)) = metric_id.split_once("::") {
        return SUMMABLE_METRICS.contains(&base);
    }

    false
}

/// Aggregate metrics from portfolio valuation results.
///
/// This function:
/// 1. Collects all metrics from position valuations (in parallel if enabled)
/// 2. FX-converts summable metrics to portfolio base currency before aggregation
/// 3. Aggregates summable metrics by entity and portfolio total
/// 4. Stores non-summable metrics by position only (in native currency)
///
/// # FX Conversion
///
/// Position-level risk sensitivities are typically denominated in the instrument's
/// native currency. Before portfolio-level summation, each metric value is
/// multiplied by the implied FX rate from the position's native currency to the
/// portfolio base currency:
///
/// ```text
/// metric_base = metric_native × (value_base / value_native)
/// ```
///
/// For positions where the native PV is zero (or the position is already in
/// base currency), no conversion is applied.
///
/// # Arguments
///
/// * `valuation` - Portfolio valuation containing per-position valuation results.
/// * `base_ccy` - Portfolio base currency for aggregation.
/// * `market` - Market context providing FX rates for zero-PV positions.
///
/// # Returns
///
/// [`Result`] with a populated [`PortfolioMetrics`] structure.
///
/// # Parallelism
///
/// When the `parallel` feature is enabled, metrics are collected in parallel
/// and then deterministically aggregated to ensure consistency across runs.
pub fn aggregate_metrics(
    valuation: &PortfolioValuation,
    base_ccy: Currency,
    market: &MarketContext,
) -> Result<PortfolioMetrics> {
    // Use parallel execution if feature is enabled
    #[cfg(feature = "parallel")]
    {
        aggregate_metrics_parallel(valuation, base_ccy, market)
    }

    #[cfg(not(feature = "parallel"))]
    {
        aggregate_metrics_serial(valuation, base_ccy, market)
    }
}

/// Compute the FX conversion factor from a position's native currency to base currency.
///
/// Attempts to derive the rate from the position's valuation (value_base / value_native)
/// when both are non-zero. Falls back to the FX matrix for positions with zero PV.
fn fx_rate_for_position(
    position_value: &crate::valuation::PositionValue,
    base_ccy: Currency,
    market: &MarketContext,
    as_of: finstack_core::dates::Date,
) -> f64 {
    let native_ccy = position_value.value_native.currency();

    // Same currency: no conversion needed
    if native_ccy == base_ccy {
        return 1.0;
    }

    // Try to derive from existing valuation (avoids extra market lookup)
    let native_amount = position_value.value_native.amount();
    if native_amount.abs() > 1e-12 {
        let base_amount = position_value.value_base.amount();
        return base_amount / native_amount;
    }

    // Fall back to FX matrix for zero-PV positions
    if let Some(fx_matrix) = market.fx() {
        let query = FxQuery::new(native_ccy, base_ccy, as_of);
        if let Ok(rate_result) = fx_matrix.rate(query) {
            return rate_result.rate;
        }
    }

    // Last resort: log warning and return 1.0 (same-currency fallback)
    tracing::warn!(
        position_id = %position_value.position_id,
        native_ccy = %native_ccy,
        base_ccy = %base_ccy,
        "Unable to determine FX rate for metric conversion; using 1.0"
    );
    1.0
}

/// Serial implementation of metrics aggregation.
#[cfg(not(feature = "parallel"))]
fn aggregate_metrics_serial(
    valuation: &PortfolioValuation,
    base_ccy: Currency,
    market: &MarketContext,
) -> Result<PortfolioMetrics> {
    use crate::types::EntityId;

    // Derive as_of from the first position's valuation, or fall back to today.
    let as_of = derive_as_of(valuation);

    let mut by_position: IndexMap<PositionId, IndexMap<String, f64>> = IndexMap::new();

    // Intermediate collection: metric_id -> (total_values, entity_id -> values)
    // Collecting all values first allows proper compensated summation
    let mut metric_values: IndexMap<String, Vec<f64>> = IndexMap::new();
    let mut entity_values: IndexMap<String, IndexMap<EntityId, Vec<f64>>> = IndexMap::new();

    // Phase 1: Collect metrics from each position
    for (position_id, position_value) in &valuation.position_values {
        if let Some(val_result) = &position_value.valuation_result {
            let metrics: IndexMap<String, f64> = val_result
                .measures
                .iter()
                .map(|(id, v)| (id.as_str().to_string(), *v))
                .collect();
            // Store native-currency metrics for per-position drill-down
            by_position.insert(position_id.clone(), metrics.clone());

            // FX rate for converting this position's metrics to base currency
            let fx_rate = fx_rate_for_position(position_value, base_ccy, market, as_of);

            // Phase 2: Collect summable metrics (FX-converted) for later aggregation
            for (metric_id, value) in metrics {
                if is_summable(&metric_id) {
                    // Skip non-finite values (NaN, Inf)
                    if !value.is_finite() {
                        tracing::warn!(
                            metric_id = %metric_id,
                            position_id = %position_id,
                            value,
                            "Skipping non-finite metric value"
                        );
                        continue;
                    }

                    let value_base = value * fx_rate;

                    // Collect for total
                    metric_values
                        .entry(metric_id.clone())
                        .or_default()
                        .push(value_base);

                    // Collect for entity
                    entity_values
                        .entry(metric_id)
                        .or_default()
                        .entry(position_value.entity_id.clone())
                        .or_default()
                        .push(value_base);
                }
            }
        }
    }

    // Phase 3: Apply compensated summation to collected values
    let mut aggregated: IndexMap<String, AggregatedMetric> = IndexMap::new();

    for (metric_id, values) in metric_values {
        // Apply neumaier_sum to all collected values at once
        let total = neumaier_sum(values.into_iter());

        let mut by_entity: IndexMap<EntityId, f64> = IndexMap::new();
        if let Some(entity_map) = entity_values.get(&metric_id) {
            for (entity_id, entity_vals) in entity_map {
                let entity_total = neumaier_sum(entity_vals.iter().copied());
                by_entity.insert(entity_id.clone(), entity_total);
            }
        }

        aggregated.insert(
            metric_id.clone(),
            AggregatedMetric {
                metric_id,
                total,
                by_entity,
            },
        );
    }

    Ok(PortfolioMetrics {
        aggregated,
        by_position,
    })
}

/// Parallel implementation of metrics aggregation.
#[cfg(feature = "parallel")]
fn aggregate_metrics_parallel(
    valuation: &PortfolioValuation,
    base_ccy: Currency,
    market: &MarketContext,
) -> Result<PortfolioMetrics> {
    use crate::types::EntityId;
    use rayon::prelude::*;

    let as_of = derive_as_of(valuation);

    // Phase 1: Collect metrics from each position in parallel
    // Convert to Vec for parallel iteration
    let position_entries: Vec<_> = valuation.position_values.iter().collect();

    let position_metrics: Vec<_> = position_entries
        .par_iter()
        .filter_map(|(position_id, position_value)| {
            position_value.valuation_result.as_ref().map(|val_result| {
                let measures: IndexMap<String, f64> = val_result
                    .measures
                    .iter()
                    .map(|(id, v)| (id.as_str().to_string(), *v))
                    .collect();
                let fx_rate = fx_rate_for_position(position_value, base_ccy, market, as_of);
                (
                    (*position_id).clone(),
                    position_value.entity_id.clone(),
                    measures,
                    fx_rate,
                )
            })
        })
        .collect();

    // Phase 2: Build by_position map and collect values for summable metrics
    let mut by_position: IndexMap<PositionId, IndexMap<String, f64>> = IndexMap::new();

    // Intermediate collection: metric_id -> (total_values, entity_id -> values)
    // Collecting all values first allows proper compensated summation
    let mut metric_values: IndexMap<String, Vec<f64>> = IndexMap::new();
    let mut entity_values: IndexMap<String, IndexMap<EntityId, Vec<f64>>> = IndexMap::new();

    for (position_id, entity_id, metrics, fx_rate) in position_metrics {
        let position_id_clone = position_id.clone();
        // Store native-currency metrics for per-position drill-down
        by_position.insert(position_id, metrics.clone());

        // Collect summable metrics (FX-converted) for later aggregation
        for (metric_id, value) in &metrics {
            if is_summable(metric_id) {
                // Skip non-finite values (NaN, Inf)
                if !value.is_finite() {
                    tracing::warn!(
                        metric_id = %metric_id,
                        position_id = %position_id_clone,
                        value,
                        "Skipping non-finite metric value"
                    );
                    continue;
                }

                let value_base = *value * fx_rate;

                // Collect for total
                metric_values
                    .entry(metric_id.clone())
                    .or_default()
                    .push(value_base);

                // Collect for entity
                entity_values
                    .entry(metric_id.clone())
                    .or_default()
                    .entry(entity_id.clone())
                    .or_default()
                    .push(value_base);
            }
        }
    }

    // Phase 3: Apply compensated summation to collected values
    let mut aggregated: IndexMap<String, AggregatedMetric> = IndexMap::new();

    for (metric_id, values) in metric_values {
        // Apply neumaier_sum to all collected values at once
        let total = neumaier_sum(values.into_iter());

        let mut by_entity: IndexMap<EntityId, f64> = IndexMap::new();
        if let Some(entity_map) = entity_values.get(&metric_id) {
            for (entity_id, entity_vals) in entity_map {
                let entity_total = neumaier_sum(entity_vals.iter().copied());
                by_entity.insert(entity_id.clone(), entity_total);
            }
        }

        aggregated.insert(
            metric_id.clone(),
            AggregatedMetric {
                metric_id,
                total,
                by_entity,
            },
        );
    }

    Ok(PortfolioMetrics {
        aggregated,
        by_position,
    })
}

/// Derive as_of date from the valuation results.
///
/// Looks at the first position's valuation result timestamp. Falls back
/// to the current date if no positions have been valued.
fn derive_as_of(valuation: &PortfolioValuation) -> finstack_core::dates::Date {
    for pv in valuation.position_values.values() {
        if let Some(ref vr) = pv.valuation_result {
            return vr.as_of;
        }
    }
    // Safe fallback: use current date (only hit for empty portfolios)
    time::OffsetDateTime::now_utc().date()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::builder::PortfolioBuilder;
    use crate::position::{Position, PositionUnit};
    use crate::test_utils::build_test_market;
    use crate::types::Entity;
    use crate::valuation::value_portfolio;
    use finstack_core::config::FinstackConfig;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use finstack_valuations::instruments::rates::deposit::Deposit;
    use std::sync::Arc;
    use time::macros::date;

    #[test]
    fn test_is_summable() {
        assert!(is_summable("dv01"));
        assert!(is_summable("cs01"));
        assert!(is_summable("delta"));
        assert!(!is_summable("ytm"));
        assert!(!is_summable("duration"));

        // Test bucketed/composite keys
        assert!(is_summable("bucketed_dv01::2y"));
        assert!(is_summable("bucketed_cs01::AAA::5y"));
        assert!(!is_summable("unknown::2y"));
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

        let valuation = value_portfolio(&portfolio, &market, &config).expect("test should succeed");
        let metrics =
            aggregate_metrics(&valuation, Currency::USD, &market).expect("test should succeed");

        // Should have position-level metrics
        assert_eq!(valuation.position_values.len(), 1);
        assert!(
            !metrics.by_position.is_empty(),
            "Should have position metrics"
        );
    }
}
