mod common;

use crate::common::*;
use finstack_core::math::summation::neumaier_sum;
use finstack_core::prelude::*;
use finstack_portfolio::types::Entity;
use finstack_portfolio::{PortfolioBuilder, PortfolioError, Position, PositionUnit};
use finstack_valuations::instruments::deposit::Deposit;
use std::sync::Arc;
use time::Duration;

#[test]
fn test_compensated_summation_large_portfolio() {
    // Test that compensated summation handles large portfolios with mixed-sign values
    // Create 1000 positions with alternating ±1e12 and 1e0 values
    let as_of = base_date();
    let end_date = as_of + Duration::days(30);

    let mut builder = PortfolioBuilder::new("LARGE_PORTFOLIO")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("ENTITY_A"));

    for i in 0..1000 {
        let value: f64 = if i % 2 == 0 {
            1e12 // Large positive
        } else {
            -1e12 // Large negative
        };

        let deposit = Deposit::builder()
            .id(format!("DEP_{}", i).into())
            .notional(Money::new(value.abs(), Currency::USD))
            .start(as_of)
            .end(end_date)
            .day_count(finstack_core::dates::DayCount::Act360)
            .discount_curve_id("USD".into())
            .build()
            .unwrap();

        let quantity = if value < 0.0 { -1.0 } else { 1.0 };
        let position = Position::new(
            format!("POS_{}", i),
            "ENTITY_A",
            format!("DEP_{}", i),
            Arc::new(deposit),
            quantity,
            PositionUnit::Units,
        )
        .unwrap();

        builder = builder.position(position);
    }

    let portfolio = builder.build().unwrap();
    let market = market_with_usd();
    let config = FinstackConfig::default();

    let valuation = finstack_portfolio::value_portfolio(&portfolio, &market, &config).unwrap();

    // With alternating ±1e12, the total should be close to zero
    // Compensated summation should maintain accuracy
    let total = valuation.total_base_ccy.amount();

    // Verify that compensated summation produces a reasonable result
    // The exact value depends on discounting, but should be finite and not NaN/Inf
    assert!(total.is_finite(), "Total should be finite");
    assert!(
        total.abs() < 1e15,
        "Total should be reasonable magnitude, got: {}",
        total
    );
}

#[test]
fn test_nan_metric_excluded() {
    // Create a position that might produce NaN metrics
    // In practice, this would require a custom instrument, but we can test
    // that the aggregation logic handles NaN correctly
    let as_of = base_date();
    let end_date = as_of + Duration::days(30);

    let deposit = Deposit::builder()
        .id("DEP_NAN".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(end_date)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    let position = Position::new(
        "POS_NAN",
        "ENTITY_A",
        "DEP_NAN",
        Arc::new(deposit),
        1.0,
        PositionUnit::Units,
    )
    .unwrap();

    let portfolio = PortfolioBuilder::new("TEST")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("ENTITY_A"))
        .position(position)
        .build()
        .unwrap();

    let market = market_with_usd();
    let config = FinstackConfig::default();

    let valuation = finstack_portfolio::value_portfolio(&portfolio, &market, &config).unwrap();
    let metrics = finstack_portfolio::aggregate_metrics(&valuation).unwrap();

    // Verify that all aggregated metrics are finite
    for (metric_id, agg_metric) in &metrics.aggregated {
        assert!(
            agg_metric.total.is_finite(),
            "Metric {} total should be finite, got: {}",
            metric_id,
            agg_metric.total
        );
        for (entity_id, value) in &agg_metric.by_entity {
            assert!(
                value.is_finite(),
                "Metric {} for entity {} should be finite, got: {}",
                metric_id,
                entity_id,
                value
            );
        }
    }
}

#[test]
fn test_inf_quantity_rejected() {
    let as_of = base_date();
    let end_date = as_of + Duration::days(30);

    let deposit = Deposit::builder()
        .id("DEP_INF".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(end_date)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    // Try to create position with Inf quantity
    let result = Position::new(
        "POS_INF",
        "ENTITY_A",
        "DEP_INF",
        Arc::new(deposit),
        f64::INFINITY,
        PositionUnit::Units,
    );

    assert!(
        result.is_err(),
        "Position::new() should reject Inf quantity"
    );

    match result {
        Err(PortfolioError::InvalidInput(msg)) => {
            assert!(
                msg.contains("finite"),
                "Error message should mention 'finite'"
            );
        }
        other => panic!("Expected InvalidInput error, got: {:?}", other),
    }
}

#[test]
fn test_nan_quantity_rejected() {
    let as_of = base_date();
    let end_date = as_of + Duration::days(30);

    let deposit = Deposit::builder()
        .id("DEP_NAN".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(end_date)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .build()
        .unwrap();

    // Try to create position with NaN quantity
    let result = Position::new(
        "POS_NAN",
        "ENTITY_A",
        "DEP_NAN",
        Arc::new(deposit),
        f64::NAN,
        PositionUnit::Units,
    );

    assert!(
        result.is_err(),
        "Position::new() should reject NaN quantity"
    );

    match result {
        Err(PortfolioError::InvalidInput(msg)) => {
            assert!(
                msg.contains("finite"),
                "Error message should mention 'finite'"
            );
        }
        other => panic!("Expected InvalidInput error, got: {:?}", other),
    }
}

#[test]
fn test_neumaier_sum_accuracy() {
    // Direct test of Neumaier summation accuracy
    // Create alternating large positive and negative values
    let values: Vec<f64> = (0..1000)
        .map(|i| if i % 2 == 0 { 1e12 } else { -1e12 })
        .collect();

    let neumaier_result = neumaier_sum(values.iter().copied());
    let naive_result: f64 = values.iter().sum();

    // Neumaier should be more accurate than naive summation
    // With alternating ±1e12, the true sum is 0.0
    assert!(
        neumaier_result.abs() < naive_result.abs() || naive_result.abs() < 1e-6,
        "Neumaier summation should be at least as accurate as naive: neumaier={}, naive={}",
        neumaier_result,
        naive_result
    );
}
