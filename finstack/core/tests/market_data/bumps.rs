use super::test_helpers::{
    sample_base_correlation_curve, sample_base_date, sample_discount_curve, sample_forward_curve,
    sample_hazard_curve, sample_inflation_curve,
};
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, Bumpable};
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::market_data::term_structures::HazardCurve;

#[test]
fn bump_spec_constructors_normalize_values() {
    let additive = BumpSpec::parallel_bp(25.0);
    assert_eq!(additive.mode, BumpMode::Additive);
    assert_eq!(additive.units, BumpUnits::RateBp);
    assert_eq!(additive.value, 25.0);

    let inflation = BumpSpec::inflation_shift_pct(2.5);
    assert_eq!(inflation.mode, BumpMode::Additive);
    assert_eq!(inflation.units, BumpUnits::Percent);
    assert_eq!(inflation.value, 2.5);

    let correlation = BumpSpec::correlation_shift_pct(15.0);
    assert_eq!(correlation.mode, BumpMode::Additive);
    assert_eq!(correlation.units, BumpUnits::Percent);
    assert_eq!(correlation.value, 15.0);

    let multiplier = BumpSpec::multiplier(1.1);
    assert_eq!(multiplier.mode, BumpMode::Multiplicative);
    assert_eq!(multiplier.units, BumpUnits::Factor);
    assert_eq!(multiplier.value, 1.1);
}

// Removed test for private ID helper functions

#[test]
fn discount_curve_parallel_bump_applies_rate_shift() {
    let curve = sample_discount_curve("USD-OIS");
    let bumped = curve
        .apply_bump(BumpSpec::parallel_bp(10.0))
        .expect("bump should succeed");
    assert_eq!(bumped.id().as_str(), "USD-OIS_bump_10bp");
    let original = curve.df(1.0);
    let bumped_df = bumped.df(1.0);
    assert!(
        bumped_df < original,
        "Additive parallel bump should decrease DF values"
    );
}

#[test]
fn forward_curve_supports_additive_and_multiplicative_bumps() {
    let curve = sample_forward_curve("USD-LIBOR");

    let additive = curve
        .apply_bump(BumpSpec {
            mode: BumpMode::Additive,
            units: BumpUnits::Percent,
            value: 1.5,
            bump_type: BumpType::Parallel,
        })
        .expect("percent bumps supported");
    assert_eq!(additive.id().as_str(), "USD-LIBOR_bump_2pct"); // percent formatted with rounding

    let multiplicative = curve
        .apply_bump(BumpSpec::multiplier(1.1))
        .expect("factor bumps supported");
    assert!(
        multiplicative.forwards()[1] > curve.forwards()[1],
        "multiplicative bumps scale rates upward"
    );
}

#[test]
fn forward_curve_parallel_bump_bp_is_additive_in_rate_space() {
    let curve = ForwardCurve::builder("USD-SOFR3M", 0.25)
        .base_date(sample_base_date())
        .knots([(0.0, 0.03), (1.0, 0.04), (2.0, 0.05)])
        .build()
        .unwrap();

    let bumped = curve.apply_bump(BumpSpec::parallel_bp(10.0)).unwrap();
    // 10bp = 0.001
    for (orig, b) in curve.forwards().iter().zip(bumped.forwards().iter()) {
        assert!((*b - (*orig + 0.001)).abs() < 1e-15);
    }
}

#[test]
fn forward_curve_triangular_key_rate_weights_match_market_standard() {
    let curve = ForwardCurve::builder("USD-SOFR3M", 0.25)
        .base_date(sample_base_date())
        // Include knots across the bucket grid so weights are visible at 0.5 and 1.0
        .knots([(0.0, 0.03), (0.5, 0.032), (1.0, 0.035), (2.0, 0.04)])
        .set_interp(finstack_core::math::interp::InterpStyle::Linear)
        .build()
        .unwrap();

    // Bucketed bump centered at 1.0 with neighbors at 0.0 and 2.0
    let bp = 20.0;
    let bumped = curve
        .apply_bump(BumpSpec::triangular_key_rate_bp(0.0, 1.0, 2.0, bp))
        .unwrap();

    let bump_rate = bp / 10_000.0;

    // Market-standard triangular weights (defined on bucket grid):
    // w(0.0)=0, w(0.5)=0.5, w(1.0)=1.0, w(2.0)=0
    let expected = [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0), (2.0, 0.0)];

    for (t, w) in expected {
        let base = curve.rate(t);
        let bumped_rate = bumped.rate(t);
        assert!(
            (bumped_rate - (base + bump_rate * w)).abs() < 1e-12,
            "t={}: expected {}, got {} (w={})",
            t,
            base + bump_rate * w,
            bumped_rate,
            w
        );
    }
}

#[test]
fn hazard_curve_requires_additive_fraction() {
    let curve = sample_hazard_curve("CDX");
    let additive = curve
        .apply_bump(BumpSpec::parallel_bp(50.0))
        .expect("hazard supports additive bp bumps");
    assert_eq!(additive.id().as_str(), "CDX_spread_50bp");

    let err = curve.apply_bump(BumpSpec::multiplier(1.2));
    assert!(err.is_err(), "hazard curves only support additive bumps");
}

#[test]
fn hazard_curve_rejects_invalid_recovery_for_bumps() {
    let curve = HazardCurve::builder("CDX-RISKLESS")
        .base_date(sample_base_date())
        .recovery_rate(1.0)
        .knots([(0.0, 0.01), (3.0, 0.015)])
        .build()
        .expect("hazard curve construction should succeed in test");

    let bumped = curve.apply_bump(BumpSpec::parallel_bp(25.0));
    assert!(
        bumped.is_err(),
        "curves with recovery >= 100% cannot convert par spread bumps"
    );
}

#[test]
fn inflation_curve_supports_percent_shifts() {
    let curve = sample_inflation_curve("USD-CPI");
    let bumped = curve
        .apply_bump(BumpSpec::inflation_shift_pct(2.0))
        .expect("inflation shift should succeed");
    assert_eq!(bumped.id().as_str(), "USD-CPI_bump_2pct");
    assert!(
        bumped.cpi(1.0) > curve.cpi(1.0),
        "additive percent shifts increase CPI levels"
    );
}

#[test]
fn base_correlation_curve_uses_percent_bumps() {
    let curve = sample_base_correlation_curve("CDX");
    let bumped = curve
        .apply_bump(BumpSpec::correlation_shift_pct(5.0))
        .expect("base correlation supports percent bump");
    assert_eq!(bumped.id().as_str(), "CDX_bump_5pct");
}
