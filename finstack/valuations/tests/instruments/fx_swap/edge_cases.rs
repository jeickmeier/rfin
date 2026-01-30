//! Edge case and error handling tests for FX swaps.
//!
//! Tests boundary conditions, error handling, and unusual scenarios
//! to ensure robustness and proper validation.

use super::fixtures::*;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::instruments::fx::fx_swap::FxSwap;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use time::Month;

#[test]
fn test_zero_notional() {
    // Test swap with zero notional
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap("ZERO_NOTIONAL", dates.near_date, dates.far_date_1y, 0.0);

    let pv = swap.value(&market, dates.as_of).unwrap();

    // Zero notional should produce zero PV
    assert_eq!(pv.amount(), 0.0, "Zero notional should yield zero PV");
}

#[test]
fn test_very_large_notional() {
    // Test swap with very large notional (billions)
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap(
        "LARGE_NOTIONAL",
        dates.near_date,
        dates.far_date_1y,
        1_000_000_000.0, // 1 billion EUR
    );

    let pv = swap.value(&market, dates.as_of).unwrap();

    // Should handle large notionals without overflow
    assert!(
        pv.amount().is_finite(),
        "Large notional should produce finite PV"
    );
}

#[test]
fn test_same_near_far_dates() {
    // Test swap where near and far dates are the same (degenerate case)
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap(
        "SAME_DATES",
        dates.near_date,
        dates.near_date, // Same as near date
        1_000_000.0,
    );

    let pv = swap.value(&market, dates.as_of).unwrap();

    // Degenerate swap should have PV close to zero
    assert!(
        pv.amount().abs() < 1000.0,
        "Swap with same near/far dates should have minimal PV"
    );
}

#[test]
fn test_far_before_near() {
    // Test swap with far date before near date (invalid)
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap(
        "INVERTED_DATES",
        dates.far_date_1y,
        dates.near_date, // Far before near
        1_000_000.0,
    );

    // Should return an error for invalid date ordering
    let result = swap.value(&market, dates.as_of);
    assert!(result.is_err(), "Should reject inverted dates");
    let err = result.expect_err("expected validation error");
    assert!(
        err.to_string().contains("near_date") && err.to_string().contains("far_date"),
        "Error should mention date ordering: {}",
        err
    );
}

#[test]
fn test_missing_fx_matrix() {
    // Test swap valuation when FX matrix is not provided
    let dates = TestDates::standard();

    // Create market without FX matrix
    let as_of = dates.as_of;
    let usd_curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.9)])
        .set_interp(finstack_core::math::interp::InterpStyle::Linear)
        .build()
        .unwrap();

    let eur_curve = finstack_core::market_data::term_structures::DiscountCurve::builder("EUR-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.95)])
        .set_interp(finstack_core::math::interp::InterpStyle::Linear)
        .build()
        .unwrap();

    let market = finstack_core::market_data::context::MarketContext::new()
        .insert_discount(usd_curve)
        .insert_discount(eur_curve);
    // No FX matrix!

    let swap = create_standard_fx_swap(
        "NO_FX_MATRIX",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
    );

    // Should return error when FX matrix is missing
    let result = swap.value(&market, dates.as_of);
    assert!(result.is_err(), "Should error without FX matrix");
}

#[test]
fn test_missing_fx_matrix_with_contract_rates() {
    // Test that swap works without FX matrix if contract rates are provided
    let dates = TestDates::standard();

    // Create market without FX matrix
    let as_of = dates.as_of;
    let usd_curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.9)])
        .set_interp(finstack_core::math::interp::InterpStyle::Linear)
        .build()
        .unwrap();

    let eur_curve = finstack_core::market_data::term_structures::DiscountCurve::builder("EUR-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.95)])
        .set_interp(finstack_core::math::interp::InterpStyle::Linear)
        .build()
        .unwrap();

    let market = finstack_core::market_data::context::MarketContext::new()
        .insert_discount(usd_curve)
        .insert_discount(eur_curve);

    let swap = create_fx_swap_with_rates(
        "NO_FX_WITH_RATES",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
        1.10,
        1.15,
    );

    // Should work when contract rates are explicit
    let result = swap.value(&market, dates.as_of);
    assert!(
        result.is_ok(),
        "Should work without FX matrix when contract rates provided"
    );
}

