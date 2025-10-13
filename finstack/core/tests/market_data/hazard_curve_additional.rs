use finstack_core::currency::Currency;
use finstack_core::market_data::term_structures::hazard_curve::{
    HazardCurve, ParInterp, Seniority,
};
use time::{Date, Month};

fn base_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

#[test]
fn hazard_curve_builder_validates_inputs() {
    let err = HazardCurve::builder("BAD").build().expect_err("no points should fail");
    assert!(matches!(err, finstack_core::Error::Input(_)));

    let err = HazardCurve::builder("NEG")
        .knots([(0.0, -0.01), (1.0, 0.02)])
        .build()
        .expect_err("negative lambda should fail");
    assert!(matches!(err, finstack_core::Error::Input(_)));
}

#[test]
fn hazard_curve_survival_and_default_probabilities() {
    let curve = HazardCurve::builder("HC")
        .base_date(base_date())
        .knots([(0.0, 0.01), (2.0, 0.015), (5.0, 0.02)])
        .par_spreads([(1.0, 100.0), (3.0, 150.0)])
        .build()
        .unwrap();

    assert!(curve.sp(0.0) - 1.0 < 1e-12);
    assert!(curve.sp(4.0) < curve.sp(1.0));
    assert!(curve.default_prob(1.0, 3.0) > 0.0);

    let linear = curve.quoted_spread_bp(2.0, ParInterp::Linear);
    let log = curve.quoted_spread_bp(2.0, ParInterp::LogLinear);
    assert!(linear > 0.0 && log > 0.0);
}

#[test]
fn hazard_curve_with_hazard_shift_clamps_negative_rates() {
    let curve = HazardCurve::builder("HC")
        .knots([(0.0, 0.01), (5.0, 0.02)])
        .build()
        .unwrap();
    let shifted = curve.with_hazard_shift(-0.02).unwrap();

    for (_, lambda) in shifted.knot_points() {
        assert!(lambda >= 0.0);
    }
    assert!(shifted.sp(5.0) > curve.sp(5.0)); // lower hazard -> higher survival
}

#[test]
fn hazard_curve_to_builder_preserves_metadata() {
    let curve = HazardCurve::builder("HC")
        .base_date(base_date())
        .recovery_rate(0.35)
        .issuer("ACME Corp")
        .seniority(Seniority::Senior)
        .currency(Currency::USD)
        .knots([(0.0, 0.01), (3.0, 0.02)])
        .par_spreads([(2.0, 120.0)])
        .build()
        .unwrap();

    let rebuilt = curve
        .to_builder_with_id("HC-NEW")
        .build()
        .expect("builder should succeed");

    assert_eq!(rebuilt.recovery_rate(), curve.recovery_rate());
    assert_eq!(rebuilt.seniority, Some(Seniority::Senior));
    assert_eq!(
        rebuilt.knot_points().collect::<Vec<_>>(),
        curve.knot_points().collect::<Vec<_>>()
    );
}
