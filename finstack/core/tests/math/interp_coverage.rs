//! Additional interpolation tests targeting coverage gaps.
//!
//! This module provides tests for:
//! - `interp_prime` (derivative) methods for all interpolators
//! - Extrapolation policies (FlatZero, FlatForward)
//! - `InterpStyle::build` method
//! - Edge cases in coefficient building

use finstack_core::math::interp::*;

// ============================================
// Test Data Helpers
// ============================================

fn standard_knots() -> Box<[f64]> {
    vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice()
}

fn standard_dfs() -> Box<[f64]> {
    vec![1.0, 0.95, 0.9, 0.85].into_boxed_slice()
}

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

// ============================================
// InterpStyle::build Tests
// ============================================

#[test]
fn interp_style_build_linear() {
    let knots = standard_knots();
    let dfs = standard_dfs();

    let interp = InterpStyle::Linear
        .build(knots, dfs, ExtrapolationPolicy::FlatZero)
        .expect("Should build linear interpolator");

    // Verify it works correctly
    assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
    assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
    assert!(approx_eq(interp.interp(2.0), 0.9, 1e-12));
    assert!(approx_eq(interp.interp(3.0), 0.85, 1e-12));
}

#[test]
fn interp_style_build_log_linear() {
    let knots = standard_knots();
    let dfs = standard_dfs();

    let interp = InterpStyle::LogLinear
        .build(knots, dfs, ExtrapolationPolicy::FlatZero)
        .expect("Should build log-linear interpolator");

    assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
    assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
}

#[test]
fn interp_style_build_monotone_convex() {
    let knots = standard_knots();
    let dfs = standard_dfs();

    let interp = InterpStyle::MonotoneConvex
        .build(knots, dfs, ExtrapolationPolicy::FlatZero)
        .expect("Should build monotone convex interpolator");

    assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
    assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
}

#[test]
fn interp_style_build_cubic_hermite() {
    let knots = standard_knots();
    let dfs = standard_dfs();

    let interp = InterpStyle::CubicHermite
        .build(knots, dfs, ExtrapolationPolicy::FlatZero)
        .expect("Should build cubic hermite interpolator");

    assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
    assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
}

#[test]
fn interp_style_build_flat_fwd() {
    let knots = standard_knots();
    let dfs = standard_dfs();

    let interp = InterpStyle::FlatFwd
        .build(knots, dfs, ExtrapolationPolicy::FlatZero)
        .expect("Should build flat forward interpolator");

    assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
    assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
}

#[test]
fn interp_style_build_all_with_flat_forward_extrapolation() {
    let knots = standard_knots();
    let dfs = standard_dfs();

    // Test all styles with FlatForward extrapolation
    for style in [
        InterpStyle::Linear,
        InterpStyle::LogLinear,
        InterpStyle::MonotoneConvex,
        InterpStyle::CubicHermite,
        InterpStyle::FlatFwd,
    ] {
        let interp = style
            .build(
                knots.clone(),
                dfs.clone(),
                ExtrapolationPolicy::FlatForward,
            )
            .unwrap_or_else(|e| panic!("Should build {:?}: {:?}", style, e));

        // Verify exact knot values
        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(3.0), 0.85, 1e-12));
    }
}

// ============================================
// Extrapolation Policy Tests
// ============================================

macro_rules! extrapolation_tests {
    ($mod_name:ident, $constructor:path) => {
        mod $mod_name {
            use super::*;

            #[test]
            fn flat_zero_extrapolation_left() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                )
                .unwrap();

                // Left extrapolation should return first value
                let left_extrap = interp.interp(-1.0);
                assert!(
                    approx_eq(left_extrap, 1.0, 1e-10),
                    "FlatZero left extrapolation should return first value, got {}",
                    left_extrap
                );
            }

            #[test]
            fn flat_zero_extrapolation_right() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                )
                .unwrap();

                // Right extrapolation should return last value
                let right_extrap = interp.interp(5.0);
                assert!(
                    approx_eq(right_extrap, 0.85, 1e-10),
                    "FlatZero right extrapolation should return last value, got {}",
                    right_extrap
                );
            }

            #[test]
            fn flat_forward_extrapolation_left() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatForward,
                )
                .unwrap();

                // Left extrapolation should use slope from first segment
                let left_extrap = interp.interp(-0.5);
                // Value should be above 1.0 since curve is decreasing
                assert!(
                    left_extrap > 1.0 && left_extrap.is_finite(),
                    "FlatForward left extrapolation {} should be > 1.0",
                    left_extrap
                );
            }

            #[test]
            fn flat_forward_extrapolation_right() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatForward,
                )
                .unwrap();

                // Right extrapolation should use slope from last segment
                let right_extrap = interp.interp(4.0);
                // Value should be below 0.85 since curve is decreasing
                assert!(
                    right_extrap < 0.85 && right_extrap > 0.0 && right_extrap.is_finite(),
                    "FlatForward right extrapolation {} should be < 0.85 and > 0",
                    right_extrap
                );
            }

            #[test]
            fn extrapolation_far_left() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                )
                .unwrap();

                // Far left should still work
                let far_left = interp.interp(-100.0);
                assert!(far_left.is_finite());
                assert!(approx_eq(far_left, 1.0, 1e-10));
            }

            #[test]
            fn extrapolation_far_right() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                )
                .unwrap();

                // Far right should still work
                let far_right = interp.interp(100.0);
                assert!(far_right.is_finite());
                assert!(approx_eq(far_right, 0.85, 1e-10));
            }
        }
    };
}

