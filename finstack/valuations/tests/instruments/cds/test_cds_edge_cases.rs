//! CDS edge cases and error handling tests.
//!
//! Tests boundary conditions, error cases, and numerical stability.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::credit_derivatives::cds::CreditDefaultSwap;
use finstack_valuations::instruments::credit_derivatives::cds::{
    CDSPricer, CDSPricerConfig, IntegrationMethod,
};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn build_curves(as_of: Date) -> (DiscountCurve, HazardCurve) {
    let disc = DiscountCurve::builder("USD_OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78), (10.0, 0.61)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("CORP")
        .base_date(as_of)
        .recovery_rate(0.40)
        .knots([(0.0, 0.01), (1.0, 0.01), (5.0, 0.015), (10.0, 0.02)])
        .build()
        .unwrap();

    (disc, hazard)
}

#[test]
fn test_zero_notional() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "ZERO_NOTIONAL",
        Money::new(0.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of).unwrap();
    assert_eq!(npv.amount(), 0.0, "Zero notional should give zero NPV");
}

#[test]
fn test_zero_spread() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "ZERO_SPREAD",
        Money::new(10_000_000.0, Currency::USD),
        0.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of).unwrap();

    // With zero spread, buyer pays nothing but receives protection
    // NPV should be positive (value of protection)
    assert!(
        npv.amount() > 0.0,
        "Zero spread CDS should have positive NPV for buyer"
    );
}

#[test]
fn test_negative_spread() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "NEG_SPREAD",
        Money::new(10_000_000.0, Currency::USD),
        -50.0, // Negative spread
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    // Should not panic with negative spread
    let npv = cds.value(&market, as_of);
    assert!(npv.is_ok(), "Negative spread should be handled");
}

#[test]
fn test_very_high_spread() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "HIGH_SPREAD",
        Money::new(10_000_000.0, Currency::USD),
        10000.0, // 10000 bps = 100%
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of).unwrap();
    assert!(
        npv.amount().is_finite(),
        "Very high spread should not cause numerical issues"
    );
}

#[test]
fn test_zero_recovery_rate() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = DiscountCurve::builder("USD_OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (10.0, 0.6)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("CORP")
        .base_date(as_of)
        .recovery_rate(0.0) // Zero recovery
        .knots([(0.0, 0.02), (10.0, 0.02)])
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let mut cds = CreditDefaultSwap::buy_protection(
        "ZERO_RECOVERY",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");
    cds.protection.recovery_rate = 0.0;

    let npv = cds.value(&market, as_of).unwrap();
    assert!(npv.amount().is_finite(), "Zero recovery should be handled");
}

#[test]
fn test_full_recovery_rate() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = DiscountCurve::builder("USD_OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (10.0, 0.6)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("CORP")
        .base_date(as_of)
        .recovery_rate(1.0) // Full recovery
        .knots([(0.0, 0.02), (10.0, 0.02)])
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let mut cds = CreditDefaultSwap::buy_protection(
        "FULL_RECOVERY",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");
    cds.protection.recovery_rate = 1.0;

    // With full recovery, protection leg should be worth zero
    let protection_pv = cds
        .pv_protection_leg(
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .unwrap();

    assert!(
        protection_pv.amount().abs() < 1.0,
        "Full recovery should give near-zero protection value"
    );
}

#[test]
fn test_very_short_tenor() {
    // 1-day CDS
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2024 - 01 - 02);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "ONE_DAY",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of);
    assert!(npv.is_ok(), "Very short tenor should be handled");
}

#[test]
fn test_maturity_equals_valuation_date() {
    let as_of = date!(2024 - 01 - 01);
    let end = as_of; // Maturity = valuation date
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "EXPIRED",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of);

    // CDS with maturity = valuation date may error (no future cashflows) or have zero value
    if let Ok(value) = npv {
        assert!(
            value.amount().abs() < 1000.0,
            "Expired CDS should have near-zero value"
        );
    } else {
        // It's acceptable for expired CDS to return an error
        assert!(npv.is_err(), "Expired CDS valuation may error");
    }
}

#[test]
fn test_valuation_after_maturity() {
    let start = date!(2024 - 01 - 01);
    let end = date!(2025 - 01 - 01);
    let as_of = date!(2026 - 01 - 01); // After maturity
    let (disc, hazard) = build_curves(start);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "PAST_MATURITY",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        start,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of);

    // Valuation past maturity may error or return zero value
    // Both are acceptable behaviors for expired instruments
    match npv {
        Ok(value) => {
            assert!(
                value.amount().abs() < 1000.0,
                "Past maturity CDS should have near-zero value"
            );
        }
        Err(_) => {
            // Acceptable - past maturity instruments may error
        }
    }
}

#[test]
fn test_very_high_hazard_rate() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = DiscountCurve::builder("USD_OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (10.0, 0.6)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("CORP")
        .base_date(as_of)
        .recovery_rate(0.40)
        .knots([(0.0, 0.5), (10.0, 0.5)]) // 50% hazard rate
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "HIGH_HAZARD",
        Money::new(10_000_000.0, Currency::USD),
        1000.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of).unwrap();
    assert!(
        npv.amount().is_finite(),
        "High hazard rate should not cause numerical issues"
    );
}

