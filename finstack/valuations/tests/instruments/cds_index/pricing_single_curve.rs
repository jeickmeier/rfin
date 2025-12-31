//! CDS Index single-curve pricing mode tests.
//!
//! Tests cover:
//! - NPV calculations in single-curve mode
//! - Leg PV calculations (premium and protection)
//! - Par spread computation
//! - Risky PV01 calculation
//! - CS01 approximation
//! - Sign conventions for buy/sell protection
//! - Notional scaling behavior

use super::test_utils::*;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::credit_derivatives::cds::PayReceive;
use finstack_valuations::instruments::credit_derivatives::cds_index::{CDSIndex, IndexPricing};
use finstack_valuations::instruments::Instrument;
use time::macros::date;

#[test]
fn test_single_curve_npv_calculation() {
    // Test: NPV calculation in single-curve mode
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-NPV", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let npv = idx.value(&ctx, as_of).unwrap();

    assert_eq!(npv.currency(), Currency::USD);
    assert!(npv.amount().is_finite());
}

#[test]
fn test_single_curve_npv_components() {
    // Test: NPV = Protection PV - Premium PV (for protection buyer)
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-COMPONENTS", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let npv = idx.npv(&ctx, as_of).unwrap();
    let pv_prot = idx.pv_protection_leg(&ctx, as_of).unwrap();
    let pv_prem = idx.pv_premium_leg(&ctx, as_of).unwrap();

    let expected_npv = pv_prot.checked_sub(pv_prem).unwrap();
    assert_money_approx_eq(npv, expected_npv, 1.0, "NPV = Protection - Premium");
}

#[test]
fn test_single_curve_protection_leg_positive() {
    // Test: Protection leg PV should be positive for protection buyer
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-PROT", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let pv_prot = idx.pv_protection_leg(&ctx, as_of).unwrap();

    assert!(
        pv_prot.amount() > 0.0,
        "Protection leg PV should be positive, got {}",
        pv_prot.amount()
    );
}

#[test]
fn test_single_curve_premium_leg_positive() {
    // Test: Premium leg PV should be positive
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-PREM", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let pv_prem = idx.pv_premium_leg(&ctx, as_of).unwrap();

    assert!(
        pv_prem.amount() > 0.0,
        "Premium leg PV should be positive, got {}",
        pv_prem.amount()
    );
}

#[test]
fn test_single_curve_par_spread() {
    // Test: Par spread calculation
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-PAR", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let par_spread = idx.par_spread(&ctx, as_of).unwrap();

    assert_positive(par_spread, "Par spread");
    // For ~1.5% hazard, 40% recovery: spread ≈ 90 bps
    assert_in_range(par_spread, 50.0, 150.0, "Par spread in reasonable range");
}

#[test]
fn test_single_curve_risky_pv01() {
    // Test: Risky PV01 calculation
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-RPV01", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let rpv01 = idx.risky_pv01(&ctx, as_of).unwrap();

    assert_positive(rpv01, "Risky PV01");
    // For $10MM, 5Y: expect $4,000-$5,000
    assert_in_range(rpv01, 3_500.0, 5_500.0, "Risky PV01 magnitude");
}

#[test]
fn test_single_curve_cs01() {
    // Test: CS01 (credit sensitivity) calculation
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-CS01", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let cs01 = idx.cs01(&ctx, as_of).unwrap();

    assert_positive(cs01, "CS01");
    // CS01 should be positive and reasonable
    assert!(cs01 > 100.0, "CS01 should be meaningful for $10MM notional");
}

#[test]
fn test_single_curve_buy_vs_sell_protection() {
    // Test: Sign conventions for buy vs sell protection
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    // Buy protection (pay fixed)
    let idx_buy = standard_single_curve_index("CDX-BUY", start, end, 10_000_000.0);

    // Sell protection (receive fixed)
    let mut params_sell = standard_construction_params(10_000_000.0);
    params_sell.side = PayReceive::ReceiveFixed;

    let idx_sell = CDSIndex::new_standard(
        "CDX-SELL",
        &standard_cdx_params(),
        &params_sell,
        start,
        end,
        &finstack_valuations::instruments::CreditParams::corporate_standard("INDEX", "HZ-INDEX"),
        "USD-OIS",
        "HZ-INDEX",
    )
    .expect("valid test parameters");

    let ctx = standard_market_context(as_of);

    let npv_buy = idx_buy.value(&ctx, as_of).unwrap();
    let npv_sell = idx_sell.value(&ctx, as_of).unwrap();

    // Buy and sell protection should have opposite signs
    assert!(
        npv_buy.amount() * npv_sell.amount() < 0.0,
        "Buy and sell protection NPVs should have opposite signs: buy={}, sell={}",
        npv_buy.amount(),
        npv_sell.amount()
    );

    // Magnitudes should be approximately equal
    assert_relative_eq(
        npv_buy.amount().abs(),
        npv_sell.amount().abs(),
        0.01,
        "Buy/sell NPV magnitudes",
    );
}

#[test]
fn test_single_curve_npv_scales_with_notional() {
    // Test: NPV scales linearly with notional
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_10mm = standard_single_curve_index("CDX-10MM", start, end, 10_000_000.0);
    let idx_20mm = standard_single_curve_index("CDX-20MM", start, end, 20_000_000.0);

    let npv_10mm = idx_10mm.value(&ctx, as_of).unwrap().amount();
    let npv_20mm = idx_20mm.value(&ctx, as_of).unwrap().amount();

    assert_linear_scaling(npv_10mm, 10_000_000.0, npv_20mm, 20_000_000.0, "NPV", 0.01);
}