extrapolation_tests!(extrap_linear, LinearDf::new);
extrapolation_tests!(extrap_log_linear, LogLinearDf::new);
extrapolation_tests!(extrap_flat_fwd, FlatFwd::new);
extrapolation_tests!(extrap_cubic_hermite, CubicHermite::new);
extrapolation_tests!(extrap_monotone_convex, MonotoneConvex::new);

// ============================================
// Derivative (interp_prime) Tests
// ============================================

macro_rules! derivative_tests {
    ($mod_name:ident, $constructor:path) => {
        mod $mod_name {
            use super::*;

            #[test]
            fn derivative_is_finite_interior() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                )
                .unwrap();

                for &x in &[0.5, 1.0, 1.5, 2.0, 2.5] {
                    let deriv = interp.interp_prime(x);
                    assert!(
                        deriv.is_finite(),
                        "Derivative at {} should be finite, got {}",
                        x,
                        deriv
                    );
                }
            }

            #[test]
            fn derivative_is_negative_for_decreasing_curve() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                )
                .unwrap();

                // For a decreasing curve, derivative should be negative
                for &x in &[0.5, 1.5, 2.5] {
                    let deriv = interp.interp_prime(x);
                    assert!(
                        deriv < 0.0,
                        "Derivative at {} should be negative for decreasing curve, got {}",
                        x,
                        deriv
                    );
                }
            }

            #[test]
            fn derivative_numerical_check() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                )
                .unwrap();

                // Use central difference to approximate derivative
                let h = 1e-6;
                for &x in &[0.5, 1.5, 2.5] {
                    let numerical = (interp.interp(x + h) - interp.interp(x - h)) / (2.0 * h);
                    let analytical = interp.interp_prime(x);

                    let error = (analytical - numerical).abs();
                    assert!(
                        error < 1e-5,
                        "Derivative mismatch at {}: analytical={}, numerical={}, error={}",
                        x,
                        analytical,
                        numerical,
                        error
                    );
                }
            }

            #[test]
            fn derivative_at_knots() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                )
                .unwrap();

                // Test derivative at exact knot values
                for &x in &[0.0, 1.0, 2.0, 3.0] {
                    let deriv = interp.interp_prime(x);
                    assert!(deriv.is_finite(), "Derivative at knot {} should be finite", x);
                }
            }

            #[test]
            fn derivative_flat_zero_extrapolation() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatZero,
                )
                .unwrap();

                // With FlatZero extrapolation, derivative should be 0 outside domain
                let left_deriv = interp.interp_prime(-1.0);
                let right_deriv = interp.interp_prime(5.0);

                assert!(
                    approx_eq(left_deriv, 0.0, 1e-10),
                    "FlatZero left derivative should be 0, got {}",
                    left_deriv
                );
                assert!(
                    approx_eq(right_deriv, 0.0, 1e-10),
                    "FlatZero right derivative should be 0, got {}",
                    right_deriv
                );
            }

            #[test]
            fn derivative_flat_forward_extrapolation() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatForward,
                )
                .unwrap();

                // With FlatForward extrapolation, derivative should continue the boundary slope
                let left_deriv = interp.interp_prime(-1.0);
                let right_deriv = interp.interp_prime(5.0);

                // Both should be non-zero and finite
                assert!(
                    left_deriv.is_finite() && left_deriv != 0.0,
                    "FlatForward left derivative should be non-zero, got {}",
                    left_deriv
                );
                assert!(
                    right_deriv.is_finite() && right_deriv != 0.0,
                    "FlatForward right derivative should be non-zero, got {}",
                    right_deriv
                );
            }
        }
    };
}

derivative_tests!(deriv_linear, LinearDf::new);
derivative_tests!(deriv_log_linear, LogLinearDf::new);
derivative_tests!(deriv_flat_fwd, FlatFwd::new);
derivative_tests!(deriv_cubic_hermite, CubicHermite::new);
derivative_tests!(deriv_monotone_convex, MonotoneConvex::new);

