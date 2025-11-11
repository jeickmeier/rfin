//! Metric framework integration tests for CDS Tranche.
//!
//! Tests the integration between CDS Tranche instrument and the metric calculation framework.
//! Validates that all registered metrics can be calculated successfully via price_with_metrics().

use super::helpers::*;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;

// ==================== Individual Metric Calculation Tests ====================

#[test]
fn test_upfront_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::custom("upfront")]);

    // Assert
    assert!(result.is_ok(), "Upfront calculation should succeed");
    let valuation = result.unwrap();
    let upfront = *valuation
        .measures
        .get("upfront")
        .expect("upfront should be in measures");
    assert!(upfront.is_finite(), "Upfront should be finite");
}

#[test]
fn test_spread_dv01_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::SpreadDv01]);

    // Assert
    assert!(result.is_ok(), "Spread DV01 calculation should succeed");
    let valuation = result.unwrap();
    let spread_dv01 = *valuation
        .measures
        .get("spread_dv01")
        .expect("spread_dv01 should be in measures");
    assert!(spread_dv01.is_finite(), "Spread DV01 should be finite");
    assert!(
        spread_dv01 > 0.0,
        "Spread DV01 should be positive for sell protection"
    );
}

#[test]
fn test_correlation_delta_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::Correlation01]);

    // Assert
    assert!(
        result.is_ok(),
        "Correlation delta calculation should succeed"
    );
    let valuation = result.unwrap();
    let corr_delta = *valuation
        .measures
        .get("correlation01")
        .expect("correlation01 should be in measures");
    assert!(corr_delta.is_finite(), "Correlation delta should be finite");
}

#[test]
fn test_cs01_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::Cs01]);

    // Assert
    assert!(result.is_ok(), "CS01 calculation should succeed");
    let valuation = result.unwrap();
    let cs01 = *valuation
        .measures
        .get("cs01")
        .expect("cs01 should be in measures");
    assert!(cs01.is_finite(), "CS01 should be finite");
}

#[test]
fn test_par_spread_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::ParSpread]);

    // Assert
    assert!(result.is_ok(), "Par spread calculation should succeed");
    let valuation = result.unwrap();
    let par_spread = *valuation
        .measures
        .get("par_spread")
        .expect("par_spread should be in measures");
    assert_finite_non_negative(par_spread, "Par spread");
}

#[test]
fn test_expected_loss_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::ExpectedLoss]);

    // Assert
    assert!(result.is_ok(), "Expected loss calculation should succeed");
    let valuation = result.unwrap();
    let el = *valuation
        .measures
        .get("expected_loss")
        .expect("expected_loss should be in measures");
    assert_finite_non_negative(el, "Expected loss");
}

#[test]
fn test_jump_to_default_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::JumpToDefault]);

    // Assert
    assert!(result.is_ok(), "Jump-to-default calculation should succeed");
    let valuation = result.unwrap();
    let jtd = *valuation
        .measures
        .get("jump_to_default")
        .expect("jump_to_default should be in measures");
    assert_finite_non_negative(jtd, "Jump-to-default");
}

#[test]
fn test_dv01_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::Dv01]);

    // Assert
    assert!(result.is_ok(), "DV01 calculation should succeed");
    let valuation = result.unwrap();
    let dv01 = *valuation
        .measures
        .get("dv01")
        .expect("dv01 should be in measures");
    // DV01 = PV(rate+1bp) - PV(base); sign depends on instrument structure
    assert!(dv01.is_finite(), "DV01 should be finite");
}

#[test]
fn test_theta_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::Theta]);

    // Assert
    assert!(result.is_ok(), "Theta calculation should succeed");
    let valuation = result.unwrap();
    let theta = *valuation
        .measures
        .get("theta")
        .expect("theta should be in measures");
    assert!(theta.is_finite(), "Theta should be finite");
}

