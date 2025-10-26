//! Tests for DiscountCurve extrapolation policies and monotonic validation.

use finstack_core::{
    dates::Date, market_data::term_structures::DiscountCurve, math::interp::ExtrapolationPolicy,
};
use time::Month;

/// Create a test curve with standard parameters.
fn create_test_curve(
    extrapolation: ExtrapolationPolicy,
    require_monotonic: bool,
) -> Result<DiscountCurve, Box<dyn std::error::Error>> {
    let mut builder = DiscountCurve::builder("TEST-CURVE")
        .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .knots([(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (5.0, 0.78)])
        .set_interp(finstack_core::math::interp::InterpStyle::MonotoneConvex)
        .extrapolation(extrapolation);

    if require_monotonic {
        builder = builder.require_monotonic();
    }

    Ok(builder.build()?)
}

#[test]
fn test_flat_zero_extrapolation() {
    let curve = create_test_curve(ExtrapolationPolicy::FlatZero, false).unwrap();

    // Test left extrapolation (negative time)
    assert_eq!(curve.df(-0.5), 1.0);
    assert_eq!(curve.df(-1.0), 1.0);

    // Test right extrapolation (beyond 5 years)
    assert_eq!(curve.df(10.0), 0.78);
    assert_eq!(curve.df(100.0), 0.78);

    // Test exact knot values remain unchanged
    assert_eq!(curve.df(0.0), 1.0);
    assert_eq!(curve.df(5.0), 0.78);
}

#[test]
fn test_flat_forward_extrapolation() {
    let curve = create_test_curve(ExtrapolationPolicy::FlatForward, false).unwrap();

    // Test left extrapolation - should extend the forward rate from first segment
    let left_extrap = curve.df(-0.5);
    assert!(left_extrap > 1.0); // Should be higher than 1.0 for negative time with positive rates

    // Test right extrapolation - should extend the forward rate from last segment
    let right_extrap_near = curve.df(7.0);
    let right_extrap_far = curve.df(10.0);
    assert!(right_extrap_near < 0.78); // Should decay further
    assert!(right_extrap_far < right_extrap_near); // Should continue decaying

    // Test exact knot values remain unchanged
    assert_eq!(curve.df(0.0), 1.0);
    assert_eq!(curve.df(5.0), 0.78);
}

#[test]
fn test_monotonic_validation_success() {
    // Valid monotonic discount factors (strictly decreasing)
    let result = DiscountCurve::builder("CREDIT-CURVE")
        .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .knots([(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (5.0, 0.78)])
        .set_interp(finstack_core::math::interp::InterpStyle::MonotoneConvex)
        .require_monotonic()
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_monotonic_validation_failure() {
    // Invalid non-monotonic discount factors (increases at t=2)
    // NOTE: Monotonicity is now enforced by default, so this will fail even without require_monotonic()
    let result = DiscountCurve::builder("INVALID-CURVE")
        .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .knots([(0.0, 1.0), (1.0, 0.95), (2.0, 0.96), (5.0, 0.78)]) // 0.96 > 0.95
        .set_interp(finstack_core::math::interp::InterpStyle::MonotoneConvex)
        .build(); // No need for require_monotonic() - it's default now

    assert!(result.is_err());
    let error = result.unwrap_err();
    // Error type changed to Validation in the new implementation
    assert!(matches!(
        error,
        finstack_core::Error::Validation(_)
    ));
}

#[test]
fn test_non_monotonic_without_validation() {
    // Non-monotonic should succeed when validation is explicitly disabled
    // NOTE: Monotonicity is now enforced by default, so must use allow_non_monotonic() to override
    let result = DiscountCurve::builder("NON-MONOTONIC")
        .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
        .knots([(0.0, 1.0), (1.0, 0.95), (2.0, 0.96), (5.0, 0.78)])
        .set_interp(finstack_core::math::interp::InterpStyle::Linear) // Use linear instead of monotone_convex for non-monotonic data
        .allow_non_monotonic() // Must explicitly allow non-monotonic DFs (dangerous!)
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_interpolation_styles_with_extrapolation() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let knots = [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90)];

    // Test each interpolation style with both extrapolation policies
    for extrapolation in [
        ExtrapolationPolicy::FlatZero,
        ExtrapolationPolicy::FlatForward,
    ] {
        // Linear
        let linear_curve = DiscountCurve::builder("LINEAR-TEST")
            .base_date(base_date)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::Linear)
            .extrapolation(extrapolation)
            .build()
            .unwrap();

        // Log-linear
        let log_curve = DiscountCurve::builder("LOG-TEST")
            .base_date(base_date)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .extrapolation(extrapolation)
            .build()
            .unwrap();

        // Monotone convex
        let mc_curve = DiscountCurve::builder("MC-TEST")
            .base_date(base_date)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::MonotoneConvex)
            .extrapolation(extrapolation)
            .build()
            .unwrap();

        // Cubic Hermite
        let ch_curve = DiscountCurve::builder("CH-TEST")
            .base_date(base_date)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::CubicHermite)
            .extrapolation(extrapolation)
            .build()
            .unwrap();

        // All curves should be valid
        assert!(linear_curve.df(1.5) > 0.0);
        assert!(log_curve.df(1.5) > 0.0);
        assert!(mc_curve.df(1.5) > 0.0);
        assert!(ch_curve.df(1.5) > 0.0);

        // Test extrapolation behavior
        let extrap_test_time = 5.0;
        match extrapolation {
            ExtrapolationPolicy::FlatZero => {
                // Should equal endpoint values
                assert_eq!(linear_curve.df(extrap_test_time), 0.90);
                assert_eq!(log_curve.df(extrap_test_time), 0.90);
                assert_eq!(mc_curve.df(extrap_test_time), 0.90);
                assert_eq!(ch_curve.df(extrap_test_time), 0.90);
            }
            ExtrapolationPolicy::FlatForward => {
                // Should continue declining based on forward rates
                assert!(linear_curve.df(extrap_test_time) < 0.90);
                assert!(log_curve.df(extrap_test_time) < 0.90);
                assert!(mc_curve.df(extrap_test_time) < 0.90);
                assert!(ch_curve.df(extrap_test_time) < 0.90);
            }
            _ => {
                // Handle any other extrapolation policies
                // Default test - ensure we get some reasonable value
                assert!(linear_curve.df(extrap_test_time) > 0.0);
                assert!(log_curve.df(extrap_test_time) > 0.0);
                assert!(mc_curve.df(extrap_test_time) > 0.0);
                assert!(ch_curve.df(extrap_test_time) > 0.0);
            }
        }
    }
}