// ============================================
// MonotoneConvex Specific Coverage Tests
// ============================================

mod monotone_convex_coverage {
    use super::*;

    #[test]
    fn derivative_at_all_knot_points() {
        let interp =
            MonotoneConvex::new(standard_knots(), standard_dfs(), ExtrapolationPolicy::FlatZero)
                .unwrap();

        // Test derivatives at all knot points including boundaries
        for &x in &[0.0, 1.0, 2.0, 3.0] {
            let deriv = interp.interp_prime(x);
            assert!(deriv.is_finite());
            assert!(deriv <= 0.0); // Should be non-positive for decreasing curve
        }
    }

    #[test]
    fn derivative_extrapolation_flat_forward_left() {
        let interp = MonotoneConvex::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
        )
        .unwrap();

        // Test derivative in left extrapolation region
        for &x in &[-0.5, -1.0, -2.0] {
            let deriv = interp.interp_prime(x);
            assert!(
                deriv.is_finite(),
                "Left extrapolation derivative at {} should be finite",
                x
            );
            assert!(
                deriv < 0.0,
                "Left extrapolation derivative should be negative for decreasing curve"
            );
        }
    }

    #[test]
    fn derivative_extrapolation_flat_forward_right() {
        let interp = MonotoneConvex::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
        )
        .unwrap();

        // Test derivative in right extrapolation region
        for &x in &[4.0, 5.0, 10.0] {
            let deriv = interp.interp_prime(x);
            assert!(
                deriv.is_finite(),
                "Right extrapolation derivative at {} should be finite",
                x
            );
            assert!(
                deriv < 0.0,
                "Right extrapolation derivative should be negative for decreasing curve"
            );
        }
    }

    #[test]
    fn interp_at_exact_knots_returns_exact_values() {
        let interp =
            MonotoneConvex::new(standard_knots(), standard_dfs(), ExtrapolationPolicy::FlatZero)
                .unwrap();

        // Exact knot values should be returned exactly
        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-15));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-15));
        assert!(approx_eq(interp.interp(2.0), 0.9, 1e-15));
        assert!(approx_eq(interp.interp(3.0), 0.85, 1e-15));
    }

    #[test]
    fn extrapolation_flat_forward_values_left() {
        let interp = MonotoneConvex::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
        )
        .unwrap();

        // Values should extend with forward slope
        let left_val = interp.interp(-0.5);
        assert!(left_val > 1.0, "Left extrapolated value should be > 1.0");
        assert!(left_val.is_finite());
    }

    #[test]
    fn extrapolation_flat_forward_values_right() {
        let interp = MonotoneConvex::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
        )
        .unwrap();

        // Values should extend with forward slope
        let right_val = interp.interp(4.0);
        assert!(
            right_val < 0.85,
            "Right extrapolated value should be < 0.85"
        );
        assert!(right_val > 0.0, "Right extrapolated value should be > 0");
        assert!(right_val.is_finite());
    }

    #[test]
    fn convexity_constraint_with_steep_curve() {
        // Test with a steeper curve that might trigger convexity constraint
        let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
        let steep_dfs = vec![1.0, 0.8, 0.6, 0.4].into_boxed_slice();

        let interp =
            MonotoneConvex::new(knots, steep_dfs, ExtrapolationPolicy::FlatZero).unwrap();

        // All interpolated values should be positive and finite
        for x in (0..30).map(|i| i as f64 * 0.1) {
            let val = interp.interp(x);
            assert!(val > 0.0 && val.is_finite(), "Value at {} = {}", x, val);
        }
    }

    #[test]
    fn near_zero_slope_handling() {
        // Test with very small differences between values (near-zero slopes)
        let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
        let near_flat = vec![1.0, 0.9999, 0.9998, 0.9997].into_boxed_slice();

        let interp =
            MonotoneConvex::new(knots, near_flat, ExtrapolationPolicy::FlatZero).unwrap();

        // Should handle near-flat curves without numerical issues
        for x in (0..30).map(|i| i as f64 * 0.1) {
            let val = interp.interp(x);
            assert!(
                val > 0.0 && val.is_finite(),
                "Near-flat curve value at {} = {}",
                x,
                val
            );
        }
    }

    #[test]
    fn sign_change_in_slopes() {
        // Test case where adjacent slopes have different signs (kink in the curve)
        let knots = vec![0.0, 1.0, 2.0, 3.0, 4.0].into_boxed_slice();
        // Non-monotone input should be rejected
        let non_monotone = vec![1.0, 0.9, 0.85, 0.86, 0.84].into_boxed_slice();

        let result =
            MonotoneConvex::new(knots, non_monotone, ExtrapolationPolicy::FlatZero);
        assert!(
            result.is_err(),
            "MonotoneConvex should reject non-monotone input"
        );
    }

    #[test]
    fn two_point_curve() {
        let knots = vec![0.0, 1.0].into_boxed_slice();
        let dfs = vec![1.0, 0.95].into_boxed_slice();

        let interp =
            MonotoneConvex::new(knots, dfs, ExtrapolationPolicy::FlatZero).unwrap();

        // Should work with just 2 points
        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));

        let mid = interp.interp(0.5);
        assert!(mid > 0.95 && mid < 1.0, "Midpoint should be between values");
    }

    #[test]
    fn five_point_curve() {
        let knots = vec![0.0, 0.5, 1.0, 2.0, 3.0].into_boxed_slice();
        let dfs = vec![1.0, 0.98, 0.95, 0.9, 0.85].into_boxed_slice();

        let interp =
            MonotoneConvex::new(knots, dfs, ExtrapolationPolicy::FlatZero).unwrap();

        // Verify all knot values
        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(0.5), 0.98, 1e-12));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
        assert!(approx_eq(interp.interp(2.0), 0.9, 1e-12));
        assert!(approx_eq(interp.interp(3.0), 0.85, 1e-12));
    }
}