#[test]
fn test_bucketed_dv01_metric_via_price_with_metrics() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &[MetricId::BucketedDv01]);

    // Assert
    assert!(result.is_ok(), "Bucketed DV01 calculation should succeed");
    let valuation = result.unwrap();
    let bucketed_dv01 = *valuation
        .measures
        .get("bucketed_dv01")
        .expect("bucketed_dv01 should be in measures");
    assert!(bucketed_dv01.is_finite(), "Bucketed DV01 should be finite");
}

// ==================== Multiple Metrics Calculation Tests ====================

#[test]
fn test_calculate_multiple_metrics_simultaneously() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    let metrics = vec![
        MetricId::Cs01,
        MetricId::ParSpread,
        MetricId::ExpectedLoss,
        MetricId::JumpToDefault,
    ];

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &metrics);

    // Assert
    assert!(
        result.is_ok(),
        "Multiple metrics calculation should succeed"
    );
    let valuation = result.unwrap();

    assert!(valuation.value.amount().is_finite(), "PV should be finite");
    assert_eq!(
        valuation.measures.len(),
        metrics.len(),
        "Should have all requested metrics"
    );

    // Verify each metric is present and finite
    assert!(valuation.measures.get("cs01").is_some());
    assert!(valuation.measures.get("par_spread").is_some());
    assert!(valuation.measures.get("expected_loss").is_some());
    assert!(valuation.measures.get("jump_to_default").is_some());
}

#[test]
fn test_all_standard_metrics_calculable() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    let all_metrics = vec![
        MetricId::custom("upfront"),
        MetricId::SpreadDv01,
        MetricId::Correlation01,
        MetricId::Cs01,
        MetricId::ParSpread,
        MetricId::ExpectedLoss,
        MetricId::JumpToDefault,
        MetricId::Dv01,
        MetricId::Theta,
    ];

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &all_metrics);

    // Assert
    assert!(result.is_ok(), "All metrics calculation should succeed");
    let valuation = result.unwrap();

    // Verify all metrics are present and finite
    for metric_value in valuation.measures.values() {
        assert!(
            metric_value.is_finite(),
            "All metrics should produce finite values"
        );
    }
}

// ==================== Edge Case: Missing Market Data ====================

#[test]
fn test_metrics_with_missing_credit_index() {
    // Arrange
    let tranche = mezzanine_tranche();

    // Create market without credit index
    let market =
        finstack_core::market_data::MarketContext::new().insert_discount(standard_discount_curve());

    let as_of = base_date();

    // Metrics that should fallback to zero when credit index is missing
    let fallback_metrics = vec![
        MetricId::custom("upfront"),
        MetricId::custom("spread_dv01"),
        MetricId::ExpectedLoss,
        MetricId::JumpToDefault,
    ];

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &fallback_metrics);

    // Assert: Should either succeed with 0.0 values or fail gracefully
    if let Ok(valuation) = result {
        for &metric_value in valuation.measures.values() {
            assert_absolute_eq(
                metric_value,
                0.0,
                1e-10,
                "Metrics should fallback to zero when credit index missing",
            );
        }
    }
}

// ==================== Metric Calculation Order Independence ====================

#[test]
fn test_metrics_order_independence() {
    // Arrange
    let tranche = mezzanine_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    let metrics_order_1 = vec![MetricId::Cs01, MetricId::ParSpread, MetricId::ExpectedLoss];
    let metrics_order_2 = vec![MetricId::ExpectedLoss, MetricId::Cs01, MetricId::ParSpread];

    // Act
    let result_1 = tranche
        .price_with_metrics(&market, as_of, &metrics_order_1)
        .unwrap();
    let result_2 = tranche
        .price_with_metrics(&market, as_of, &metrics_order_2)
        .unwrap();

    // Assert: Results should be consistent regardless of order
    let cs01_1 = *result_1.measures.get("cs01").unwrap();
    let cs01_2 = *result_2.measures.get("cs01").unwrap();
    assert_relative_eq(cs01_1, cs01_2, 1e-6, "CS01 should be order-independent");

    let par_1 = *result_1.measures.get("par_spread").unwrap();
    let par_2 = *result_2.measures.get("par_spread").unwrap();
    assert_relative_eq(par_1, par_2, 1e-6, "Par spread should be order-independent");

    let el_1 = *result_1.measures.get("expected_loss").unwrap();
    let el_2 = *result_2.measures.get("expected_loss").unwrap();
    assert_relative_eq(
        el_1,
        el_2,
        1e-6,
        "Expected loss should be order-independent",
    );
}

