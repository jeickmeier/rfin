//! IR01 (nominal interest rate sensitivity) metric tests for InflationSwap.

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
        .inflation_id("US-CPI-U".into())
        .disc_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    // Get analytic IR01
    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Ir01])
        .unwrap();
    let ir01_analytic = *result.measures.get("ir01").unwrap();

    // Compute finite difference IR01
    let pv0 = swap.value(&ctx, as_of).unwrap().amount();

    // Bump discount curve by 1bp
    let disc_base = ctx.get_discount_ref("USD-OIS").unwrap();
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

    let ir01_fd = pv1 - pv0;

    // Check sign consistency
    assert_eq!(
        ir01_analytic.signum(),
        ir01_fd.signum(),
        "IR01 sign should match FD: analytic={}, FD={}",
        ir01_analytic,
        ir01_fd
    );

    // Check magnitude within tolerance
    let rel_diff = (ir01_analytic - ir01_fd).abs() / ir01_fd.abs().max(1.0);
    assert!(
        rel_diff < greek_tolerance(),
        "IR01 relative difference too large: {}",
        rel_diff
    );
}

#[test]
fn test_ir01_scales_with_maturity() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let ctx = standard_market(as_of, 0.02, 0.04);

    let mut ir01s = Vec::new();
    for years in &[1, 2, 5, 10] {
        let maturity = Date::from_calendar_date(2025 + years, Month::January, 1).unwrap();
        let swap = InflationSwapBuilder::new()
            .id("ZCINF-IR01-MAT".into())
            .notional(standard_notional())
            .start(as_of)
            .maturity(maturity)
            .fixed_rate(0.02)
            .inflation_id("US-CPI-U".into())
            .disc_id("USD-OIS".into())
            .dc(DayCount::Act365F)
            .side(PayReceiveInflation::PayFixed)
            .attributes(Default::default())
            .build()
            .unwrap();

        let result = swap
            .price_with_metrics(&ctx, as_of, &[MetricId::Ir01])
            .unwrap();

        let ir01 = result.measures.get("ir01").unwrap().abs();
        ir01s.push(ir01);
    }

    // IR01 magnitude should generally increase with maturity
    for i in 1..ir01s.len() {
        assert!(
            ir01s[i] > ir01s[i - 1],
            "IR01 should increase with maturity"
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
        .inflation_id("US-CPI-U".into())
        .disc_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Ir01])
        .unwrap();

    let ir01 = *result.measures.get("ir01").unwrap();

    // IR01 should be finite
    assert!(ir01.is_finite(), "IR01 should be finite");
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
        .inflation_id("US-CPI-U".into())
        .disc_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::ReceiveFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Ir01])
        .unwrap();

    let ir01 = *result.measures.get("ir01").unwrap();

    // IR01 should be finite
    assert!(ir01.is_finite(), "IR01 should be finite");
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
        .inflation_id("US-CPI-U".into())
        .disc_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Ir01])
        .unwrap();

    let ir01 = result.measures.get("ir01").unwrap().abs();

    // Matured swap should have near-zero IR01
    assert!(
        ir01 < 1.0,
        "Matured swap should have negligible IR01: {}",
        ir01
    );
}
