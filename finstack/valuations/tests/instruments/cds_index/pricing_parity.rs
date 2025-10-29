//! CDS Index pricing parity tests between single-curve and constituents modes.
//!
//! Market Standard: When all constituents have identical credit quality,
//! single-curve and constituents pricing should produce similar results.
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

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_npv_parity_equal_hazards() {
    // Market Standard: Single curve vs constituents pricing with equal hazards
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

    // Allow 10% tolerance due to different calculation paths
    assert_relative_eq(
        npv_single.amount(),
        npv_constituents.amount(),
        0.10,
        "NPV parity between pricing modes",
    );
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_par_spread_parity_equal_hazards() {
    // Test: Par spread consistency across modes
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let par_single = idx_single.par_spread(&ctx, as_of).unwrap();
    let par_constituents = idx_constituents.par_spread(&ctx, as_of).unwrap();

    // Par spread should be very similar (within 5%)
    assert_relative_eq(par_single, par_constituents, 0.05, "Par spread parity");
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_risky_pv01_parity_equal_hazards() {
    // Test: Risky PV01 consistency across modes
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let rpv01_single = idx_single.risky_pv01(&ctx, as_of).unwrap();
    let rpv01_constituents = idx_constituents.risky_pv01(&ctx, as_of).unwrap();

    // Risky PV01 should be very close (within 5%)
    assert_relative_eq(rpv01_single, rpv01_constituents, 0.05, "Risky PV01 parity");
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_cs01_parity_equal_hazards() {
    // Test: CS01 consistency across modes
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let cs01_single = idx_single.cs01(&ctx, as_of).unwrap();
    let cs01_constituents = idx_constituents.cs01(&ctx, as_of).unwrap();

    // CS01 should be reasonably close (within 15% due to bump approximation)
    assert_relative_eq(cs01_single, cs01_constituents, 0.15, "CS01 parity");
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_protection_leg_parity_equal_hazards() {
    // Test: Protection leg PV consistency
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let prot_single = idx_single.pv_protection_leg(&ctx, as_of).unwrap();
    let prot_constituents = idx_constituents.pv_protection_leg(&ctx, as_of).unwrap();

    assert_relative_eq(
        prot_single.amount(),
        prot_constituents.amount(),
        0.10,
        "Protection leg parity",
    );
}

#[ignore = "QuantLib parity: comprehensive validation"]
#[test]
fn test_premium_leg_parity_equal_hazards() {
    // Test: Premium leg PV consistency
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx_single = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let idx_constituents = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);

    let ctx = multi_constituent_market_context(as_of, 5);

    let prem_single = idx_single.pv_premium_leg(&ctx, as_of).unwrap();
    let prem_constituents = idx_constituents.pv_premium_leg(&ctx, as_of).unwrap();

    assert_relative_eq(
        prem_single.amount(),
        prem_constituents.amount(),
        0.10,
        "Premium leg parity",
    );
}

#[ignore = "QuantLib parity: comprehensive validation"]
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

#[ignore = "QuantLib parity: comprehensive validation"]
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
        assert_relative_eq(npv_single.amount(), npv_const.amount(), 0.15, &msg);
    }
}

#[ignore = "QuantLib parity: comprehensive validation"]
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
        assert_relative_eq(npv_single.amount(), npv_const.amount(), 0.10, &msg);
    }
}

#[ignore = "QuantLib parity: comprehensive validation"]
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
        assert_relative_eq(npv_single.amount(), npv_const.amount(), 0.10, &msg);
    }
}

#[ignore = "QuantLib parity: comprehensive validation"]
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

#[ignore = "QuantLib parity: comprehensive validation"]
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
