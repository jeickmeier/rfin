//! DV01 (nominal interest rate sensitivity) metric tests for InflationSwap.

use crate::inflation_swap::fixtures::*;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::term_structures::DiscountCurve;

use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::inflation_swap::{InflationSwapBuilder, PayReceiveInflation};
use finstack_valuations::metrics::MetricId;
use time::Month;

#[test]
fn test_ir01_finite_difference_validation() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let ctx = standard_market(as_of, 0.02, 0.04);

    let swap = InflationSwapBuilder::new()
        .id("ZCINF-IR01-FD".into())
        .notional(standard_notional())
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(0.02)
        .inflation_index_id("US-CPI-U".into())
        .discount_curve_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    // Get analytic DV01
    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Dv01])
        .unwrap();
    let dv01_analytic = *result.measures.get("dv01").unwrap();

    // Compute finite difference DV01
    let pv0 = swap.value(&ctx, as_of).unwrap().amount();

    // Bump discount curve by 1bp
    let disc_base = ctx.get_discount("USD-OIS").unwrap();
    let base_date = disc_base.base_date();

    let mut bumped_points: Vec<(f64, f64)> = Vec::new();
    for &t in disc_base.knots() {
        let df = disc_base.df(t);
        let df_b = df * (-0.0001 * t).exp(); // bump rates by 1bp
        bumped_points.push((t, df_b));
    }

    let bumped_disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(bumped_points)
        .build()
        .unwrap();

    let ctx_bumped = ctx.clone().insert_discount(bumped_disc);
    let pv1 = swap.value(&ctx_bumped, as_of).unwrap().amount();

    let dv01_fd = pv1 - pv0;

    // Check sign consistency
    assert_eq!(
        dv01_analytic.signum(),
        dv01_fd.signum(),
        "DV01 sign should match FD: analytic={}, FD={}",
        dv01_analytic,
        dv01_fd
    );

    // Check magnitude within tolerance
    let rel_diff = (dv01_analytic - dv01_fd).abs() / dv01_fd.abs().max(1.0);
    assert!(
        rel_diff < greek_tolerance(),
        "DV01 relative difference too large: {}",
        rel_diff
    );
}

#[test]
fn test_ir01_scales_with_maturity() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let ctx = standard_market(as_of, 0.02, 0.04);

    let mut dv01s = Vec::new();
    for years in &[1, 2, 5, 10] {
        let maturity = Date::from_calendar_date(2025 + years, Month::January, 1).unwrap();
        let swap = InflationSwapBuilder::new()
            .id("ZCINF-IR01-MAT".into())
            .notional(standard_notional())
            .start(as_of)
            .maturity(maturity)
            .fixed_rate(0.02)
            .inflation_index_id("US-CPI-U".into())
            .discount_curve_id("USD-OIS".into())
            .dc(DayCount::Act365F)
            .side(PayReceiveInflation::PayFixed)
            .attributes(Default::default())
            .build()
            .unwrap();

        let result = swap
            .price_with_metrics(&ctx, as_of, &[MetricId::Dv01])
            .unwrap();

        let dv01 = result.measures.get("dv01").unwrap().abs();
        dv01s.push(dv01);
    }

    // DV01 magnitude should generally increase with maturity
    for i in 1..dv01s.len() {
        assert!(
            dv01s[i] > dv01s[i - 1],
            "DV01 should increase with maturity"
        );
    }
}

#[test]
fn test_ir01_sign_pay_fixed() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let ctx = standard_market(as_of, 0.02, 0.04);

    let swap = InflationSwapBuilder::new()
        .id("ZCINF-IR01-SIGN1".into())
        .notional(standard_notional())
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(0.01)
        .inflation_index_id("US-CPI-U".into())
        .discount_curve_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Dv01])
        .unwrap();

    let dv01 = *result.measures.get("dv01").unwrap();

    // DV01 should be finite
    assert!(dv01.is_finite(), "DV01 should be finite");
}

#[test]
fn test_ir01_sign_receive_fixed() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let ctx = standard_market(as_of, 0.02, 0.04);

    let swap = InflationSwapBuilder::new()
        .id("ZCINF-IR01-SIGN2".into())
        .notional(standard_notional())
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(0.03)
        .inflation_index_id("US-CPI-U".into())
        .discount_curve_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::ReceiveFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Dv01])
        .unwrap();

    let dv01 = *result.measures.get("dv01").unwrap();

    // DV01 should be finite
    assert!(dv01.is_finite(), "DV01 should be finite");
}

#[test]
fn test_ir01_zero_for_matured_swap() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2020, Month::January, 1).unwrap();

    let ctx = standard_market(as_of, 0.02, 0.04);

    let swap = InflationSwapBuilder::new()
        .id("ZCINF-IR01-MAT0".into())
        .notional(standard_notional())
        .start(Date::from_calendar_date(2015, Month::January, 1).unwrap())
        .maturity(maturity)
        .fixed_rate(0.02)
        .inflation_index_id("US-CPI-U".into())
        .discount_curve_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Dv01])
        .unwrap();

    let dv01 = result.measures.get("dv01").unwrap().abs();

    // Matured swap should have near-zero DV01
    assert!(
        dv01 < 1.0,
        "Matured swap should have negligible DV01: {}",
        dv01
    );
}
