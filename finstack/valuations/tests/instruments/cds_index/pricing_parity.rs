#![cfg(feature = "slow")]
//! CDS Index pricing parity tests between single-curve and constituents modes.
//!
//! Market Standard: When all constituents have identical credit quality,
//! single-curve and constituents pricing should produce similar results.
//!
//! ## Tolerance Rationale
//!
//! The tolerances in these tests account for legitimate algorithmic differences:
//!
//! - **NPV/Leg PV (5%)**: Different curve lookups (single HZ-INDEX vs HZ1..HZn)
//!   with identical underlying hazard rates. Small interpolation differences.
//!
//! - **Par Spread (2%)**: Pure ratio of protection/premium PV, so curve
//!   interpolation differences partially cancel. Should be tighter.
//!
//! - **Risky PV01 (3%)**: Annuity calculation aggregates over N periods.
//!
//! - **CS01 (5%)**: Single-curve bumps HZ-INDEX; constituents mode bumps
//!   each HZ1..HZn independently. Method should be identical per-CDS.
//!
//! Tests cover:
//! - NPV parity with equal hazard rates
//! - Par spread consistency
//! - Risky PV01 consistency
//! - CS01 consistency
//! - Protection/premium leg consistency

use super::test_utils::*;
use finstack_valuations::instruments::cds_index::IndexPricing;
use finstack_valuations::instruments::common::traits::Instrument;
use time::macros::date;

#[test]
fn test_npv_parity_equal_hazards() {
    // Market Standard: Single curve vs constituents pricing with equal hazards
    //
    // Both modes use flat 0.015 hazard rate and 40% recovery, so NPV should
    // be very close. Differences arise from curve lookup paths only.
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    // Single curve index
    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);

    // Constituents index with equal hazards
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let npv_single = idx_single.value(&ctx, as_of).unwrap();
    let npv_constituents = idx_constituents.value(&ctx, as_of).unwrap();

    // 5% tolerance: identical hazard rates, only curve lookup path differs
    assert_relative_eq(
        npv_single.amount(),
        npv_constituents.amount(),
        0.05,
        "NPV parity between pricing modes",
    );
}

#[test]
fn test_par_spread_parity_equal_hazards() {
    // Test: Par spread consistency across modes
    //
    // Par spread = Protection PV / Risky Annuity is a ratio, so curve
    // interpolation differences should largely cancel out.
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let par_single = idx_single.par_spread(&ctx, as_of).unwrap();
    let par_constituents = idx_constituents.par_spread(&ctx, as_of).unwrap();

    // 2% tolerance: ratio metric, errors cancel partially
    assert_relative_eq(par_single, par_constituents, 0.02, "Par spread parity");
}

#[test]
fn test_risky_pv01_parity_equal_hazards() {
    // Test: Risky PV01 consistency across modes
    //
    // Risky PV01 (annuity) is sum of discounted survival probabilities.
    // Aggregation across N constituents vs single calculation.
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let rpv01_single = idx_single.risky_pv01(&ctx, as_of).unwrap();
    let rpv01_constituents = idx_constituents.risky_pv01(&ctx, as_of).unwrap();

    // 3% tolerance: annuity aggregation differences
    assert_relative_eq(rpv01_single, rpv01_constituents, 0.03, "Risky PV01 parity");
}

#[test]
fn test_cs01_parity_equal_hazards() {
    // Test: CS01 consistency across modes
    //
    // CS01 uses finite-difference bumping of hazard curves:
    // - Single-curve: bumps HZ-INDEX by 1bp
    // - Constituents: bumps each HZ1..HZn by 1bp independently
    //
    // With identical hazard rates, the sum of constituent CS01s should
    // equal the single-curve CS01.
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let cs01_single = idx_single.cs01(&ctx, as_of).unwrap();
    let cs01_constituents = idx_constituents.cs01(&ctx, as_of).unwrap();

    // 5% tolerance: aggregation of per-constituent CS01 vs single curve
    assert_relative_eq(cs01_single, cs01_constituents, 0.05, "CS01 parity");
}

#[test]
fn test_protection_leg_parity_equal_hazards() {
    // Test: Protection leg PV consistency
    //
    // Protection leg integrates (1 - Recovery) × hazard × discount over time.
    // With identical hazard rates and recovery, results should be close.
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let prot_single = idx_single.pv_protection_leg(&ctx, as_of).unwrap();
    let prot_constituents = idx_constituents.pv_protection_leg(&ctx, as_of).unwrap();

    // 5% tolerance: integration methodology
    assert_relative_eq(
        prot_single.amount(),
        prot_constituents.amount(),
        0.05,
        "Protection leg parity",
    );
}

