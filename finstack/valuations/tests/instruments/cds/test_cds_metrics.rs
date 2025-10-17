//! CDS metrics calculation tests.
//!
//! Comprehensive tests for all CDS metrics including CS01, DV01,
//! expected loss, jump-to-default, par spread, and risk sensitivities.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::cds::CreditDefaultSwap;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn build_test_discount(rate: f64, base: Date, id: &str) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
        ])
        .build()
        .unwrap()
}

fn build_test_hazard(hz: f64, rec: f64, base: Date, id: &str) -> HazardCurve {
    HazardCurve::builder(id)
        .base_date(base)
        .recovery_rate(rec)
        .knots([(0.0, hz), (1.0, hz), (5.0, hz), (10.0, hz)])
        .build()
        .unwrap()
}

fn create_test_market(as_of: Date) -> MarketContext {
    MarketContext::new()
        .insert_discount(build_test_discount(0.05, as_of, "USD_OIS"))
        .insert_hazard(build_test_hazard(0.015, 0.40, as_of, "CORP"))
}

fn create_test_cds(as_of: Date, maturity: Date) -> CreditDefaultSwap {
    CreditDefaultSwap::buy_protection(
        "METRICS_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        maturity,
        "USD_OIS",
        "CORP",
    )
}

#[test]
fn test_cs01_positive_for_buyer() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::Cs01])
        .unwrap();

    let cs01 = *result.measures.get("cs01").unwrap();

    assert!(cs01 > 0.0, "CS01 should be positive for protection buyer");
    assert!(cs01 < 1_000_000.0, "CS01 should be reasonable magnitude");
}

#[test]
fn test_cs01_hazard_vs_risky_pv01_consistency() {
    // Validate hazard-bump CS01 is consistent with risky PV01 approximation
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);

    // Ensure hazard builder used in market has explicit base/daycount/recovery
    let market = {
        let mut ctx = MarketContext::new();
        let disc = DiscountCurve::builder("USD_OIS")
            .base_date(as_of)
            .day_count(DayCount::Act360)
            .knots([(0.0, 1.0), (10.0, (-(0.05_f64 * 10.0_f64)).exp())])
            .build()
            .unwrap();
        let hazard = HazardCurve::builder(cds.protection.credit_id.clone())
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (5.0, 0.012), (10.0, 0.013)])
            .build()
            .unwrap();
        ctx = ctx.insert_discount(disc).insert_hazard(hazard);
        ctx
    };

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::RiskyPv01, MetricId::HazardCs01])
        .unwrap();

    let risky_pv01 = *result.measures.get("risky_pv01").unwrap();
    let hazard_cs01 = *result.measures.get("hazard_cs01").unwrap();

    // Both measures should be positive for protection buyer
    assert!(risky_pv01 > 0.0, "Risky PV01 should be positive");
    assert!(hazard_cs01 > 0.0, "Hazard CS01 should be positive");

    // They should be within reasonable tolerance in magnitude for small bumps
    let rel_diff = (hazard_cs01 - risky_pv01).abs() / risky_pv01.max(1e-6);
    // Allow wider tolerance; methods differ (hazard bump vs spread PV01 proxy)
    assert!(rel_diff < 0.45);
}

#[test]
fn test_risky_pv01_positive() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::RiskyPv01])
        .unwrap();

    let risky_pv01 = *result.measures.get("risky_pv01").unwrap();

    assert!(risky_pv01 > 0.0, "Risky PV01 should be positive");

    // For $10MM, 5Y CDS, risky PV01 should be in reasonable range
    assert!(
        risky_pv01 > 1_000.0 && risky_pv01 < 100_000.0,
        "Risky PV01={:.2} outside expected range",
        risky_pv01
    );
}

#[test]
fn test_par_spread_metric() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::ParSpread])
        .unwrap();

    let par_spread = *result.measures.get("par_spread").unwrap();

    assert!(par_spread > 0.0, "Par spread should be positive");
    assert!(par_spread.is_finite(), "Par spread should be finite");

    // Reasonable range for investment grade
    assert!(
        par_spread > 10.0 && par_spread < 500.0,
        "Par spread={:.2} bps outside typical IG range",
        par_spread
    );
}

#[test]
fn test_protection_leg_pv_metric() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::ProtectionLegPv])
        .unwrap();

    let prot_pv = *result.measures.get("protection_leg_pv").unwrap();

    assert!(prot_pv > 0.0, "Protection leg PV should be positive");
}

