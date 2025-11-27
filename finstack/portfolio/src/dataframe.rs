//! DataFrame exports for portfolio results.
//!
//! Functions in this module turn portfolio valuations and metrics into
//! [`polars::prelude::DataFrame`] objects that can be consumed by downstream
//! analytics pipelines or saved for offline processing.

use crate::metrics::PortfolioMetrics;
use crate::valuation::PortfolioValuation;
use finstack_core::prelude::*;

/// Export position values to a Polars `DataFrame`.
///
/// Columns produced:
/// `position_id`, `entity_id`, `value_native`, `value_base`, `currency_native`, `currency_base`.
/// The instrument identifier can be re-joined by consumers if required.
///
/// # Arguments
///
/// * `valuation` - Portfolio valuation containing per-position monetary values.
///
/// # Returns
///
/// A [`Result`] wrapping the generated [`polars::prelude::DataFrame`].
pub fn positions_to_dataframe(
    valuation: &PortfolioValuation,
) -> Result<polars::prelude::DataFrame> {
    let n = valuation.position_values.len();
    let mut position_ids: Vec<String> = Vec::with_capacity(n);
    let mut entity_ids: Vec<String> = Vec::with_capacity(n);
    let mut values_native: Vec<f64> = Vec::with_capacity(n);
    let mut values_base: Vec<f64> = Vec::with_capacity(n);
    let mut currencies_native: Vec<String> = Vec::with_capacity(n);
    let mut currencies_base: Vec<String> = Vec::with_capacity(n);

    for (position_id, position_value) in &valuation.position_values {
        position_ids.push(position_id.to_string());
        entity_ids.push(position_value.entity_id.to_string());
        values_native.push(position_value.value_native.amount());
        values_base.push(position_value.value_base.amount());
        currencies_native.push(position_value.value_native.currency().to_string());
        currencies_base.push(position_value.value_base.currency().to_string());
    }

    let df = polars::prelude::df! (
        "position_id" => position_ids,
        "entity_id" => entity_ids,
        "value_native" => values_native,
        "value_base" => values_base,
        "currency_native" => currencies_native,
        "currency_base" => currencies_base,
    )
    .map_err(|_| Error::Input(InputError::Invalid))?;

    Ok(df)
}

/// Export entity-level aggregates to a Polars `DataFrame`.
///
/// Columns produced: `entity_id`, `total_value`, `currency`.
///
/// # Arguments
///
/// * `valuation` - Portfolio valuation containing aggregated entity values.
///
/// # Returns
///
/// A [`Result`] containing the [`polars::prelude::DataFrame`].
pub fn entities_to_dataframe(valuation: &PortfolioValuation) -> Result<polars::prelude::DataFrame> {
    let n = valuation.by_entity.len();
    let mut entity_ids: Vec<String> = Vec::with_capacity(n);
    let mut total_values: Vec<f64> = Vec::with_capacity(n);
    let mut currencies: Vec<String> = Vec::with_capacity(n);

    for (entity_id, money) in &valuation.by_entity {
        entity_ids.push(entity_id.to_string());
        total_values.push(money.amount());
        currencies.push(money.currency().to_string());
    }

    let df = polars::prelude::df! (
        "entity_id" => entity_ids,
        "total_value" => total_values,
        "currency" => currencies,
    )
    .map_err(|_| Error::Input(InputError::Invalid))?;

    Ok(df)
}

/// Export metrics to a Polars `DataFrame` in long format.
///
/// Columns produced: `metric_id`, `position_id`, `value`.
///
/// # Arguments
///
/// * `metrics` - Aggregated and per-position portfolio metrics.
///
/// # Returns
///
/// A [`Result`] containing the [`polars::prelude::DataFrame`].
pub fn metrics_to_dataframe(metrics: &PortfolioMetrics) -> Result<polars::prelude::DataFrame> {
    let mut metric_ids: Vec<String> = Vec::new();
    let mut position_ids: Vec<String> = Vec::new();
    let mut values: Vec<f64> = Vec::new();

    for (position_id, position_metrics) in &metrics.by_position {
        for (metric_id, value) in position_metrics {
            metric_ids.push(metric_id.clone());
            position_ids.push(position_id.to_string());
            values.push(*value);
        }
    }

    let df = polars::prelude::df! (
        "metric_id" => metric_ids,
        "position_id" => position_ids,
        "value" => values,
    )
    .map_err(|_| Error::Input(InputError::Invalid))?;

    Ok(df)
}

/// Export aggregated metrics to a Polars `DataFrame`.
///
/// Columns produced: `metric_id`, `total`, where `total` is the summation across positions.
///
/// # Arguments
///
/// * `metrics` - Portfolio metrics containing aggregate totals.
///
/// # Returns
///
/// A [`Result`] containing the [`polars::prelude::DataFrame`].
pub fn aggregated_metrics_to_dataframe(
    metrics: &PortfolioMetrics,
) -> Result<polars::prelude::DataFrame> {
    let n = metrics.aggregated.len();
    let mut metric_ids: Vec<String> = Vec::with_capacity(n);
    let mut totals: Vec<f64> = Vec::with_capacity(n);

    for (metric_id, agg_metric) in &metrics.aggregated {
        metric_ids.push(metric_id.clone());
        totals.push(agg_metric.total);
    }

    let df = polars::prelude::df! (
        "metric_id" => metric_ids,
        "total" => totals,
    )
    .map_err(|_| Error::Input(InputError::Invalid))?;

    Ok(df)
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
            .expect("test should succeed");

        MarketContext::new().insert_discount(curve)
    }

    #[test]
    fn test_positions_to_dataframe() {
        let as_of = date!(2024 - 01 - 01);

        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
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
        let df = positions_to_dataframe(&valuation).expect("test should succeed");

        assert_eq!(df.height(), 1);
        let col_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert!(col_names.contains(&"position_id"));
        assert!(col_names.contains(&"value_base"));
    }

    #[test]
    fn test_entities_to_dataframe() {
        let as_of = date!(2024 - 01 - 01);

        let deposit = Deposit::builder()
            .id("DEP_1M".into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
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
        let df = entities_to_dataframe(&valuation).expect("test should succeed");

        assert_eq!(df.height(), 1);
        let col_names: Vec<&str> = df.get_column_names().iter().map(|s| s.as_str()).collect();
        assert!(col_names.contains(&"entity_id"));
        assert!(col_names.contains(&"total_value"));
    }
}