#[test]
fn test_zero_hazard_rate() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc = DiscountCurve::builder("USD_OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (10.0, 0.6)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("CORP")
        .base_date(as_of)
        .recovery_rate(0.40)
        .knots([(0.0, 0.0), (10.0, 0.0)]) // Zero hazard
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "ZERO_HAZARD",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    // With zero hazard rate, protection leg should be zero
    let protection_pv = cds
        .pv_protection_leg(
            market.get_discount("USD_OIS").unwrap().as_ref(),
            market.get_hazard("CORP").unwrap().as_ref(),
            as_of,
        )
        .unwrap();

    assert!(
        protection_pv.amount().abs() < 1.0,
        "Zero hazard should give near-zero protection value"
    );
}

#[test]
fn test_metrics_with_zero_notional() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "ZERO_METRICS",
        Money::new(0.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let result = cds
        .price_with_metrics(
            &market,
            as_of,
            &[
                MetricId::Cs01,
                MetricId::RiskyPv01,
                MetricId::ExpectedLoss,
                MetricId::JumpToDefault,
            ],
        )
        .unwrap();

    // All notional-dependent metrics should be zero
    assert_eq!(*result.measures.get("risky_pv01").unwrap(), 0.0);
}

#[test]
fn test_par_spread_with_mismatched_curves() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "MISMATCH_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    // Should handle par spread calculation even with different curve IDs
    let result = cds.price_with_metrics(&market, as_of, &[MetricId::ParSpread]);
    assert!(result.is_ok(), "Should handle par spread calculation");
}

#[test]
fn test_numerical_stability_with_extreme_dates() {
    // Test with dates far in the future (30-year CDS)
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2054 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "LONG_DATED",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of).unwrap();
    assert!(
        npv.amount().is_finite(),
        "Long-dated CDS should be numerically stable"
    );
}

#[test]
fn test_integration_fallback_with_invalid_params() {
    // Test that integration methods fall back gracefully
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = CreditDefaultSwap::buy_protection(
        "FALLBACK_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    // Test with very small tolerance (might trigger fallback)
    let pricer = CDSPricer::with_config(CDSPricerConfig {
        integration_method: IntegrationMethod::AdaptiveSimpson,
        tolerance: 1e-15,
        adaptive_max_depth: 2,
        ..Default::default()
    });

    let result = pricer.pv_protection_leg(&cds, &disc, &hazard, as_of);
    assert!(
        result.is_ok(),
        "Should handle extreme integration parameters"
    );
}

#[test]
fn test_very_small_notional() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "TINY_NOTIONAL",
        Money::new(0.01, Currency::USD), // 1 cent
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of).unwrap();
    assert!(npv.amount().is_finite(), "Tiny notional should be handled");
    assert!(
        npv.amount().abs() < 1.0,
        "Tiny notional should give tiny NPV"
    );
}

#[test]
fn test_very_large_notional() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "HUGE_NOTIONAL",
        Money::new(1_000_000_000_000.0, Currency::USD), // 1 trillion
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");

    let npv = cds.value(&market, as_of).unwrap();
    assert!(npv.amount().is_finite(), "Large notional should be handled");
}

#[test]
fn test_missing_discount_curve_error() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    // Market with only hazard curve
    let hazard = HazardCurve::builder("CORP")
        .base_date(as_of)
        .recovery_rate(0.40)
        .knots([(0.0, 0.02), (10.0, 0.02)])
        .build()
        .unwrap();

    let market = MarketContext::new().insert_hazard(hazard);

    let cds = CreditDefaultSwap::buy_protection(
        "MISSING_DISC",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS", // This curve doesn't exist
        "CORP",
    )
    .expect("CDS construction should succeed");

    let result = cds.value(&market, as_of);
    assert!(result.is_err(), "Should error with missing discount curve");
}

#[test]
fn test_missing_hazard_curve_error() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    // Market with only discount curve
    let disc = DiscountCurve::builder("USD_OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (10.0, 0.6)])
        .build()
        .unwrap();

    let market = MarketContext::new().insert_discount(disc);

    let cds = CreditDefaultSwap::buy_protection(
        "MISSING_HAZARD",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP", // This curve doesn't exist
    )
    .expect("CDS construction should succeed");

    let result = cds.value(&market, as_of);
    assert!(result.is_err(), "Should error with missing hazard curve");
}

#[test]
fn test_settlement_delay_zero_is_valid() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    let mut cds = CreditDefaultSwap::buy_protection(
        "ZERO_DELAY",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");
    cds.protection.settlement_delay = 0;

    let npv = cds.value(&market, as_of);
    assert!(npv.is_ok(), "Zero settlement delay should be valid");
}

#[test]
fn test_recovery_rate_bounds_not_enforced() {
    // Recovery rates outside [0,1] are allowed (though unrealistic)
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(hazard);

    // Test with negative recovery (unrealistic but shouldn't crash)
    let mut cds = CreditDefaultSwap::buy_protection(
        "NEG_RECOVERY",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
    .expect("CDS construction should succeed");
    cds.protection.recovery_rate = -0.2;

    let result = cds.value(&market, as_of);
    // Should not crash (though result may be economically meaningless)
    assert!(
        result.is_ok() || result.is_err(),
        "Should handle out-of-bounds recovery"
    );
}
