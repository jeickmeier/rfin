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
//! For a par bond on a flat curve (coupon = yield), the difference is minimal
//! (typically <2%) because both methods are measuring the same underlying
//! rate sensitivity. The 2% tolerance accounts for:
//! - Minor compounding convention differences
//! - Numerical precision in bump-and-reprice vs analytical formula

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::{Bond, CashflowSpec};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn create_flat_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
            (30.0, (-rate * 30.0).exp()),
        ])
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

    let curve = create_flat_curve(0.06, as_of, "USD-OIS");
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
    let as_of = date!(2024 - 01 - 01);
    let maturity = date!(2029 - 01 - 01);

    // Use pricing override to ensure bond is at par (clean price = 100)
    let pricing_overrides = PricingOverrides::default().with_clean_price(100.0);

    let bond = Bond::builder()
        .id("DV01_REL".into())
        .notional(Money::new(100.0, Currency::USD))
        .cashflow_spec(CashflowSpec::fixed(
            0.08,
            Frequency::annual(),
            DayCount::Act365F,
        ))
        .issue(as_of)
        .maturity(maturity)
        .discount_curve_id("USD_DISC".into())
        .pricing_overrides(pricing_overrides)
        .call_put_opt(None)
        .custom_cashflows_opt(None)
        .attributes(Default::default())
        .build()
        .unwrap();

    let curve = create_flat_curve(0.08, as_of, "USD_DISC");
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
    // For a par bond on a flat curve:
    // - Compounding difference: e^0.05 vs (1+0.025)^2 → ~0.6% difference
    // - Curve vs yield at par: minimal difference
    // - Convexity for 1bp: negligible
    //
    // Combined effect for par/flat case: <2%
    let approx_dv01 = -(price * mod_dur * 0.0001);
    let relative_diff = ((dv01 - approx_dv01) / approx_dv01).abs();

    assert!(
        relative_diff < 0.02, // 2% tolerance - actual diff ~1.4% due to compounding conventions
        "DV01={:.6} differs from duration estimate {:.6} by {:.2}%",
        dv01,
        approx_dv01,
        relative_diff * 100.0
    );
}
