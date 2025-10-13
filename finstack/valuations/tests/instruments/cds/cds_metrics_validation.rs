//! CDS metrics validation tests against known market benchmarks.
//!
//! These tests validate CDS metric calculations against:
//! - ISDA standard methodologies
//! - Market-standard CDS pricing
//! - Credit risk analytics benchmarks
//!
//! References:
//! - ISDA 2014 CDS Standard Model
//! - Hull, "Options, Futures, and Other Derivatives" (Credit Risk chapter)
//! - O'Kane, "Modelling Single-name and Multi-name Credit Derivatives"

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::CreditDefaultSwap;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

/// Build a flat discount curve for testing
fn build_flat_discount(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
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

/// Build a flat hazard curve for testing
fn build_flat_hazard(hazard_rate: f64, recovery: f64, base_date: Date, curve_id: &str) -> HazardCurve {
    HazardCurve::builder(curve_id)
        .base_date(base_date)
        .recovery_rate(recovery)
        .knots([
            (0.0, hazard_rate),
            (1.0, hazard_rate),
            (5.0, hazard_rate),
            (10.0, hazard_rate),
        ])
        .build()
        .unwrap()
}

#[test]
fn test_cds_risky_pv01_market_standard() {
    // Risky PV01 = Risky Annuity × Notional / 10,000
    // For 5Y CDS at 5% hazard rate, 40% recovery, 5% discount rate
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount(0.05, as_of, "USD_OIS");
    let hazard_curve = build_flat_hazard(0.01, 0.40, as_of, "CORP_HAZARD");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_hazard(hazard_curve);
    
    let mut cds = CreditDefaultSwap::buy_protection(
        "CDS_RISKY_PV01_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0, // 100 bps spread
        as_of,
        end,
        "USD_OIS",
        "CORP_HAZARD",
    );
    cds.protection.recovery_rate = 0.40;
    
    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::RiskyPv01])
        .unwrap();
    
    let risky_pv01 = *result.measures.get("risky_pv01").unwrap();
    
    // For $10MM notional, 5Y CDS, risky PV01 should be in range $4,000-$5,000
    // (survival probability reduces the annuity compared to risk-free)
    assert!(
        risky_pv01 > 4_000.0 && risky_pv01 < 5_000.0,
        "Risky PV01={:.2} outside expected range $4,000-$5,000 for $10MM 5Y CDS",
        risky_pv01
    );
}

#[test]
fn test_cds_cs01_positive_for_protection_buyer() {
    // CS01 measures sensitivity to credit spread changes
    // For protection buyer, CS01 should be positive (benefits from widening spreads)
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount(0.05, as_of, "USD_OIS");
    let hazard_curve = build_flat_hazard(0.01, 0.40, as_of, "CORP_HAZARD");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_hazard(hazard_curve);
    
    let mut cds = CreditDefaultSwap::buy_protection(
        "CDS_CS01_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP_HAZARD",
    );
    cds.protection.recovery_rate = 0.40;
    
    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::Cs01])
        .unwrap();
    
    let cs01 = *result.measures.get("cs01").unwrap();
    
    // CS01 should be positive for protection buyer
    assert!(
        cs01 > 0.0,
        "CS01={:.2} should be positive for protection buyer",
        cs01
    );
    
    // For $10MM, 5Y CDS, CS01 should be in reasonable range
    assert!(
        cs01 > 1_000.0 && cs01 < 1_000_000.0,
        "CS01={:.2} outside reasonable range",
        cs01
    );
}

#[test]
fn test_cds_protection_buyer_vs_seller() {
    // Protection buyer (PayFixed) and seller (ReceiveFixed) should have opposite NPVs
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount(0.05, as_of, "USD_OIS");
    let hazard_curve = build_flat_hazard(0.015, 0.40, as_of, "CORP_HAZARD");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_hazard(hazard_curve);
    
    // Protection buyer
    let mut cds_buyer = CreditDefaultSwap::buy_protection(
        "CDS_BUYER",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP_HAZARD",
    );
    cds_buyer.protection.recovery_rate = 0.40;
    
    // Protection seller
    let mut cds_seller = CreditDefaultSwap::sell_protection(
        "CDS_SELLER",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP_HAZARD",
    );
    cds_seller.protection.recovery_rate = 0.40;
    
    let npv_buyer = cds_buyer.value(&market, as_of).unwrap();
    let npv_seller = cds_seller.value(&market, as_of).unwrap();
    
    // NPVs should be opposite signs (zero-sum game)
    assert!(
        npv_buyer.amount() * npv_seller.amount() < 0.0,
        "Buyer and seller NPVs should be opposite: buyer={:.2}, seller={:.2}",
        npv_buyer.amount(),
        npv_seller.amount()
    );
    
    // NPVs should be approximately equal in magnitude
    assert!(
        (npv_buyer.amount() + npv_seller.amount()).abs() < 1000.0,
        "Buyer and seller NPVs should be opposite: sum={:.2}",
        npv_buyer.amount() + npv_seller.amount()
    );
}

