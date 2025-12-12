//! Tests for ForwardCurve functionality.
//!
//! This module covers:
//! - Builder validation and construction
//! - Interpolation styles
//! - Rate calculations
//! - Clone safety (panic-free)
//! - Serialization roundtrips

use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use time::Month;

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

// =============================================================================
// Clone Safety Tests
// =============================================================================

/// Verifies that cloning a ForwardCurve is infallible and produces identical results.
#[test]
fn clone_is_panic_free_and_equivalent() {
    let original = ForwardCurve::builder("USD-SOFR3M", 0.25)
        .base_date(test_date())
        .reset_lag(2)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 0.03),
            (0.25, 0.032),
            (0.5, 0.035),
            (1.0, 0.04),
            (2.0, 0.042),
            (5.0, 0.045),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Clone should not panic
    let cloned = original.clone();

    // Verify structural equality
    assert_eq!(original.id(), cloned.id());
    assert_eq!(original.base_date(), cloned.base_date());
    assert_eq!(original.reset_lag(), cloned.reset_lag());
    assert_eq!(original.day_count(), cloned.day_count());
    assert_eq!(original.tenor(), cloned.tenor());
    assert_eq!(original.knots(), cloned.knots());
    assert_eq!(original.forwards(), cloned.forwards());

    // Verify interpolation produces identical results
    for t in [0.0, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0] {
        let orig_rate = original.rate(t);
        let cloned_rate = cloned.rate(t);
        assert!(
            (orig_rate - cloned_rate).abs() < 1e-14,
            "Rate mismatch after clone at t={}: {} vs {}",
            t,
            orig_rate,
            cloned_rate
        );
    }
}

/// Verifies clone works correctly for all interpolation styles.
#[test]
fn clone_works_for_all_interp_styles() {
    let interp_styles = [
        InterpStyle::Linear,
        InterpStyle::LogLinear,
        InterpStyle::CubicHermite,
        InterpStyle::PiecewiseQuadraticForward,
    ];

    for style in interp_styles {
        let curve = ForwardCurve::builder("TEST", 0.25)
            .base_date(test_date())
            .knots([(0.0, 0.03), (1.0, 0.04), (5.0, 0.05)])
            .set_interp(style)
            .build()
            .unwrap();

        // Clone should not panic for any interpolation style
        let cloned = curve.clone();

        // Verify rates match
        for t in [0.0, 0.5, 1.0, 3.0, 5.0] {
            let orig_rate = curve.rate(t);
            let cloned_rate = cloned.rate(t);
            assert!(
                (orig_rate - cloned_rate).abs() < 1e-14,
                "Clone rate mismatch for {:?} at t={}: {} vs {}",
                style,
                t,
                orig_rate,
                cloned_rate
            );
        }
    }
}

/// Verifies clone works correctly with different extrapolation policies.
#[test]
fn clone_works_for_all_extrapolation_policies() {
    let policies = [
        ExtrapolationPolicy::FlatZero,
        ExtrapolationPolicy::FlatForward,
    ];

    for policy in policies {
        let curve = ForwardCurve::builder("TEST", 0.25)
            .base_date(test_date())
            .knots([(0.0, 0.03), (1.0, 0.04), (5.0, 0.05)])
            .extrapolation(policy)
            .build()
            .unwrap();

        // Clone should not panic for any extrapolation policy
        let cloned = curve.clone();

        // Verify extrapolated rates match beyond the knot range
        for t in [0.0, 2.5, 5.0, 10.0, 20.0] {
            let orig_rate = curve.rate(t);
            let cloned_rate = cloned.rate(t);
            assert!(
                (orig_rate - cloned_rate).abs() < 1e-14,
                "Clone rate mismatch for {:?} at t={}: {} vs {}",
                policy,
                t,
                orig_rate,
                cloned_rate
            );
        }
    }
}

// =============================================================================
// Serialization Tests
// =============================================================================

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    #[test]
    fn roundtrip_with_all_fields() {
        let original = ForwardCurve::builder("USD-SOFR3M", 0.25)
            .base_date(test_date())
            .reset_lag(2)
            .day_count(DayCount::Act360)
            .knots([
                (0.0, 0.03),
                (0.25, 0.032),
                (0.5, 0.035),
                (1.0, 0.04),
                (2.0, 0.042),
                (5.0, 0.045),
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ForwardCurve = serde_json::from_str(&json).unwrap();

        assert_eq!(original.id(), deserialized.id());
        assert_eq!(original.base_date(), deserialized.base_date());
        assert_eq!(original.reset_lag(), deserialized.reset_lag());
        assert_eq!(original.day_count(), deserialized.day_count());
        assert_eq!(original.tenor(), deserialized.tenor());
        assert_eq!(original.knots(), deserialized.knots());
        assert_eq!(original.forwards(), deserialized.forwards());

        // Test rate interpolation
        for t in [0.0, 0.1, 0.25, 0.4, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0] {
            let original_rate = original.rate(t);
            let deserialized_rate = deserialized.rate(t);
            assert!(
                (original_rate - deserialized_rate).abs() < 1e-12,
                "Forward rate mismatch at t={}: {} vs {}",
                t,
                original_rate,
                deserialized_rate
            );
        }
    }

    #[test]
    fn roundtrip_multiple_interp_styles() {
        // Note: MonotoneConvex requires non-increasing values, so it's not suitable for forward rates
        let interp_styles = [
            InterpStyle::Linear,
            InterpStyle::LogLinear,
            InterpStyle::CubicHermite,
            InterpStyle::LogLinear,
        ];

        for style in interp_styles {
            let original = ForwardCurve::builder("EUR-EURIBOR6M", 0.5)
                .base_date(Date::from_calendar_date(2025, Month::June, 30).unwrap())
                .reset_lag(2)
                .day_count(DayCount::Act360)
                .knots([(0.0, 0.025), (1.0, 0.03), (2.0, 0.035), (5.0, 0.04)])
                .set_interp(style)
                .build()
                .unwrap();

            let json = serde_json::to_string(&original).unwrap();
            let deserialized: ForwardCurve = serde_json::from_str(&json).unwrap();

            // Test interpolation accuracy for each style
            for t in [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 5.0] {
                let original_rate = original.rate(t);
                let deserialized_rate = deserialized.rate(t);
                assert!(
                    (original_rate - deserialized_rate).abs() < 1e-12,
                    "Forward rate mismatch for {:?} at t={}: {} vs {}",
                    style,
                    t,
                    original_rate,
                    deserialized_rate
                );
            }
        }
    }
}