#[test]
fn test_single_curve_risky_pv01_scales_with_notional() {
    // Test: Risky PV01 scales linearly with notional
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_10mm = standard_single_curve_index("CDX-10MM", start, end, 10_000_000.0);
    let idx_20mm = standard_single_curve_index("CDX-20MM", start, end, 20_000_000.0);

    let rpv01_10mm = idx_10mm.risky_pv01(&ctx, as_of).unwrap();
    let rpv01_20mm = idx_20mm.risky_pv01(&ctx, as_of).unwrap();

    assert_linear_scaling(
        rpv01_10mm,
        10_000_000.0,
        rpv01_20mm,
        20_000_000.0,
        "Risky PV01",
        0.01,
    );
}

#[test]
fn test_single_curve_cs01_scales_with_notional() {
    // Test: CS01 scales linearly with notional
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_10mm = standard_single_curve_index("CDX-10MM", start, end, 10_000_000.0);
    let idx_20mm = standard_single_curve_index("CDX-20MM", start, end, 20_000_000.0);

    let cs01_10mm = idx_10mm.cs01(&ctx, as_of).unwrap();
    let cs01_20mm = idx_20mm.cs01(&ctx, as_of).unwrap();

    assert_linear_scaling(
        cs01_10mm,
        10_000_000.0,
        cs01_20mm,
        20_000_000.0,
        "CS01",
        0.05,
    );
}

#[test]
fn test_single_curve_par_spread_independent_of_notional() {
    // Test: Par spread should be independent of notional
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_1mm = standard_single_curve_index("CDX-1MM", start, end, 1_000_000.0);
    let idx_100mm = standard_single_curve_index("CDX-100MM", start, end, 100_000_000.0);

    let par_1mm = idx_1mm.par_spread(&ctx, as_of).unwrap();
    let par_100mm = idx_100mm.par_spread(&ctx, as_of).unwrap();

    assert_relative_eq(
        par_1mm,
        par_100mm,
        0.001,
        "Par spread should be notional-independent",
    );
}

#[test]
fn test_single_curve_pricing_mode_verification() {
    // Test: Single-curve index has correct pricing mode
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let idx = standard_single_curve_index("CDX-MODE", start, end, 10_000_000.0);

    assert_eq!(idx.pricing, IndexPricing::SingleCurve);
    assert!(idx.constituents.is_empty());
}

#[test]
fn test_single_curve_maturity_impact() {
    // Test: Longer maturity increases protection value
    let start = date!(2025 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_3y = standard_single_curve_index("CDX-3Y", start, date!(2028 - 01 - 01), 10_000_000.0);
    let idx_5y = standard_single_curve_index("CDX-5Y", start, date!(2030 - 01 - 01), 10_000_000.0);
    let idx_10y =
        standard_single_curve_index("CDX-10Y", start, date!(2035 - 01 - 01), 10_000_000.0);

    let prot_3y = idx_3y.pv_protection_leg(&ctx, as_of).unwrap().amount();
    let prot_5y = idx_5y.pv_protection_leg(&ctx, as_of).unwrap().amount();
    let prot_10y = idx_10y.pv_protection_leg(&ctx, as_of).unwrap().amount();

    assert!(
        prot_3y < prot_5y && prot_5y < prot_10y,
        "Protection PV should increase with maturity: 3Y={}, 5Y={}, 10Y={}",
        prot_3y,
        prot_5y,
        prot_10y
    );
}

#[test]
fn test_single_curve_risky_pv01_maturity_impact() {
    // Test: Longer maturity increases risky PV01
    let start = date!(2025 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx_3y = standard_single_curve_index("CDX-3Y", start, date!(2028 - 01 - 01), 10_000_000.0);
    let idx_5y = standard_single_curve_index("CDX-5Y", start, date!(2030 - 01 - 01), 10_000_000.0);

    let rpv01_3y = idx_3y.risky_pv01(&ctx, as_of).unwrap();
    let rpv01_5y = idx_5y.risky_pv01(&ctx, as_of).unwrap();

    assert!(
        rpv01_3y < rpv01_5y,
        "Risky PV01 should increase with maturity: 3Y={}, 5Y={}",
        rpv01_3y,
        rpv01_5y
    );
}

#[test]
fn test_single_curve_synthetic_cds_pricing_equivalence() {
    // Test: Direct pricing matches synthetic CDS pricing
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx = standard_single_curve_index("CDX-EQUIV", start, end, 10_000_000.0);
    let synthetic_cds = idx.to_synthetic_cds();

    let idx_npv = idx.value(&ctx, as_of).unwrap();
    let cds_npv = synthetic_cds.value(&ctx, as_of).unwrap();

    assert_money_approx_eq(
        idx_npv,
        cds_npv,
        1.0,
        "Index and synthetic CDS NPV should match",
    );
}

#[test]
fn test_single_curve_zero_notional() {
    // Test: Zero notional produces zero NPV
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let idx = standard_single_curve_index("CDX-ZERO", start, end, 0.0);
    let npv = idx.value(&ctx, as_of).unwrap();

    assert_eq!(npv.amount(), 0.0, "Zero notional should produce zero NPV");
}

#[test]
fn test_single_curve_upfront_payment() {
    // Test: Upfront payment affects NPV
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = standard_market_context(as_of);

    let mut idx = standard_single_curve_index("CDX-UPFRONT", start, end, 10_000_000.0);
    let npv_no_upfront = idx.value(&ctx, as_of).unwrap();

    // Add upfront payment
    let upfront = Money::new(100_000.0, Currency::USD);
    idx.pricing_overrides.upfront_payment = Some(upfront);

    let npv_with_upfront = idx.value(&ctx, as_of).unwrap();
    let expected = npv_no_upfront.checked_add(upfront).unwrap();

    assert_money_approx_eq(
        npv_with_upfront,
        expected,
        1.0,
        "Upfront payment should be added to NPV",
    );
}