#[test]
fn test_premium_leg_pv_metric() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::PremiumLegPv])
        .unwrap();

    let prem_pv = *result.measures.get("premium_leg_pv").unwrap();

    assert!(
        prem_pv > 0.0,
        "Premium leg PV should be positive for positive spread"
    );
}

#[test]
fn test_expected_loss_positive() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::ExpectedLoss])
        .unwrap();

    let expected_loss = *result.measures.get("expected_loss").unwrap();

    assert!(expected_loss > 0.0, "Expected loss should be positive");

    // Should be less than notional × LGD
    let max_loss = 10_000_000.0 * 0.6; // 60% LGD
    assert!(
        expected_loss < max_loss,
        "Expected loss should be less than max possible loss"
    );
}

#[test]
fn test_expected_loss_formula() {
    // EL = Notional × PD × LGD
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let hazard_rate = 0.02; // 2% per year
    let recovery = 0.40;

    let mut cds = create_test_cds(as_of, maturity);
    cds.protection.recovery_rate = recovery;

    let market = MarketContext::new()
        .insert_discount(build_test_discount(0.05, as_of, "USD_OIS"))
        .insert_hazard(build_test_hazard(hazard_rate, recovery, as_of, "CORP"));

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::ExpectedLoss])
        .unwrap();

    let expected_loss = *result.measures.get("expected_loss").unwrap();

    // For 5Y: PD ≈ 1 - exp(-λ×T) ≈ 1 - exp(-0.02×5) ≈ 0.095
    // EL ≈ 10MM × 0.6 × 0.095 ≈ $570,000
    assert!(
        expected_loss > 400_000.0 && expected_loss < 800_000.0,
        "Expected loss={:.0} outside expected range",
        expected_loss
    );
}

#[test]
fn test_jump_to_default_positive_for_buyer() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::JumpToDefault])
        .unwrap();

    let jtd = *result.measures.get("jump_to_default").unwrap();

    // Protection buyer gains on default
    assert!(jtd > 0.0, "JTD should be positive for protection buyer");
}

#[test]
fn test_jump_to_default_negative_for_seller() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = CreditDefaultSwap::sell_protection(
        "JTD_SELLER",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        maturity,
        "USD_OIS",
        "CORP",
    );

    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::JumpToDefault])
        .unwrap();

    let jtd = *result.measures.get("jump_to_default").unwrap();

    // Protection seller loses on default
    assert!(jtd < 0.0, "JTD should be negative for protection seller");
}

#[test]
fn test_jump_to_default_magnitude() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let mut cds = create_test_cds(as_of, maturity);
    cds.protection.recovery_rate = 0.40;

    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::JumpToDefault])
        .unwrap();

    let jtd = *result.measures.get("jump_to_default").unwrap();

    // JTD ≈ Notional × LGD = $10MM × 0.6 = $6MM
    assert!(
        jtd > 5_500_000.0 && jtd < 6_500_000.0,
        "JTD={:.0} should be approximately $6MM",
        jtd
    );
}

// Note: DefaultProbability metric is not currently implemented for CDS
// The probability can be derived from the hazard curve directly if needed

#[test]
fn test_dv01_metric() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();

    let dv01 = *result.measures.get("dv01").unwrap();

    assert!(dv01 > 0.0, "DV01 should be positive");
    assert!(dv01.is_finite(), "DV01 should be finite");
}

#[test]
fn test_theta_metric() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();

    let theta = *result.measures.get("theta").unwrap();

    assert!(theta.is_finite(), "Theta should be finite");
}

#[test]
fn test_hazard_cs01_metric() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::HazardCs01])
        .unwrap();

    let hazard_cs01 = *result.measures.get("hazard_cs01").unwrap();

    assert!(hazard_cs01 > 0.0, "Hazard CS01 should be positive");
    assert!(hazard_cs01.is_finite(), "Hazard CS01 should be finite");
}