// ==================== Instrument Trait Integration ====================

#[test]
fn test_price_with_metrics_returns_pv_and_metrics() {
    // Arrange
    let tranche: Box<dyn Instrument> = Box::new(mezzanine_tranche());
    let market = standard_market_context();
    let as_of = base_date();

    let metrics = vec![MetricId::Cs01, MetricId::ParSpread];

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &metrics);

    // Assert
    assert!(result.is_ok(), "price_with_metrics should succeed");
    let valuation = result.unwrap();

    assert!(valuation.value.amount().is_finite(), "PV should be finite");
    assert_eq!(
        valuation.measures.len(),
        metrics.len(),
        "Should have all requested metrics"
    );
}

#[test]
fn test_equity_tranche_metrics() {
    // Arrange
    let tranche = equity_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    let metrics = vec![MetricId::ParSpread, MetricId::ExpectedLoss];

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &metrics);

    // Assert
    assert!(result.is_ok());
    let valuation = result.unwrap();

    let par_spread = *valuation.measures.get("par_spread").unwrap();
    let expected_loss = *valuation.measures.get("expected_loss").unwrap();

    assert_finite_non_negative(par_spread, "Equity par spread");
    assert_finite_non_negative(expected_loss, "Equity expected loss");
}

#[test]
fn test_senior_tranche_metrics() {
    // Arrange
    let tranche = senior_tranche();
    let market = standard_market_context();
    let as_of = base_date();

    let metrics = vec![MetricId::ParSpread, MetricId::ExpectedLoss];

    // Act
    let result = tranche.price_with_metrics(&market, as_of, &metrics);

    // Assert
    assert!(result.is_ok());
    let valuation = result.unwrap();

    let par_spread = *valuation.measures.get("par_spread").unwrap();
    let expected_loss = *valuation.measures.get("expected_loss").unwrap();

    assert_finite_non_negative(par_spread, "Senior par spread");
    assert_finite_non_negative(expected_loss, "Senior expected loss");
}

// ==================== Metrics Scaling Tests ====================

#[test]
fn test_metrics_scale_with_notional() {
    // Arrange
    let market = standard_market_context();
    let as_of = base_date();

    let mut tranche_10mm = mezzanine_tranche();
    tranche_10mm.notional =
        finstack_core::money::Money::new(10_000_000.0, finstack_core::currency::Currency::USD);

    let mut tranche_20mm = mezzanine_tranche();
    tranche_20mm.notional =
        finstack_core::money::Money::new(20_000_000.0, finstack_core::currency::Currency::USD);

    let metrics = vec![MetricId::ExpectedLoss, MetricId::JumpToDefault];

    // Act
    let result_10 = tranche_10mm
        .price_with_metrics(&market, as_of, &metrics)
        .unwrap();
    let result_20 = tranche_20mm
        .price_with_metrics(&market, as_of, &metrics)
        .unwrap();

    // Assert: Dollar metrics should scale with notional
    let el_10 = *result_10.measures.get("expected_loss").unwrap();
    let el_20 = *result_20.measures.get("expected_loss").unwrap();
    assert_relative_eq(
        el_20 / el_10,
        2.0,
        0.001,
        "Expected loss should scale with notional",
    );

    let jtd_10 = *result_10.measures.get("jump_to_default").unwrap();
    let jtd_20 = *result_20.measures.get("jump_to_default").unwrap();
    assert_relative_eq(
        jtd_20 / jtd_10,
        2.0,
        0.001,
        "JTD should scale with notional",
    );
}
