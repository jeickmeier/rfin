//! Tests for HazardCurve functionality.
//!
//! This module covers:
//! - Builder validation and construction
//! - Survival and default probability calculations
//! - Hazard rate shifts
//! - Metadata preservation
//! - Analytical verification

use finstack_core::currency::Currency;
use finstack_core::market_data::term_structures::hazard_curve::{
    HazardCurve, ParInterp, Seniority,
};
use time::{Date, Month};

fn base_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

// =============================================================================
// Builder Validation Tests
// =============================================================================

#[test]
fn builder_rejects_empty_knots() {
    let err = HazardCurve::builder("BAD")
        .build()
        .expect_err("no points should fail");
    assert!(matches!(err, finstack_core::Error::Input(_)));
}

#[test]
fn builder_rejects_negative_hazard_rate() {
    let err = HazardCurve::builder("NEG")
        .knots([(0.0, -0.01), (1.0, 0.02)])
        .build()
        .expect_err("negative lambda should fail");
    assert!(matches!(err, finstack_core::Error::Input(_)));
}

// =============================================================================
// Survival and Default Probability Tests
// =============================================================================

#[test]
fn survival_and_default_probabilities() {
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
fn hazard_shift_clamps_negative_rates() {
    let curve = HazardCurve::builder("HC")
        .base_date(base_date())
        .knots([(0.0, 0.01), (5.0, 0.02)])
        .build()
        .unwrap();
    let shifted = curve.with_hazard_shift(-0.02).unwrap();

    for (_, lambda) in shifted.knot_points() {
        assert!(lambda >= 0.0);
    }
    assert!(shifted.sp(5.0) > curve.sp(5.0)); // lower hazard -> higher survival
}

// =============================================================================
// Metadata Preservation Tests
// =============================================================================

#[test]
fn to_builder_preserves_metadata() {
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

// =============================================================================
// Analytical Verification Tests
// =============================================================================

#[test]
fn sp_analytical_verification_constant_hazard() {
    // Constant hazard rate for simple verification
    let curve = HazardCurve::builder("SP-VERIFY")
        .base_date(base_date())
        .knots([(0.0, 0.02), (5.0, 0.02), (10.0, 0.02)])
        .build()
        .unwrap();

    // S(t) = exp(-λ*t) for constant λ
    for t in [1.0, 2.5, 5.0, 7.5, 10.0] {
        let expected = (-0.02_f64 * t).exp();
        let actual = curve.sp(t);
        assert!(
            (actual - expected).abs() < 1e-12,
            "SP at t={}: got {}, expected {}",
            t,
            actual,
            expected
        );
    }
}

#[test]
fn sp_piecewise_verification() {
    // Piecewise hazard curve: knots define λ at each point
    // Implementation uses λ[i] for interval (knot[i-1], knot[i]]
    // So for knots [(0.0, 0.01), (2.0, 0.015), (5.0, 0.02)]:
    // - From t=0 to t=2: uses λ=0.015 (rate at upper bound)
    // - From t=2 to t=5: uses λ=0.02 (rate at upper bound)
    let curve = HazardCurve::builder("SP-PIECEWISE")
        .base_date(base_date())
        .knots([(0.0, 0.01), (2.0, 0.015), (5.0, 0.02)])
        .build()
        .unwrap();

    // S(1) = exp(-0.015*1) - using λ=0.015 from knot[1] for interval (0,2]
    let expected_1 = (-0.015_f64 * 1.0).exp();
    assert!(
        (curve.sp(1.0) - expected_1).abs() < 1e-12,
        "SP at t=1: got {}, expected {}",
        curve.sp(1.0),
        expected_1
    );

    // S(3) = exp(-(0.015*2 + 0.02*1))
    // First 2 years at λ=0.015, next 1 year at λ=0.02
    let expected_3 = (-(0.015_f64 * 2.0 + 0.02 * 1.0)).exp();
    assert!(
        (curve.sp(3.0) - expected_3).abs() < 1e-12,
        "SP at t=3: got {}, expected {}",
        curve.sp(3.0),
        expected_3
    );

    // Default prob P(1,3) = S(1) - S(3)
    let expected_dp = expected_1 - expected_3;
    assert!(
        (curve.default_prob(1.0, 3.0) - expected_dp).abs() < 1e-12,
        "Default prob 1-3: got {}, expected {}",
        curve.default_prob(1.0, 3.0),
        expected_dp
    );
}