#[test]
fn test_credit_curve_construction() {
    // Test typical credit curve construction with monotonic validation
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // CDS spreads to survival probabilities (approximately)
    let times: [f64; 6] = [0.0, 0.5, 1.0, 3.0, 5.0, 10.0];
    let cds_spreads: [f64; 6] = [0.0, 50e-4, 75e-4, 120e-4, 150e-4, 200e-4]; // bps

    // Convert spreads to approximate survival probabilities
    // SP(t) ≈ exp(-spread * t) for small spreads
    let survival_probs: Vec<f64> = times
        .iter()
        .zip(cds_spreads.iter())
        .map(|(t, s)| {
            let product: f64 = (*s) * (*t);
            (-product).exp()
        })
        .collect();

    let knots: Vec<(f64, f64)> = times
        .iter()
        .zip(survival_probs.iter())
        .map(|(t, sp)| (*t, *sp))
        .collect();

    let credit_curve = DiscountCurve::builder("CREDIT-5Y")
        .base_date(base_date)
        .knots(knots)
        .set_interp(finstack_core::math::interp::InterpStyle::MonotoneConvex)
        .extrapolation(ExtrapolationPolicy::FlatForward) // Appropriate for credit curves
        .require_monotonic() // Critical for credit
        .build()
        .unwrap();

    // Test that survival probabilities are indeed decreasing
    let sp_1y = credit_curve.df(1.0);
    let sp_5y = credit_curve.df(5.0);
    let sp_10y = credit_curve.df(10.0);
    let sp_15y = credit_curve.df(15.0); // Extrapolated

    assert!(sp_1y > sp_5y);
    assert!(sp_5y > sp_10y);
    assert!(sp_10y > sp_15y);

    // Survival probabilities should be positive and <= 1
    assert!(sp_15y > 0.0 && sp_15y <= 1.0);
}

#[test]
fn test_edge_cases() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Test with minimum two points
    let minimal_curve = DiscountCurve::builder("MINIMAL")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .set_interp(finstack_core::math::interp::InterpStyle::Linear)
        .build()
        .unwrap();

    assert_eq!(minimal_curve.df(0.0), 1.0);
    assert_eq!(minimal_curve.df(1.0), 0.95);

    // Test zero time (should always return first DF)
    assert_eq!(minimal_curve.df(0.0), 1.0);

    // Test extrapolation with minimal curve
    assert_eq!(minimal_curve.df(-0.5), 1.0); // Flat-zero left
    assert_eq!(minimal_curve.df(2.0), 0.95); // Flat-zero right
}

#[test]
fn test_interpolation_consistency() {
    // Verify that all interpolation methods produce the same result at knot points
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let knots = [(0.0, 1.0), (1.0, 0.98), (2.0, 0.95), (5.0, 0.88)];

    let curves = [
        DiscountCurve::builder("LINEAR")
            .base_date(base_date)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::Linear)
            .build()
            .unwrap(),
        DiscountCurve::builder("LOG")
            .base_date(base_date)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .build()
            .unwrap(),
        DiscountCurve::builder("MC")
            .base_date(base_date)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::MonotoneConvex)
            .build()
            .unwrap(),
        DiscountCurve::builder("CH")
            .base_date(base_date)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::CubicHermite)
            .build()
            .unwrap(),
        DiscountCurve::builder("FF")
            .base_date(base_date)
            .knots(knots)
            .set_interp(finstack_core::math::interp::InterpStyle::FlatFwd)
            .build()
            .unwrap(),
    ];

    // All methods should agree exactly at knot points
    for (t, expected_df) in knots {
        for curve in &curves {
            assert!((curve.df(t) - expected_df).abs() < 1e-12);
        }
    }
}