#[test]
fn test_missing_discount_curve() {
    // Test swap valuation when discount curve is missing
    let dates = TestDates::standard();
    let market = finstack_core::market_data::context::MarketContext::new(); // Empty market

    let swap = create_standard_fx_swap("NO_CURVE", dates.near_date, dates.far_date_1y, 1_000_000.0);

    // Should return error when curves are missing
    let result = swap.value(&market, dates.as_of);
    assert!(result.is_err(), "Should error when discount curves missing");
}

#[test]
fn test_currency_mismatch_notional() {
    // Test swap with notional currency not matching base currency
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = FxSwap::builder()
        .id("CURRENCY_MISMATCH".to_string().into())
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .near_date(dates.near_date)
        .far_date(dates.far_date_1y)
        .base_notional(Money::new(1_000_000.0, Currency::GBP)) // Wrong currency!
        .domestic_discount_curve_id("USD-OIS".into())
        .foreign_discount_curve_id("EUR-OIS".into())
        .build()
        .unwrap();

    // Should return error due to currency mismatch
    let result = swap.value(&market, dates.as_of);
    assert!(
        result.is_err(),
        "Should error when notional currency doesn't match base currency"
    );
}

#[test]
fn test_valuation_at_near_date() {
    // Test valuation when as_of equals near date
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap(
        "AT_NEAR_DATE",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
    );

    let pv = swap.value(&market, dates.near_date).unwrap();

    // Should produce valid PV at near date
    assert!(pv.amount().is_finite(), "PV at near date should be finite");
}

#[test]
fn test_valuation_at_far_date() {
    // Test valuation when as_of equals far date
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_fx_swap_with_rates(
        "AT_FAR_DATE",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
        1.10,
        1.15,
    );

    let pv = swap.value(&market, dates.far_date_1y).unwrap();

    // At far date, remaining PV should be small
    assert!(pv.amount().is_finite(), "PV at far date should be finite");
}

#[test]
fn test_valuation_after_maturity() {
    // Test valuation after far date (expired swap)
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap("EXPIRED", dates.near_date, dates.far_date_1y, 1_000_000.0);

    let as_of_after = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let pv = swap.value(&market, as_of_after).unwrap();

    // After maturity, PV should be close to zero or represent final settlement
    assert!(
        pv.amount().is_finite(),
        "PV after maturity should be finite"
    );
}

#[test]
fn test_extreme_contract_rates() {
    // Test swap with extreme contract rates
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_fx_swap_with_rates(
        "EXTREME_RATES",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
        0.01,  // Very low near rate
        100.0, // Very high far rate
    );

    let pv = swap.value(&market, dates.as_of).unwrap();

    // Should handle extreme rates without overflow
    assert!(pv.amount().is_finite(), "Should handle extreme rates");
}

#[test]
fn test_negative_contract_rates() {
    // Test that negative FX rates are rejected (FX rates must be positive)
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_fx_swap_with_rates(
        "NEGATIVE_RATES",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
        1.10,
        -0.05, // Negative far rate - should be rejected
    );

    let result = swap.value(&market, dates.as_of);

    // Negative FX rates are invalid and should be rejected
    assert!(result.is_err(), "Negative FX rate should be rejected");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("positive"),
        "Error should mention rate must be positive: {}",
        err_msg
    );
}

#[test]
fn test_builder_validation() {
    // Test that builder properly validates required fields
    // Attempt to build without required fields
    let result = FxSwap::builder()
        .id("INCOMPLETE".to_string().into())
        .base_currency(Currency::EUR)
        // Missing other required fields
        .build();

    assert!(result.is_err(), "Builder should validate required fields");
}

#[test]
fn test_attributes_access() {
    // Test that instrument attributes can be accessed and modified
    let dates = TestDates::standard();

    let mut swap = create_standard_fx_swap(
        "ATTRIBUTES",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
    );

    // Access attributes
    let _attrs = swap.attributes();

    // Modify attributes
    let attrs_mut = swap.attributes_mut();
    attrs_mut
        .meta
        .insert("trader".to_string(), "test_trader".to_string());

    // Verify modification
    assert_eq!(
        swap.attributes().meta.get("trader"),
        Some(&"test_trader".to_string())
    );
}

#[test]
fn test_metric_error_handling() {
    // Test that metrics handle missing data gracefully
    let dates = TestDates::standard();
    let market = finstack_core::market_data::context::MarketContext::new(); // Empty market

    let swap = create_standard_fx_swap(
        "METRIC_ERROR",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
    );

    // Should return error when trying to calculate metrics without market data
    let result =
        swap.price_with_metrics(&market, dates.as_of, &[MetricId::custom("forward_points")]);

    assert!(result.is_err(), "Metrics should error without market data");
}
