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

    /// Raw metrics by position (all metrics), with explicit native currency context.
    pub by_position: IndexMap<PositionId, PositionMetrics>,
}

/// Position-level metrics with explicit native currency context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionMetrics {
    /// Native currency for this position's valuation and non-summable metrics.
    pub currency: Currency,
    /// Raw metric values for the position.
    pub metrics: IndexMap<String, f64>,
}

impl std::ops::Deref for PositionMetrics {
    type Target = IndexMap<String, f64>;

    fn deref(&self) -> &Self::Target {
        &self.metrics
    }
}

impl<'a> IntoIterator for &'a PositionMetrics {
    type Item = (&'a String, &'a f64);
    type IntoIter = indexmap::map::Iter<'a, String, f64>;

    fn into_iter(self) -> Self::IntoIter {
        self.metrics.iter()
    }
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
    pub fn get_position_metrics(&self, position_id: &str) -> Option<&PositionMetrics> {
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
///
/// # Returns
///
/// `true` when the metric is safe to sum across positions after any required
/// FX conversion.
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
/// * `as_of` - Valuation date used for FX rate lookups.
///
/// # Returns
///
/// [`Result`] with a populated [`PortfolioMetrics`] structure.
///
/// # Parallelism
///
/// When the `parallel` feature is enabled, metrics are collected in parallel
/// and then deterministically aggregated to ensure consistency across runs.
///
/// # References
///
/// - Fixed-income risk conventions:
///   `docs/REFERENCES.md#tuckman-serrat-fixed-income`
/// - Numerically stable aggregation:
///   `docs/REFERENCES.md#kahan-1965`
pub fn aggregate_metrics(
    valuation: &PortfolioValuation,
    base_ccy: Currency,
    market: &MarketContext,
    as_of: finstack_core::dates::Date,
) -> Result<PortfolioMetrics> {
    #[cfg(feature = "parallel")]
    {
        aggregate_metrics_parallel(valuation, base_ccy, market, as_of)
    }

    #[cfg(not(feature = "parallel"))]
    {
        aggregate_metrics_serial(valuation, base_ccy, market, as_of)
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
) -> Result<f64> {
    let native_ccy = position_value.value_native.currency();

    if native_ccy == base_ccy {
        return Ok(1.0);
    }

    let native_amount = position_value.value_native.amount();
    if native_amount.abs() > 1e-12 {
        let base_amount = position_value.value_base.amount();
        return Ok(base_amount / native_amount);
    }

    let fx_matrix = market.fx().ok_or_else(|| {
        crate::error::Error::MissingMarketData(
            "FX matrix not available for metric FX conversion".to_string(),
        )
    })?;
    let query = FxQuery::new(native_ccy, base_ccy, as_of);
    let rate_result =
        fx_matrix
            .rate(query)
            .map_err(|_| crate::error::Error::FxConversionFailed {
                from: native_ccy,
                to: base_ccy,
            })?;
    Ok(rate_result.rate)
}

fn scale_position_metric(metric_id: &str, value: f64, metric_scale: f64) -> f64 {
    if is_summable(metric_id) {
        value * metric_scale
    } else {
        value
    }
}

/// Serial implementation of metrics aggregation.
#[cfg(not(feature = "parallel"))]
fn aggregate_metrics_serial(
    valuation: &PortfolioValuation,
    base_ccy: Currency,
    market: &MarketContext,
    as_of: finstack_core::dates::Date,
) -> Result<PortfolioMetrics> {
    let mut collected = Vec::new();

    for (position_id, position_value) in &valuation.position_values {
        if let Some(val_result) = &position_value.valuation_result {
            let metrics: IndexMap<String, f64> = val_result
                .measures
                .iter()
                .map(|(id, v)| {
                    let metric_id = id.as_str().to_string();
                    let scaled = scale_position_metric(&metric_id, *v, position_value.metric_scale);
                    (metric_id, scaled)
                })
                .collect();
            let fx_rate = fx_rate_for_position(position_value, base_ccy, market, as_of)?;
            collected.push(PositionMetricData {
                position_id: position_id.clone(),
                entity_id: position_value.entity_id.clone(),
                currency: position_value.value_native.currency(),
                metrics,
                fx_rate,
            });
        }
    }

    Ok(aggregate_collected_metrics(collected))
}

/// Parallel implementation of metrics aggregation.
#[cfg(feature = "parallel")]
fn aggregate_metrics_parallel(
    valuation: &PortfolioValuation,
    base_ccy: Currency,
    market: &MarketContext,
    as_of: finstack_core::dates::Date,
) -> Result<PortfolioMetrics> {
    use rayon::prelude::*;

    let position_entries: Vec<_> = valuation.position_values.iter().collect();

    let collected: Vec<PositionMetricData> = position_entries
        .par_iter()
        .filter_map(|(position_id, position_value)| {
            position_value.valuation_result.as_ref().map(|val_result| {
                let metrics: IndexMap<String, f64> = val_result
                    .measures
                    .iter()
                    .map(|(id, v)| {
                        let metric_id = id.as_str().to_string();
                        let scaled =
                            scale_position_metric(&metric_id, *v, position_value.metric_scale);
                        (metric_id, scaled)
                    })
                    .collect();
                let fx_rate = fx_rate_for_position(position_value, base_ccy, market, as_of)?;
                Ok(PositionMetricData {
                    position_id: (*position_id).clone(),
                    entity_id: position_value.entity_id.clone(),
                    currency: position_value.value_native.currency(),
                    metrics,
                    fx_rate,
                })
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(aggregate_collected_metrics(collected))
}

/// Collected per-position metric data ready for aggregation.
struct PositionMetricData {
    position_id: PositionId,
    entity_id: EntityId,
    currency: Currency,
    metrics: IndexMap<String, f64>,
    fx_rate: f64,
}

/// Aggregate collected position metric data into portfolio metrics.
///
/// This is the shared Phase 2+3 logic used by both serial and parallel implementations.
fn aggregate_collected_metrics(collected: Vec<PositionMetricData>) -> PortfolioMetrics {
    let mut by_position: IndexMap<PositionId, PositionMetrics> = IndexMap::new();
    let mut metric_values: IndexMap<String, Vec<f64>> = IndexMap::new();
    let mut entity_values: IndexMap<String, IndexMap<EntityId, Vec<f64>>> = IndexMap::new();

    for data in &collected {
        by_position.insert(
            data.position_id.clone(),
            PositionMetrics {
                currency: data.currency,
                metrics: data.metrics.clone(),
            },
        );

        for (metric_id, value) in &data.metrics {
            if is_summable(metric_id) {
                if !value.is_finite() {
                    tracing::warn!(
                        metric_id = %metric_id,
                        position_id = %data.position_id,
                        value,
                        "Skipping non-finite metric value"
                    );
                    continue;
                }

                let value_base = *value * data.fx_rate;

                metric_values
                    .entry(metric_id.clone())
                    .or_default()
                    .push(value_base);

                entity_values
                    .entry(metric_id.clone())
                    .or_default()
                    .entry(data.entity_id.clone())
                    .or_default()
                    .push(value_base);
            }
        }
    }

    let mut aggregated: IndexMap<String, AggregatedMetric> = IndexMap::new();

    for (metric_id, values) in metric_values {
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

    PortfolioMetrics {
        aggregated,
        by_position,
    }
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
            .start_date(as_of)
            .maturity(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .quote_rate_opt(Some(
                rust_decimal::Decimal::try_from(0.045).expect("valid literal"),
            ))
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

        let valuation = value_portfolio(&portfolio, &market, &config, &Default::default())
            .expect("test should succeed");
        let metrics = aggregate_metrics(&valuation, Currency::USD, &market, as_of)
            .expect("test should succeed");

        // Should have position-level metrics
        assert_eq!(valuation.position_values.len(), 1);
        assert!(
            !metrics.by_position.is_empty(),
            "Should have position metrics"
        );
        let position_metrics = metrics
            .get_position_metrics("POS_001")
            .expect("position metrics should be present");
        assert_eq!(position_metrics.currency, Currency::USD);
    }
}
