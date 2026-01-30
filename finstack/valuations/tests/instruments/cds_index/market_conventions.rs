//! CDS Index market conventions tests.
//!
//! Tests cover:
//! - CDX NA IG conventions
//! - CDX NA HY conventions
//! - iTraxx Europe conventions
//! - Standard fixed coupons
//! - Payment frequencies
//! - Day count conventions
//! - Business day conventions

use super::test_utils::*;
use finstack_core::dates::{DayCount, Tenor};
use finstack_valuations::instruments::credit_derivatives::cds::{CDSConvention, PayReceive};
use finstack_valuations::instruments::credit_derivatives::cds_index::CDSIndex;
use finstack_valuations::instruments::credit_derivatives::cds_index::{
    CDSIndexConstructionParams, CDSIndexParams,
};
use finstack_valuations::instruments::CreditParams;
use finstack_valuations::instruments::Instrument;
use time::macros::date;

#[test]
fn test_cdx_na_ig_standard_conventions() {
    // Market Standard: CDX NA IG uses ISDA NA conventions
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0);
    let idx = CDSIndex::new_standard(
        "CDX.NA.IG.42",
        &params,
        &CDSIndexConstructionParams::new(
            standard_construction_params(10_000_000.0).notional,
            PayReceive::PayFixed,
            CDSConvention::IsdaNa,
        ),
        start,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-INDEX"),
        "USD-OIS",
        "HZ-INDEX",
    )
    .expect("valid test parameters");

    assert_eq!(idx.convention, CDSConvention::IsdaNa);
    assert_eq!(idx.premium.freq, Tenor::quarterly());
    assert_eq!(idx.premium.dc, DayCount::Act360);
}

#[test]
fn test_cdx_na_hy_standard_conventions() {
    // Market Standard: CDX NA HY uses ISDA NA conventions
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let params = CDSIndexParams::cdx_na_hy(39, 1, 500.0);
    let idx = CDSIndex::new_standard(
        "CDX.NA.HY.39",
        &params,
        &CDSIndexConstructionParams::new(
            standard_construction_params(10_000_000.0).notional,
            PayReceive::PayFixed,
            CDSConvention::IsdaNa,
        ),
        start,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-INDEX"),
        "USD-OIS",
        "HZ-INDEX",
    )
    .expect("valid test parameters");

    assert_eq!(idx.convention, CDSConvention::IsdaNa);
    assert_eq!(idx.premium.freq, Tenor::quarterly());
}

#[test]
fn test_itraxx_europe_standard_conventions() {
    // Market Standard: iTraxx Europe uses ISDA European conventions
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let params = CDSIndexParams::itraxx_europe(41, 1, 25.0);
    let idx = CDSIndex::new_standard(
        "iTraxx.Europe.41",
        &params,
        &CDSIndexConstructionParams::new(
            standard_construction_params(10_000_000.0).notional,
            PayReceive::PayFixed,
            CDSConvention::IsdaEu,
        ),
        start,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-INDEX"),
        "EUR-OIS",
        "HZ-INDEX",
    )
    .expect("valid test parameters");

    assert_eq!(idx.convention, CDSConvention::IsdaEu);
}

#[test]
fn test_cdx_ig_standard_coupon() {
    // Market Standard: CDX IG standard coupon is 100 bps
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0);

    assert_eq!(params.fixed_coupon_bp, 100.0);
}

#[test]
fn test_cdx_hy_standard_coupon() {
    // Market Standard: CDX HY standard coupon is 500 bps
    let params = CDSIndexParams::cdx_na_hy(39, 1, 500.0);

    assert_eq!(params.fixed_coupon_bp, 500.0);
}

#[test]
fn test_itraxx_standard_coupon() {
    // Market Standard: iTraxx standard coupon is typically 25 bps
    let params = CDSIndexParams::itraxx_europe(41, 1, 25.0);

    assert_eq!(params.fixed_coupon_bp, 25.0);
}

#[test]
fn test_quarterly_payment_frequency() {
    // Market Standard: CDS indices pay quarterly
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let idx = standard_single_curve_index("CDX-FREQ", start, end, 10_000_000.0);

    assert_eq!(idx.premium.freq, Tenor::quarterly());
}

#[test]
fn test_act_360_day_count() {
    // Market Standard: NA CDS use Act/360 day count
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let idx = standard_single_curve_index("CDX-DC", start, end, 10_000_000.0);

    assert_eq!(idx.premium.dc, DayCount::Act360);
}

#[test]
fn test_standard_maturity_dates() {
    // Market Standard: CDS indices roll to standard IMM dates (Mar/Jun/Sep/Dec 20)
    // This test verifies we can create indices with standard maturities
    let start = date!(2025 - 03 - 20);

    let maturities = [
        date!(2026 - 03 - 20), // 1Y
        date!(2028 - 03 - 20), // 3Y
        date!(2030 - 03 - 20), // 5Y
        date!(2032 - 03 - 20), // 7Y
        date!(2035 - 03 - 20), // 10Y
    ];

    for (i, end) in maturities.iter().enumerate() {
        let id = format!("CDX-IMM-{}", i);
        let idx = standard_single_curve_index(&id, start, *end, 10_000_000.0);

        assert!(idx.premium.end == *end);
    }
}

