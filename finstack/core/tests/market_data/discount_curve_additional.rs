use super::common::{sample_base_date, sample_discount_curve};
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::ExtrapolationPolicy;
use time::Month;

#[test]
fn discount_curve_require_monotonic_enforces_decreasing_dfs() {
    let result = DiscountCurve::builder("BAD")
        .base_date(sample_base_date())
        .require_monotonic()
        .knots([(0.0, 1.0), (1.0, 0.99), (2.0, 1.01)])
        .build();
    assert!(
        result.is_err(),
        "non-monotonic discounts should be rejected"
    );
}

#[test]
fn discount_curve_parallel_bump_and_df_batch() {
    let curve = sample_discount_curve("USD-OIS");
    let bumped = curve.with_parallel_bump(15.0);
    assert_eq!(bumped.id().as_str(), "USD-OIS_bump_15bp");

    let times = [0.5, 1.0, 2.0];
    let solo: Vec<f64> = times.iter().map(|&t| curve.df(t)).collect();
    assert_eq!(curve.df_batch(&times), solo);

    for &t in &times {
        assert!(bumped.df(t) < curve.df(t));
    }
}

#[test]
fn discount_curve_forward_and_df_on_date() {
    let curve = sample_discount_curve("USD-OIS");
    let t1 = 0.5;
    let t2 = 1.0;
    let fwd = curve.forward(t1, t2);
    let zero_1 = curve.zero(t1);
    let zero_2 = curve.zero(t2);
    assert!((fwd - (zero_1 * t1 - zero_2 * t2) / (t2 - t1)).abs() < 1e-12);

    let base = curve.base_date();
    let date = Date::from_calendar_date(base.year(), Month::December, 31).unwrap();
    let df_curve = curve.df_on_date_curve(date);
    let df_static = DiscountCurve::df_on(&curve, base, date, curve.day_count());
    assert!((df_curve - df_static).abs() < 1e-12);
}

#[test]
fn discount_curve_flat_forward_extrapolation_continues_slope() {
    let curve = DiscountCurve::builder("EXTRAP")
        .base_date(sample_base_date())
        .knots([(0.0, 1.0), (2.0, 0.95)])
        .extrapolation(ExtrapolationPolicy::FlatForward)
        .build()
        .unwrap();

    let df2 = curve.df(2.0);
    let df4 = curve.df(4.0);
    assert!(
        df4 < df2,
        "flat-forward extrapolation should decay beyond last knot"
    );
}

#[test]
fn discount_curve_builder_rejects_invalid_input() {
    let err = DiscountCurve::builder("INVALID")
        .base_date(sample_base_date())
        .knots([(0.0, 1.0)])
        .build()
        .expect_err("should fail with fewer than two points");
    assert!(matches!(err, finstack_core::Error::Input(_)));

    let err = DiscountCurve::builder("NONPOS")
        .base_date(sample_base_date())
        .knots([(0.0, 1.0), (1.0, 0.0)])
        .build()
        .expect_err("non-positive discount factor should be rejected");
    assert!(matches!(err, finstack_core::Error::Input(_)));
}

#[test]
fn discount_curve_key_rate_bump_targets_segment() {
    let curve = DiscountCurve::builder("KR")
        .base_date(sample_base_date())
        .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
        .build()
        .unwrap();

    let bumped = curve.with_key_rate_bump_years(1.2, 25.0);

    // First segment untouched, later segments scaled
    assert_eq!(bumped.df(0.0), curve.df(0.0));
    assert!(bumped.df(1.5) < curve.df(1.5));
}

#[test]
fn discount_curve_df_batch_handles_beyond_last_knot() {
    let curve = sample_discount_curve("USD-OIS");
    let times = [0.25, 1.0, 5.0, 10.0];
    let dfs = curve.df_batch(&times);
    assert_eq!(dfs.len(), times.len());
    assert!(dfs[3].is_finite());
}