// ============================================
// Linear Specific Coverage Tests
// ============================================

mod linear_coverage {
    use super::*;

    #[test]
    fn derivative_at_segment_boundaries() {
        let interp =
            LinearDf::new(standard_knots(), standard_dfs(), ExtrapolationPolicy::FlatZero).unwrap();

        // At segment boundaries, linear derivative should match the segment slope
        let h = 1e-8;
        for &x in &[1.0, 2.0] {
            let deriv = interp.interp_prime(x);
            let numerical = (interp.interp(x + h) - interp.interp(x - h)) / (2.0 * h);
            assert!(
                approx_eq(deriv, numerical, 1e-4),
                "Derivative at {} should match numerical approximation",
                x
            );
        }
    }
}

// ============================================
// LogLinear Specific Coverage Tests
// ============================================

mod log_linear_coverage {
    use super::*;

    #[test]
    fn derivative_formula_consistency() {
        let interp = LogLinearDf::new(standard_knots(), standard_dfs(), ExtrapolationPolicy::FlatZero)
            .unwrap();

        // For log-linear: f(x) = exp(log_interp(x))
        // df/dx = f(x) * d(log_interp)/dx
        // The analytical derivative should match this formula
        let h = 1e-7;
        for &x in &[0.5, 1.5, 2.5] {
            let analytical = interp.interp_prime(x);
            let numerical = (interp.interp(x + h) - interp.interp(x - h)) / (2.0 * h);
            
            let rel_error = (analytical - numerical).abs() / numerical.abs();
            assert!(
                rel_error < 1e-5,
                "Log-linear derivative at {}: analytical={}, numerical={}, rel_error={}",
                x, analytical, numerical, rel_error
            );
        }
    }
}

// ============================================
// CubicHermite Specific Coverage Tests
// ============================================

mod cubic_hermite_coverage {
    use super::*;

    #[test]
    fn extrapolation_uses_boundary_slopes() {
        let interp = CubicHermite::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
        )
        .unwrap();

        // Left extrapolation should use first slope
        let left_deriv = interp.interp_prime(-1.0);
        let first_deriv = interp.interp_prime(0.0);
        assert!(
            approx_eq(left_deriv, first_deriv, 1e-10),
            "Left extrapolation slope should match boundary slope"
        );

        // Right extrapolation should use last slope
        let right_deriv = interp.interp_prime(5.0);
        let last_deriv = interp.interp_prime(3.0);
        assert!(
            approx_eq(right_deriv, last_deriv, 1e-10),
            "Right extrapolation slope should match boundary slope"
        );
    }

    #[test]
    fn derivative_continuity_across_knots() {
        let interp =
            CubicHermite::new(standard_knots(), standard_dfs(), ExtrapolationPolicy::FlatZero)
                .unwrap();

        // Test derivative continuity at interior knots
        let h = 1e-8;
        for &x in &[1.0, 2.0] {
            let left_deriv = interp.interp_prime(x - h);
            let right_deriv = interp.interp_prime(x + h);
            let knot_deriv = interp.interp_prime(x);

            // All three should be approximately equal (C1 continuity)
            assert!(
                approx_eq(left_deriv, knot_deriv, 1e-4),
                "Left derivative at {} differs from knot derivative",
                x
            );
            assert!(
                approx_eq(right_deriv, knot_deriv, 1e-4),
                "Right derivative at {} differs from knot derivative",
                x
            );
        }
    }
}