#[test]
fn test_cdx_series_numbering() {
    // Test: Series and version numbering
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0);

    assert_eq!(params.series, 42);
    assert_eq!(params.version, 1);
}

#[test]
fn test_index_factor_default() {
    // Market Standard: Index factor defaults to 1.0 (no defaults)
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let idx = standard_single_curve_index("CDX-FACTOR", start, end, 10_000_000.0);

    assert_eq!(idx.index_factor, 1.0);
}

#[test]
fn test_index_factor_application() {
    // Market Standard: Index factor < 1.0 for seasoned indices with defaults
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let factor = 0.96; // 4% defaults
    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0).with_index_factor(factor);

    let idx = CDSIndex::new_standard(
        "CDX-SEASONED",
        &params,
        &CDSIndexConstructionParams::new(
            standard_construction_params(10_000_000.0).notional,
            PayReceive::PayFixed,
            CDSConvention::IsdaNa,
        ),
        start,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-INDEX"),
        "USD-OIS",
        "HZ-INDEX",
    )
    .expect("valid test parameters");

    assert_eq!(idx.index_factor, factor);
}

#[test]
fn test_recovery_rate_standard() {
    // Market Standard: Senior unsecured recovery typically 40%
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let idx = standard_single_curve_index("CDX-REC", start, end, 10_000_000.0);

    // Recovery should be standard senior unsecured
    assert_eq!(idx.protection.recovery_rate, 0.40);
}

#[test]
fn test_cdx_vs_itraxx_naming() {
    // Test: CDX and iTraxx naming conventions
    let cdx_ig = CDSIndexParams::cdx_na_ig(42, 1, 100.0);
    let cdx_hy = CDSIndexParams::cdx_na_hy(39, 1, 500.0);
    let itraxx = CDSIndexParams::itraxx_europe(41, 1, 25.0);

    assert_eq!(cdx_ig.index_name, "CDX.NA.IG");
    assert_eq!(cdx_hy.index_name, "CDX.NA.HY");
    assert_eq!(itraxx.index_name, "iTraxx Europe");
}

#[test]
fn test_standard_constituent_count_cdx_ig() {
    // Market Standard: CDX IG has 125 constituents
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let idx = standard_constituents_index("CDX.IG", start, end, 10_000_000.0, 125);

    assert_eq!(idx.constituents.len(), 125);
}

#[test]
fn test_standard_constituent_count_cdx_hy() {
    // Market Standard: CDX HY has 100 constituents
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let idx = standard_constituents_index("CDX.HY", start, end, 10_000_000.0, 100);

    assert_eq!(idx.constituents.len(), 100);
}

#[test]
fn test_equal_weight_convention() {
    // Market Standard: Index constituents are typically equal-weighted
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let idx = standard_constituents_index("CDX-EW", start, end, 10_000_000.0, 125);

    let expected_weight = 1.0 / 125.0;
    for constituent in &idx.constituents {
        assert_relative_eq(
            constituent.weight,
            expected_weight,
            0.001,
            "Equal weight per constituent",
        );
    }
}

#[test]
fn test_pricing_with_standard_conventions() {
    // Test: Pricing works with standard market conventions
    let start = date!(2025 - 03 - 20); // IMM date
    let end = date!(2030 - 03 - 20); // 5Y IMM
    let as_of = start;

    let params = CDSIndexParams::cdx_na_ig(42, 1, 100.0);
    let idx = CDSIndex::new_standard(
        "CDX.NA.IG.42",
        &params,
        &CDSIndexConstructionParams::new(
            standard_construction_params(10_000_000.0).notional,
            PayReceive::PayFixed,
            CDSConvention::IsdaNa,
        ),
        start,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-INDEX"),
        "USD-OIS",
        "HZ-INDEX",
    )
    .expect("valid test parameters");

    let ctx = standard_market_context(as_of);
    let result = idx.value(&ctx, as_of);

    assert!(
        result.is_ok(),
        "Standard conventions should price successfully"
    );
}

#[test]
fn test_settlement_delay_standard() {
    // Market Standard: CDS indices typically have T+3 settlement
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let idx = standard_single_curve_index("CDX-SETTLE", start, end, 10_000_000.0);

    // Settlement delay from convention
    assert_eq!(idx.protection.settlement_delay, 3);
}

#[test]
fn test_fixed_coupon_in_premium_leg() {
    // Test: Fixed coupon flows to premium leg
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);

    let fixed_coupon = 100.0; // 100 bps
    let params = CDSIndexParams::cdx_na_ig(42, 1, fixed_coupon);

    let idx = CDSIndex::new_standard(
        "CDX-COUPON",
        &params,
        &CDSIndexConstructionParams::new(
            standard_construction_params(10_000_000.0).notional,
            PayReceive::PayFixed,
            CDSConvention::IsdaNa,
        ),
        start,
        end,
        &CreditParams::corporate_standard("INDEX", "HZ-INDEX"),
        "USD-OIS",
        "HZ-INDEX",
    )
    .expect("valid test parameters");

    assert_eq!(idx.premium.spread_bp, rust_decimal::Decimal::from(100));
}