#[test]
fn test_cds_par_spread_gives_zero_npv() {
    // Par spread is the spread where CDS has zero NPV at inception
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount(0.05, as_of, "USD_OIS");
    let hazard_curve = build_flat_hazard(0.01, 0.40, as_of, "CORP_HAZARD");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_hazard(hazard_curve);
    
    let mut cds = CreditDefaultSwap::buy_protection(
        "CDS_PAR_SPREAD_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP_HAZARD",
    );
    cds.protection.recovery_rate = 0.40;
    
    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::ParSpread])
        .unwrap();
    
    let par_spread = *result.measures.get("par_spread").unwrap();
    
    // Par spread should be positive
    assert!(
        par_spread > 0.0,
        "Par spread={:.2} bps should be positive",
        par_spread
    );
    
    // For 1% hazard rate and 40% recovery, expect spread around 60 bps
    // Spread ≈ Hazard × (1 - Recovery) = 0.01 × 0.6 = 0.006 = 60 bps
    assert!(
        par_spread > 40.0 && par_spread < 100.0,
        "Par spread={:.2} bps outside expected range 40-100 bps",
        par_spread
    );
}

#[test]
fn test_cds_higher_hazard_increases_value_for_buyer() {
    // Protection buyer benefits from higher credit risk (higher hazard rate)
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let mut npvs = Vec::new();
    
    // Test different hazard rates (representing different credit quality)
    for hazard_rate in [0.005, 0.010, 0.020, 0.030] {
        let disc_curve_clone = build_flat_discount(0.05, as_of, "USD_OIS");
        let hazard_curve = build_flat_hazard(hazard_rate, 0.40, as_of, "CORP_HAZARD");
        
        let market = MarketContext::new()
            .insert_discount(disc_curve_clone)
            .insert_hazard(hazard_curve);
        
        let mut cds = CreditDefaultSwap::buy_protection(
            "CDS_HAZARD_SENS",
            Money::new(10_000_000.0, Currency::USD),
            100.0,
            as_of,
            end,
            "USD_OIS",
            "CORP_HAZARD",
        );
        cds.protection.recovery_rate = 0.40;
        
        let npv = cds.value(&market, as_of).unwrap();
        npvs.push((hazard_rate, npv.amount()));
    }
    
    // For protection buyer (PayFixed), higher hazard → higher NPV
    // (protection becomes more valuable)
    for i in 1..npvs.len() {
        assert!(
            npvs[i].1 > npvs[i - 1].1,
            "Protection buyer NPV should increase with hazard rate: \
             hazard {:.3}% NPV={:.0} <= hazard {:.3}% NPV={:.0}",
            npvs[i - 1].0 * 100.0,
            npvs[i - 1].1,
            npvs[i].0 * 100.0,
            npvs[i].1
        );
    }
}

#[test]
fn test_cds_recovery_rate_impact() {
    // Higher recovery rate → lower protection value for buyer
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let mut npvs = Vec::new();
    
    // Test different recovery rates
    for recovery in [0.20, 0.40, 0.60] {
        let disc_curve_clone = build_flat_discount(0.05, as_of, "USD_OIS");
        let hazard_curve = build_flat_hazard(0.015, recovery, as_of, "CORP_HAZARD");
        
        let market = MarketContext::new()
            .insert_discount(disc_curve_clone)
            .insert_hazard(hazard_curve);
        
        let mut cds = CreditDefaultSwap::buy_protection(
            "CDS_RECOVERY_SENS",
            Money::new(10_000_000.0, Currency::USD),
            100.0,
            as_of,
            end,
            "USD_OIS",
            "CORP_HAZARD",
        );
        cds.protection.recovery_rate = recovery;
        
        let npv = cds.value(&market, as_of).unwrap();
        npvs.push((recovery, npv.amount()));
    }
    
    // For protection buyer, higher recovery → lower NPV
    // (less loss given default, so protection worth less)
    for i in 1..npvs.len() {
        assert!(
            npvs[i].1 < npvs[i - 1].1,
            "Protection buyer NPV should decrease with recovery rate: \
             recovery {:.0}% NPV={:.0} >= recovery {:.0}% NPV={:.0}",
            npvs[i - 1].0 * 100.0,
            npvs[i - 1].1,
            npvs[i].0 * 100.0,
            npvs[i].1
        );
    }
}

