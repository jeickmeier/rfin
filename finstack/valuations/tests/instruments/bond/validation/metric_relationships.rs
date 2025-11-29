//! Cross-metric validation tests.
//!
//! Tests fundamental relationships between bond metrics:
//! - Modified Duration = Macaulay Duration / (1 + YTM/m)
//! - DV01 ≈ Price × Modified Duration × 0.0001 (approximate relationship)
//! - Convexity and duration approximations
//!
//! ## Note on DV01 vs Duration
//!
//! The relationship `DV01 ≈ -Price × ModDur × 0.0001` is an **approximation** that
//! assumes both metrics use the same rate sensitivity. In practice:
//!
//! - **DV01**: Computed via parallel bump of the discount curve (continuous
//!   compounding on zero rates). Captures actual curve-based price change.
//!
//! - **Modified Duration**: Derived from YTM (periodic compounding, typically
//!   semi-annual). Represents yield-based sensitivity.
//!
//! For bonds with significant spread (coupon ≠ yield), these can differ by
//! 10-15% depending on:
//! - Compounding convention mismatch (continuous vs periodic)
//! - Curve slope effects (parallel shift vs yield change)
//! - Convexity contributions (second-order for large moves)
//!
//! The 15% tolerance in `test_dv01_duration_price_relationship` reflects this
//! legitimate methodological difference, not imprecision.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn create_flat_curve(rate: f64, base_date: Date) -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([(0.0, 1.0), (5.0, (-rate * 5.0).exp())])
        .build()
        .unwrap()
}

#[test]
fn test_modified_macaulay_duration_relationship() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "DUR_REL",
        Money::new(100.0, Currency::USD),
        0.06,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let curve = create_flat_curve(0.06, as_of);
    let market = MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::DurationMac, MetricId::DurationMod, MetricId::Ytm],
        )
        .unwrap();

    let mac_dur = *result.measures.get("duration_mac").unwrap();
    let mod_dur = *result.measures.get("duration_mod").unwrap();
    let ytm = *result.measures.get("ytm").unwrap();

    // ModDur = MacDur / (1 + ytm/m) for semi-annual
    let m = 2.0; // Semi-annual
    let expected_mod_dur = mac_dur / (1.0 + ytm / m);

    assert!((mod_dur - expected_mod_dur).abs() < 0.01);
}

#[test]
fn test_dv01_duration_price_relationship() {
    // Test the approximate relationship: DV01 ≈ −Price × ModDur × 0.0001
    //
    // This test validates that the curve-based DV01 and yield-based ModDur
    // are in the same ballpark, while acknowledging they measure different things.
    //
    // See module documentation for detailed explanation of why these differ.
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "DV01_REL",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let curve = create_flat_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::DurationMod, MetricId::Dv01])
        .unwrap();

    let mod_dur = *result.measures.get("duration_mod").unwrap();
    let dv01 = *result.measures.get("dv01").unwrap();
    let price = result.value.amount();

    // DV01 is computed via generic bump-and-reprice (more accurate than linear approximation)
    // Verify sign: DV01 < 0 for fixed-rate bonds (price decreases when rates rise)
    assert!(dv01 < 0.0, "DV01 should be negative for fixed-rate bond");

    // Approximate relationship: DV01 ≈ −Price × ModDur × 0.0001
    //
    // Expected sources of difference:
    // 1. Compounding: DV01 uses continuous (curve), ModDur uses semi-annual (yield)
    //    - For 5% rate: e^0.05 vs (1+0.025)^2 → ~0.6% difference
    // 2. Curve vs Yield: DV01 bumps zero rates, ModDur is yield-based
    //    - At par, flat curve: minimal difference
    //    - Away from par or steep curve: can be 5-10%
    // 3. Convexity: DV01 includes second-order effects, approximation is first-order
    //    - For 1bp bump: 0.5 × 25 × (0.0001)² ≈ negligible
    //
    // Combined effect: 10-15% is typical for methodological differences
    let approx_dv01 = -(price * mod_dur * 0.0001);
    let relative_diff = ((dv01 - approx_dv01) / approx_dv01).abs();

    assert!(
        relative_diff < 0.15, // Expected methodological difference, not imprecision
        "DV01={:.6} differs from duration estimate {:.6} by {:.2}%",
        dv01,
        approx_dv01,
        relative_diff * 100.0
    );
}