#[test]
fn test_multiple_metrics_simultaneously() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let metrics = vec![
        MetricId::Cs01,
        MetricId::RiskyPv01,
        MetricId::ParSpread,
        MetricId::ProtectionLegPv,
        MetricId::PremiumLegPv,
        MetricId::ExpectedLoss,
        MetricId::JumpToDefault,
        MetricId::Dv01,
        MetricId::Theta,
        MetricId::HazardCs01,
    ];

    let result = cds.price_with_metrics(&market, as_of, &metrics).unwrap();

    // All metrics should be present
    assert!(result.measures.contains_key("cs01"));
    assert!(result.measures.contains_key("risky_pv01"));
    assert!(result.measures.contains_key("par_spread"));
    assert!(result.measures.contains_key("protection_leg_pv"));
    assert!(result.measures.contains_key("premium_leg_pv"));
    assert!(result.measures.contains_key("expected_loss"));
    assert!(result.measures.contains_key("jump_to_default"));
    assert!(result.measures.contains_key("dv01"));
    assert!(result.measures.contains_key("theta"));
    assert!(result.measures.contains_key("hazard_cs01"));
}

#[test]
fn test_pv01_alias_matches_risky_pv01() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::RiskyPv01, MetricId::custom("pv01")],
        )
        .unwrap();

    let risky_pv01 = *result.measures.get("risky_pv01").unwrap();
    let pv01 = *result.measures.get("pv01").unwrap();

    assert_eq!(risky_pv01, pv01, "pv01 alias should match risky_pv01");
}

#[test]
fn test_metrics_scale_with_notional() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds_small = CreditDefaultSwap::buy_protection(
        "SMALL",
        Money::new(1_000_000.0, Currency::USD),
        100.0,
        as_of,
        maturity,
        "USD_OIS",
        "CORP",
    );

    let cds_large = CreditDefaultSwap::buy_protection(
        "LARGE",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        maturity,
        "USD_OIS",
        "CORP",
    );

    let market = create_test_market(as_of);

    let metrics = vec![
        MetricId::RiskyPv01,
        MetricId::ExpectedLoss,
        MetricId::JumpToDefault,
    ];

    let result_small = cds_small
        .price_with_metrics(&market, as_of, &metrics)
        .unwrap();
    let result_large = cds_large
        .price_with_metrics(&market, as_of, &metrics)
        .unwrap();

    for metric in ["risky_pv01", "expected_loss", "jump_to_default"] {
        let val_small = *result_small.measures.get(metric).unwrap();
        let val_large = *result_large.measures.get(metric).unwrap();
        let ratio = val_large / val_small;

        assert!(
            (ratio - 10.0).abs() < 0.1,
            "{} should scale with notional, got ratio {}",
            metric,
            ratio
        );
    }
}

#[test]
fn test_cs01_increases_with_tenor() {
    let as_of = date!(2024 - 01 - 01);
    let market = create_test_market(as_of);

    let mut cs01_values = Vec::new();

    for years in [1, 3, 5, 10] {
        let maturity = Date::from_calendar_date(2024 + years, time::Month::January, 1).unwrap();
        let cds = create_test_cds(as_of, maturity);

        let result = cds
            .price_with_metrics(&market, as_of, &[MetricId::Cs01])
            .unwrap();

        let cs01 = *result.measures.get("cs01").unwrap();
        cs01_values.push((years, cs01));
    }

    // CS01 should generally increase with tenor
    for i in 1..cs01_values.len() {
        assert!(
            cs01_values[i].1 > cs01_values[i - 1].1,
            "CS01 should increase with tenor: {}Y={:.2} <= {}Y={:.2}",
            cs01_values[i - 1].0,
            cs01_values[i - 1].1,
            cs01_values[i].0,
            cs01_values[i].1
        );
    }
}

#[test]
fn test_expected_loss_increases_with_tenor() {
    let as_of = date!(2024 - 01 - 01);
    let market = create_test_market(as_of);

    let mut el_values = Vec::new();

    for years in [1, 3, 5, 10] {
        let maturity = Date::from_calendar_date(2024 + years, time::Month::January, 1).unwrap();
        let cds = create_test_cds(as_of, maturity);

        let result = cds
            .price_with_metrics(&market, as_of, &[MetricId::ExpectedLoss])
            .unwrap();

        let el = *result.measures.get("expected_loss").unwrap();
        el_values.push((years, el));
    }

    // Expected loss should increase with tenor (more time for default)
    for i in 1..el_values.len() {
        assert!(
            el_values[i].1 > el_values[i - 1].1,
            "Expected loss should increase with tenor"
        );
    }
}

#[test]
fn test_bucketed_dv01_metric() {
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, maturity);
    let market = create_test_market(as_of);

    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    // Bucketed DV01 should be computed
    let bucketed_dv01 = result.measures.get("bucketed_dv01");
    assert!(bucketed_dv01.is_some(), "Bucketed DV01 should be available");
}