#[test]
fn test_premium_leg_parity_equal_hazards() {
    // Test: Premium leg PV consistency
    //
    // Premium leg is spread × risky annuity + accrued-on-default.
    // With identical parameters, aggregation should match.
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let prem_single = idx_single.pv_premium_leg(&ctx, as_of).unwrap();
    let prem_constituents = idx_constituents.pv_premium_leg(&ctx, as_of).unwrap();

    // 5% tolerance: aggregation methodology
    assert_relative_eq(
        prem_single.amount(),
        prem_constituents.amount(),
        0.05,
        "Premium leg parity",
    );
}

#[test]
fn test_mode_switching_preserves_basics() {
    // Test: Switching pricing mode maintains core fields
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let mut idx = standard_single_curve_index("CDX-SWITCH", start, end, 10_000_000.0);
    assert_eq!(idx.pricing, IndexPricing::SingleCurve);

    // Switch to constituents
    let credits = equal_weight_constituents(5)
        .into_iter()
        .map(|c| c.credit)
        .collect::<Vec<_>>();
    idx = idx.with_constituents_equal_weight(credits);

    // Verify basics preserved
    assert_eq!(idx.id(), "CDX-SWITCH");
    assert_eq!(idx.notional.amount(), 10_000_000.0);
    assert_eq!(idx.pricing, IndexPricing::Constituents);
}

#[test]
fn test_parity_with_different_constituent_counts() {
    // Test: Parity holds across different constituent counts
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);

    for count in [3, 5, 10, 25] {
        let id = format!("CDX-N{}", count);
        let idx_const = standard_constituents_index(&id, start, end, 10_000_000.0, count);

        let ctx = multi_constituent_market_context(as_of, count);

        let npv_single = idx_single.value(&ctx, as_of).unwrap();
        let npv_const = idx_const.value(&ctx, as_of).unwrap();

        let msg = format!("NPV parity with {} constituents", count);
        // 5% tolerance: constituent count shouldn't affect parity with identical hazards
        assert_relative_eq(npv_single.amount(), npv_const.amount(), 0.05, &msg);
    }
}

#[test]
fn test_parity_across_maturities() {
    // Test: Parity holds across different maturities
    let start = date!(2025 - 01 - 01);
    let as_of = start;

    let maturities = [
        ("3Y", date!(2028 - 01 - 01)),
        ("5Y", date!(2030 - 01 - 01)),
        ("7Y", date!(2032 - 01 - 01)),
    ];

    for (label, end) in maturities {
        let single_id = format!("CDX-SINGLE-{}", label);
        let const_id = format!("CDX-CONST-{}", label);
        let idx_single = standard_single_curve_index(&single_id, start, end, 10_000_000.0);
        let idx_const = standard_constituents_index(&const_id, start, end, 10_000_000.0, 5);

        let ctx = multi_constituent_market_context(as_of, 5);

        let npv_single = idx_single.value(&ctx, as_of).unwrap();
        let npv_const = idx_const.value(&ctx, as_of).unwrap();

        let msg = format!("NPV parity for {} maturity", label);
        // 5% tolerance: maturity shouldn't affect parity with identical hazards
        assert_relative_eq(npv_single.amount(), npv_const.amount(), 0.05, &msg);
    }
}

#[test]
fn test_parity_with_different_notionals() {
    // Test: Relative parity holds across notional sizes
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;
    let ctx = multi_constituent_market_context(as_of, 5);

    for notional in [1_000_000.0, 10_000_000.0, 100_000_000.0] {
        let single_id = format!("CDX-SINGLE-{}", notional);
        let const_id = format!("CDX-CONST-{}", notional);
        let idx_single = standard_single_curve_index(&single_id, start, end, notional);
        let idx_const = standard_constituents_index(&const_id, start, end, notional, 5);

        let npv_single = idx_single.value(&ctx, as_of).unwrap();
        let npv_const = idx_const.value(&ctx, as_of).unwrap();

        let msg = format!("NPV parity for ${:.0} notional", notional);
        // 5% tolerance: notional scales linearly, shouldn't affect relative parity
        assert_relative_eq(npv_single.amount(), npv_const.amount(), 0.05, &msg);
    }
}

#[test]
fn test_mode_independence_of_par_spread() {
    // Test: Par spread (a ratio) should be nearly identical across modes
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_const = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let par_single = idx_single.par_spread(&ctx, as_of).unwrap();
    let par_const = idx_const.par_spread(&ctx, as_of).unwrap();

    // Par spread is a pure ratio and should match very closely
    assert_relative_eq(par_single, par_const, 0.03, "Par spread mode independence");
}

#[test]
fn test_consistent_sign_conventions_across_modes() {
    // Test: Sign conventions consistent between modes
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_const = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let npv_single = idx_single.value(&ctx, as_of).unwrap().amount();
    let npv_const = idx_const.value(&ctx, as_of).unwrap().amount();

    // Both should have the same sign
    assert!(
        npv_single.signum() == npv_const.signum(),
        "NPV signs should match: single={}, const={}",
        npv_single,
        npv_const
    );
}
