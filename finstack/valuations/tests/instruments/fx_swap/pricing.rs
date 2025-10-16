//! Core pricing tests for FX swaps.
//!
//! Tests the fundamental valuation logic including:
//! - Basic PV calculation at inception and over time
//! - Contract rates vs. model-implied rates
//! - Fair value pricing
//! - Currency consistency

use super::fixtures::*;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::FxSwap;

#[test]
fn test_basic_pv_at_inception() {
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap("BASIC_PV", dates.near_date, dates.far_date_1y, 1_000_000.0);

    let pv = swap.value(&market, dates.as_of).unwrap();

    // At inception with model-implied rates, PV should be close to zero
    // (within a few thousand due to rounding and discrete curve points)
    assert!(
        pv.amount().abs() < 5000.0,
        "PV at inception should be near zero, got: {}",
        pv.amount()
    );
    assert_eq!(
        pv.currency(),
        Currency::USD,
        "PV should be in quote currency"
    );
}

#[test]
fn test_pv_with_contract_rates_fair() {
    // Test that when contract rates match model rates, PV is near zero
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    // Calculate model-implied forward
    let spot = 1.1;
    let df_dom_far = 0.99; // Approximate from curve
    let df_for_far = 0.995; // Approximate from curve
    let model_fwd = spot * df_for_far / df_dom_far;

    let swap = create_fx_swap_with_rates(
        "FAIR_CONTRACT",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
        spot,
        model_fwd,
    );

    let pv = swap.value(&market, dates.as_of).unwrap();

    // With fair contract rates, PV should be very close to zero
    assert!(
        pv.amount().abs() < 10000.0,
        "PV with fair contract rates should be near zero, got: {}",
        pv.amount()
    );
}

#[test]
fn test_pv_with_mispriced_far_rate() {
    // Test that mispriced contract rates produce non-zero PV
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_fx_swap_with_rates(
        "MISPRICED",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
        1.10, // spot
        1.25, // significantly off-market forward
    );

    let pv = swap.value(&market, dates.as_of).unwrap();

    // With mispriced far rate, should have material PV
    assert!(
        pv.amount().abs() > 1000.0,
        "PV with mispriced far rate should be material, got: {}",
        pv.amount()
    );
}

#[test]
fn test_pv_different_tenors() {
    // Test PV calculation for various swap tenors
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap_1m =
        create_standard_fx_swap("SWAP_1M", dates.near_date, dates.far_date_1m, 1_000_000.0);

    let swap_3m =
        create_standard_fx_swap("SWAP_3M", dates.near_date, dates.far_date_3m, 1_000_000.0);

    let swap_1y =
        create_standard_fx_swap("SWAP_1Y", dates.near_date, dates.far_date_1y, 1_000_000.0);

    let pv_1m = swap_1m.value(&market, dates.as_of).unwrap();
    let pv_3m = swap_3m.value(&market, dates.as_of).unwrap();
    let pv_1y = swap_1y.value(&market, dates.as_of).unwrap();

    // All should be close to zero at inception
    assert!(
        pv_1m.amount().abs() < 5000.0,
        "1M swap PV should be near zero"
    );
    assert!(
        pv_3m.amount().abs() < 5000.0,
        "3M swap PV should be near zero"
    );
    assert!(
        pv_1y.amount().abs() < 5000.0,
        "1Y swap PV should be near zero"
    );
}

#[test]
fn test_pv_with_different_notionals() {
    // Test that PV scales linearly with notional
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap_1m = create_fx_swap_with_rates(
        "NOTIONAL_1M",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
        1.10,
        1.20,
    );

    let swap_2m = create_fx_swap_with_rates(
        "NOTIONAL_2M",
        dates.near_date,
        dates.far_date_1y,
        2_000_000.0,
        1.10,
        1.20,
    );

    let pv_1m = swap_1m.value(&market, dates.as_of).unwrap();
    let pv_2m = swap_2m.value(&market, dates.as_of).unwrap();

    // 2M notional should produce roughly 2x the PV
    assert_within_pct(
        pv_2m.amount(),
        pv_1m.amount() * 2.0,
        1.0,
        "PV should scale linearly with notional",
    );
}

#[test]
fn test_pv_steep_curves() {
    // Test PV calculation with steep interest rate curves
    let dates = TestDates::standard();
    let market = setup_steep_curve_market(dates.as_of);

    let swap = create_standard_fx_swap(
        "STEEP_CURVES",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
    );

    let pv = swap.value(&market, dates.as_of).unwrap();

    // Should still produce a valid PV
    assert!(
        pv.amount().is_finite(),
        "PV with steep curves should be finite"
    );
}

#[test]
fn test_pv_inverted_curves() {
    // Test PV with inverted yield curves (negative term premium)
    let dates = TestDates::standard();
    let market = setup_inverted_curve_market(dates.as_of);

    let swap = create_standard_fx_swap("INVERTED", dates.near_date, dates.far_date_1y, 1_000_000.0);

    let pv = swap.value(&market, dates.as_of).unwrap();

    // Should handle inverted curves gracefully
    assert!(
        pv.amount().is_finite(),
        "PV with inverted curves should be finite"
    );
}

#[test]
fn test_pv_currency_consistency() {
    // Verify that PV is always returned in quote currency
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = create_standard_fx_swap(
        "CURRENCY_CHECK",
        dates.near_date,
        dates.far_date_1y,
        1_000_000.0,
    );

    let pv = swap.value(&market, dates.as_of).unwrap();

    assert_eq!(
        pv.currency(),
        Currency::USD,
        "PV must be in quote currency (USD)"
    );
}

#[test]
fn test_pv_with_only_near_rate() {
    // Test swap with only near rate specified (far rate from model)
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = FxSwap::builder()
        .id("NEAR_RATE_ONLY".to_string().into())
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .near_date(dates.near_date)
        .far_date(dates.far_date_1y)
        .base_notional(Money::new(1_000_000.0, Currency::EUR))
        .domestic_disc_id("USD-OIS".into())
        .foreign_disc_id("EUR-OIS".into())
        .near_rate(1.10)
        .build()
        .unwrap();

    let pv = swap.value(&market, dates.as_of).unwrap();

    // Should produce valid PV using model forward
    assert!(pv.amount().is_finite(), "PV should be finite");
}

#[test]
fn test_pv_with_only_far_rate() {
    // Test swap with only far rate specified (near rate from market)
    let dates = TestDates::standard();
    let market = setup_standard_market(dates.as_of);

    let swap = FxSwap::builder()
        .id("FAR_RATE_ONLY".to_string().into())
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .near_date(dates.near_date)
        .far_date(dates.far_date_1y)
        .base_notional(Money::new(1_000_000.0, Currency::EUR))
        .domestic_disc_id("USD-OIS".into())
        .foreign_disc_id("EUR-OIS".into())
        .far_rate(1.20)
        .build()
        .unwrap();

    let pv = swap.value(&market, dates.as_of).unwrap();

    // Should produce valid PV using market spot
    assert!(pv.amount().is_finite(), "PV should be finite");
}