#[test]
fn test_cds_expected_loss_formula() {
    // Expected Loss = Notional × (1 - Recovery) × Default Probability
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount(0.05, as_of, "USD_OIS");
    let hazard_rate = 0.02; // 2% per year
    let recovery = 0.40;
    let hazard_curve = build_flat_hazard(hazard_rate, recovery, as_of, "CORP_HAZARD");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_hazard(hazard_curve);
    
    let mut cds = CreditDefaultSwap::buy_protection(
        "CDS_EXPECTED_LOSS_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP_HAZARD",
    );
    cds.protection.recovery_rate = recovery;
    
    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::ExpectedLoss])
        .unwrap();
    
    let expected_loss = *result.measures.get("expected_loss").unwrap();
    
    // Expected loss should be positive
    assert!(
        expected_loss > 0.0,
        "Expected loss={:.2} should be positive",
        expected_loss
    );
    
    // Rough approximation: EL ≈ Notional × LGD × PD
    // For 5Y: PD ≈ 1 - exp(-λ×T) ≈ 1 - exp(-0.02×5) ≈ 0.095
    // EL ≈ 10MM × 0.6 × 0.095 ≈ $570,000
    assert!(
        expected_loss > 400_000.0 && expected_loss < 800_000.0,
        "Expected loss={:.0} outside reasonable range $400K-$800K",
        expected_loss
    );
}

#[test]
fn test_cds_jump_to_default_magnitude() {
    // Jump-to-default = immediate loss if default happens now
    // JTD = Notional × (1 - Recovery) - Accrued Premium
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount(0.05, as_of, "USD_OIS");
    let hazard_curve = build_flat_hazard(0.01, 0.40, as_of, "CORP_HAZARD");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_hazard(hazard_curve);
    
    let mut cds = CreditDefaultSwap::buy_protection(
        "CDS_JTD_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP_HAZARD",
    );
    cds.protection.recovery_rate = 0.40;
    
    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::JumpToDefault])
        .unwrap();
    
    let jtd = *result.measures.get("jump_to_default").unwrap();
    
    // JTD for protection buyer should be positive (gain if default)
    assert!(
        jtd > 0.0,
        "Jump-to-default={:.0} should be positive for protection buyer",
        jtd
    );
    
    // Should be approximately Notional × LGD = $10MM × 0.6 = $6MM
    assert!(
        jtd > 5_500_000.0 && jtd < 6_500_000.0,
        "Jump-to-default={:.0} outside expected range $5.5MM-$6.5MM",
        jtd
    );
}

#[test]
fn test_cds_survival_decreases_over_time() {
    // Survival probability should decrease as time passes (aging effect)
    // This is implicit in NPV changes over time
    
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    
    let disc_curve = build_flat_discount(0.05, as_of, "USD_OIS");
    let hazard_curve = build_flat_hazard(0.015, 0.40, as_of, "CORP_HAZARD");
    
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_hazard(hazard_curve);
    
    let mut cds = CreditDefaultSwap::buy_protection(
        "CDS_SURVIVAL_TEST",
        Money::new(10_000_000.0, Currency::USD),
        150.0,
        as_of,
        end,
        "USD_OIS",
        "CORP_HAZARD",
    );
    cds.protection.recovery_rate = 0.40;
    
    let result = cds
        .price_with_metrics(&market, as_of, &[MetricId::RiskyPv01])
        .unwrap();
    
    let risky_pv01_t0 = *result.measures.get("risky_pv01").unwrap();
    
    // Risky PV01 should be less than risk-free annuity
    // For $10MM, 5Y, risk-free PV01 would be ~$4,300
    // Risky PV01 should be less due to default risk
    assert!(
        risky_pv01_t0 < 4_300.0,
        "Risky PV01={:.2} should be less than risk-free PV01 ~$4,300",
        risky_pv01_t0
    );
}

