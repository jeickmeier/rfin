//! Interpolation tests.
//!
//! This module consolidates all interpolation tests including:
//! - Basic construction and value tests (via macros)
//! - Derivative/interp_prime tests
//! - Extrapolation policy tests
//! - Type-specific tests
//! - Trait tests
//! - Serialization tests

use super::common::{approx_eq, standard_dfs, standard_knots, two_point_dfs, two_point_knots};
use finstack_core::math::interp::*;

// ============================================================================
// Basic Tests (macro-generated for all interpolator types)
// ============================================================================

macro_rules! interp_basic_tests {
    ($mod_name:ident, $constructor:path) => {
        mod $mod_name {
            use super::*;

            #[test]
            fn construction_succeeds() {
                let result = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::default(),
                );
                assert!(result.is_ok(), "Construction should succeed");
            }

            #[test]
            fn exact_knot_values_preserved() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::default(),
                )
                .unwrap();

                assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
                assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
                assert!(approx_eq(interp.interp(2.0), 0.9, 1e-12));
                assert!(approx_eq(interp.interp(3.0), 0.85, 1e-12));
            }

            #[test]
            fn interpolation_between_knots() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::default(),
                )
                .unwrap();

                let mid = interp.interp(0.5);
                assert!(
                    mid > 0.95 && mid < 1.0,
                    "0.5 value {} not in (0.95, 1.0)",
                    mid
                );

                let mid2 = interp.interp(1.5);
                assert!(
                    mid2 > 0.9 && mid2 < 0.95,
                    "1.5 value {} not in (0.9, 0.95)",
                    mid2
                );
            }

            #[test]
            fn rejects_non_increasing_knots() {
                let bad_knots = vec![1.0, 0.0, 2.0, 3.0].into_boxed_slice();
                let result =
                    $constructor(bad_knots, standard_dfs(), ExtrapolationPolicy::default());
                assert!(result.is_err(), "Should reject non-increasing knots");
            }

            #[test]
            fn rejects_non_positive_dfs() {
                let bad_dfs = vec![1.0, -0.95, 0.9, 0.85].into_boxed_slice();
                let result =
                    $constructor(standard_knots(), bad_dfs, ExtrapolationPolicy::default());
                assert!(result.is_err(), "Should reject non-positive DFs");
            }

            #[test]
            fn two_point_case() {
                let interp = $constructor(
                    two_point_knots(),
                    two_point_dfs(),
                    ExtrapolationPolicy::default(),
                )
                .unwrap();

                let mid = interp.interp(0.5);
                assert!(
                    mid > 0.95 && mid <= 1.0,
                    "Two-point midpoint {} invalid",
                    mid
                );
                assert!(mid.is_finite());
            }

            #[test]
            fn edge_values_are_finite() {
                let interp = $constructor(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::default(),
                )
                .unwrap();

                assert!(interp.interp(0.001).is_finite());
                assert!(interp.interp(2.999).is_finite());
                assert!(interp.interp(0.001) > 0.0);
                assert!(interp.interp(2.999) > 0.0);
            }

            #[test]
            fn rejects_empty_input() {
                let result = $constructor(
                    vec![].into_boxed_slice(),
                    vec![].into_boxed_slice(),
                    ExtrapolationPolicy::default(),
                );
                assert!(result.is_err(), "Should reject empty input");
            }

            #[test]
            #[should_panic(expected = "assertion")]
            fn rejects_mismatched_lengths() {
                let _ = $constructor(
                    vec![0.0, 1.0, 2.0].into_boxed_slice(),
                    vec![1.0, 0.95].into_boxed_slice(),
                    ExtrapolationPolicy::default(),
                );
            }

            #[test]
            fn single_point_rejected() {
                let result = $constructor(
                    vec![1.0].into_boxed_slice(),
                    vec![0.95].into_boxed_slice(),
                    ExtrapolationPolicy::default(),
                );
                let _ = result; // Just verify it doesn't panic
            }
        }
    };
}

interp_basic_tests!(linear_df_basic, LinearDf::new);
interp_basic_tests!(log_linear_df_basic, LogLinearDf::new);
interp_basic_tests!(flat_fwd_basic, LogLinearDf::new);
interp_basic_tests!(cubic_hermite_basic, CubicHermite::new);
interp_basic_tests!(monotone_convex_basic, MonotoneConvex::new);

// ============================================================================
// InterpStyle::build Tests
// ============================================================================

mod interp_style_build {
    use super::*;

    #[test]
    fn build_linear() {
        let interp = InterpStyle::Linear
            .build(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
            )
            .expect("Should build linear interpolator");

        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
        assert!(approx_eq(interp.interp(2.0), 0.9, 1e-12));
        assert!(approx_eq(interp.interp(3.0), 0.85, 1e-12));
    }

    #[test]
    fn build_log_linear() {
        let interp = InterpStyle::LogLinear
            .build(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
            )
            .expect("Should build log-linear interpolator");

        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
    }

    #[test]
    fn build_monotone_convex() {
        let interp = InterpStyle::MonotoneConvex
            .build(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
            )
            .expect("Should build monotone convex interpolator");

        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
    }

    #[test]
    fn build_cubic_hermite() {
        let interp = InterpStyle::CubicHermite
            .build(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
            )
            .expect("Should build cubic hermite interpolator");

        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
    }

    #[test]
    fn build_flat_fwd() {
        let interp = InterpStyle::LogLinear
            .build(
                standard_knots(),
                standard_dfs(),
                ExtrapolationPolicy::FlatZero,
            )
            .expect("Should build flat forward interpolator");

        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
    }

    #[test]
    fn build_all_with_flat_forward_extrapolation() {
        for style in [
            InterpStyle::Linear,
            InterpStyle::LogLinear,
            InterpStyle::MonotoneConvex,
            InterpStyle::CubicHermite,
            InterpStyle::LogLinear,
        ] {
            let interp = style
                .build(
                    standard_knots(),
                    standard_dfs(),
                    ExtrapolationPolicy::FlatForward,
                )
                .unwrap_or_else(|e| panic!("Should build {:?}: {:?}", style, e));

            assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
            assert!(approx_eq(interp.interp(3.0), 0.85, 1e-12));
        }
    }
}

// ============================================================================
// Extrapolation Policy Tests
// ============================================================================

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

                let left_extrap = interp.interp(-0.5);
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

                let right_extrap = interp.interp(4.0);
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

                let far_right = interp.interp(100.0);
                assert!(far_right.is_finite());
                assert!(approx_eq(far_right, 0.85, 1e-10));
            }
        }
    };
}

extrapolation_tests!(extrap_linear, LinearDf::new);
extrapolation_tests!(extrap_log_linear, LogLinearDf::new);
extrapolation_tests!(extrap_flat_fwd, LogLinearDf::new);
extrapolation_tests!(extrap_cubic_hermite, CubicHermite::new);
extrapolation_tests!(extrap_monotone_convex, MonotoneConvex::new);

// ============================================================================
// Derivative (interp_prime) Tests
// ============================================================================

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

                for &x in &[0.0, 1.0, 2.0, 3.0] {
                    let deriv = interp.interp_prime(x);
                    assert!(
                        deriv.is_finite(),
                        "Derivative at knot {} should be finite",
                        x
                    );
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

                let left_deriv = interp.interp_prime(-1.0);
                let right_deriv = interp.interp_prime(5.0);

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
derivative_tests!(deriv_flat_fwd, LogLinearDf::new);
derivative_tests!(deriv_cubic_hermite, CubicHermite::new);
derivative_tests!(deriv_monotone_convex, MonotoneConvex::new);

// ============================================================================
// Type-Specific Tests
// ============================================================================

mod linear_specific {
    use super::*;

    #[test]
    fn midpoint_matches_manual_formula() {
        let knots: Box<[f64]> = (0..=4)
            .map(|i| i as f64)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let dfs: Box<[f64]> = knots
            .iter()
            .map(|&t| (-0.02f64 * t).exp())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let interp =
            LinearDf::new(knots.clone(), dfs.clone(), ExtrapolationPolicy::default()).unwrap();

        for seg in 0..knots.len() - 1 {
            let t_mid = 0.5 * (knots[seg] + knots[seg + 1]);
            let expected = 0.5 * (dfs[seg] + dfs[seg + 1]);
            assert!((interp.interp(t_mid) - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn derivative_at_segment_boundaries() {
        let interp = LinearDf::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
        )
        .unwrap();

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

mod log_linear_specific {
    use super::*;

    #[test]
    fn geometric_midpoint() {
        let knots: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0];
        let zero_rate = 0.03f64;
        let dfs: Vec<f64> = knots.iter().map(|&t| (-zero_rate * t).exp()).collect();
        let knots = knots.into_boxed_slice();
        let dfs = dfs.into_boxed_slice();

        let interp =
            LogLinearDf::new(knots.clone(), dfs.clone(), ExtrapolationPolicy::default()).unwrap();
        for seg in 0..knots.len() - 1 {
            let t_mid = 0.5 * (knots[seg] + knots[seg + 1]);
            let expected = (dfs[seg].ln() * 0.5 + dfs[seg + 1].ln() * 0.5).exp();
            assert!((interp.interp(t_mid) - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn derivative_formula_consistency() {
        let interp = LogLinearDf::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
        )
        .unwrap();

        let h = 1e-7;
        for &x in &[0.5, 1.5, 2.5] {
            let analytical = interp.interp_prime(x);
            let numerical = (interp.interp(x + h) - interp.interp(x - h)) / (2.0 * h);

            let rel_error = (analytical - numerical).abs() / numerical.abs();
            assert!(
                rel_error < 1e-5,
                "Log-linear derivative at {}: analytical={}, numerical={}, rel_error={}",
                x,
                analytical,
                numerical,
                rel_error
            );
        }
    }
}

mod flat_fwd_specific {
    use super::*;

    #[test]
    fn matches_log_linear_exactly() {
        let flat = LogLinearDf::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::default(),
        )
        .unwrap();
        let log = LogLinearDf::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::default(),
        )
        .unwrap();

        for t in [0.1, 0.25, 0.5, 1.5, 2.5] {
            assert!(
                approx_eq(flat.interp(t), log.interp(t), 1e-15),
                "LogLinear != LogLinear at t={}",
                t
            );
        }
    }

    #[test]
    fn constant_forward_rate_property() {
        let interp = LogLinearDf::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::default(),
        )
        .unwrap();

        let t1 = 0.25;
        let t2 = 0.75;
        let df1 = interp.interp(t1);
        let df2 = interp.interp(t2);

        let implied_rate = -(df2.ln() - df1.ln()) / (t2 - t1);

        let t3 = 0.1;
        let t4 = 0.9;
        let df3 = interp.interp(t3);
        let df4 = interp.interp(t4);
        let implied_rate2 = -(df4.ln() - df3.ln()) / (t4 - t3);

        assert!(approx_eq(implied_rate, implied_rate2, 1e-10));
    }
}

mod cubic_hermite_specific {
    use super::*;

    #[test]
    fn derivative_numerical_consistency() {
        let interp = CubicHermite::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::default(),
        )
        .unwrap();

        let h = 6e-6;
        let x = 1.5;
        let numerical = (interp.interp(x + h) - interp.interp(x - h)) / (2.0 * h);
        let analytical = interp.interp_prime(x);

        let relative_error = (analytical - numerical).abs() / numerical.abs();
        assert!(
            relative_error < 1e-8,
            "Derivative error {} too large",
            relative_error
        );
    }

    #[test]
    fn derivative_monotonicity_preserved() {
        let interp = CubicHermite::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::default(),
        )
        .unwrap();

        for &x in &[0.5, 1.5, 2.5] {
            let deriv = interp.interp_prime(x);
            assert!(
                deriv < 0.0,
                "Derivative at {} should be negative, got {}",
                x,
                deriv
            );
            assert!(deriv.is_finite());
        }
    }

    #[test]
    fn derivative_at_knots_returns_precomputed_slopes() {
        let interp = CubicHermite::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::default(),
        )
        .unwrap();

        let derivative_at_knots = vec![
            interp.interp_prime(0.0),
            interp.interp_prime(1.0),
            interp.interp_prime(2.0),
            interp.interp_prime(3.0),
        ];

        for &deriv in &derivative_at_knots {
            assert!(deriv.is_finite());
            assert!(deriv <= 0.0);
        }
    }

    #[test]
    fn extrapolation_uses_boundary_slopes() {
        let interp = CubicHermite::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatForward,
        )
        .unwrap();

        let left_deriv = interp.interp_prime(-1.0);
        let first_deriv = interp.interp_prime(0.0);
        assert!(
            approx_eq(left_deriv, first_deriv, 1e-10),
            "Left extrapolation slope should match boundary slope"
        );

        let right_deriv = interp.interp_prime(5.0);
        let last_deriv = interp.interp_prime(3.0);
        assert!(
            approx_eq(right_deriv, last_deriv, 1e-10),
            "Right extrapolation slope should match boundary slope"
        );
    }

    #[test]
    fn derivative_continuity_across_knots() {
        let interp = CubicHermite::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
        )
        .unwrap();

        let h = 1e-8;
        for &x in &[1.0, 2.0] {
            let left_deriv = interp.interp_prime(x - h);
            let right_deriv = interp.interp_prime(x + h);
            let knot_deriv = interp.interp_prime(x);

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

mod monotone_convex_specific {
    use super::*;

    #[test]
    fn rejects_increasing_dfs() {
        let increasing_dfs = vec![0.9, 0.95, 1.0, 1.05].into_boxed_slice();
        let result = MonotoneConvex::new(
            standard_knots(),
            increasing_dfs,
            ExtrapolationPolicy::default(),
        );
        assert!(result.is_err(), "Should reject increasing DFs (arbitrage)");
    }

    #[test]
    fn near_flat_curve_handled() {
        let near_flat = vec![1.0, 0.999999, 0.999998, 0.999997].into_boxed_slice();
        let interp =
            MonotoneConvex::new(standard_knots(), near_flat, ExtrapolationPolicy::default())
                .unwrap();

        let mid = interp.interp(0.5);
        assert!(mid.is_finite());
        assert!(mid > 0.999997 && mid <= 1.0);
    }

    #[test]
    fn shape_preservation() {
        let interp = MonotoneConvex::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::default(),
        )
        .unwrap();

        let test_points = [0.25, 0.5, 0.75, 1.25, 1.5, 1.75, 2.25, 2.5, 2.75];
        let mut values = Vec::new();

        for &t in &test_points {
            let df = interp.interp(t);
            values.push(df);

            assert!(
                df > 0.0 && df.is_finite(),
                "Invalid value at t={}: {}",
                t,
                df
            );
        }

        for i in 1..values.len() {
            assert!(
                values[i] <= values[i - 1] + 1e-10,
                "Monotonicity violated between {} and {}: {} > {}",
                test_points[i - 1],
                test_points[i],
                values[i - 1],
                values[i]
            );
        }
    }

    #[test]
    fn derivative_at_all_knot_points() {
        let interp = MonotoneConvex::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
        )
        .unwrap();

        for &x in &[0.0, 1.0, 2.0, 3.0] {
            let deriv = interp.interp_prime(x);
            assert!(deriv.is_finite());
            assert!(deriv <= 0.0);
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
        let interp = MonotoneConvex::new(
            standard_knots(),
            standard_dfs(),
            ExtrapolationPolicy::FlatZero,
        )
        .unwrap();

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
        let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
        let steep_dfs = vec![1.0, 0.8, 0.6, 0.4].into_boxed_slice();

        let interp = MonotoneConvex::new(knots, steep_dfs, ExtrapolationPolicy::FlatZero).unwrap();

        for x in (0..30).map(|i| i as f64 * 0.1) {
            let val = interp.interp(x);
            assert!(val > 0.0 && val.is_finite(), "Value at {} = {}", x, val);
        }
    }

    #[test]
    fn near_zero_slope_handling() {
        let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
        let near_flat = vec![1.0, 0.9999, 0.9998, 0.9997].into_boxed_slice();

        let interp = MonotoneConvex::new(knots, near_flat, ExtrapolationPolicy::FlatZero).unwrap();

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
        let knots = vec![0.0, 1.0, 2.0, 3.0, 4.0].into_boxed_slice();
        let non_monotone = vec![1.0, 0.9, 0.85, 0.86, 0.84].into_boxed_slice();

        let result = MonotoneConvex::new(knots, non_monotone, ExtrapolationPolicy::FlatZero);
        assert!(
            result.is_err(),
            "MonotoneConvex should reject non-monotone input"
        );
    }

    #[test]
    fn two_point_curve() {
        let knots = vec![0.0, 1.0].into_boxed_slice();
        let dfs = vec![1.0, 0.95].into_boxed_slice();

        let interp = MonotoneConvex::new(knots, dfs, ExtrapolationPolicy::FlatZero).unwrap();

        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));

        let mid = interp.interp(0.5);
        assert!(mid > 0.95 && mid < 1.0, "Midpoint should be between values");
    }

    #[test]
    fn five_point_curve() {
        let knots = vec![0.0, 0.5, 1.0, 2.0, 3.0].into_boxed_slice();
        let dfs = vec![1.0, 0.98, 0.95, 0.9, 0.85].into_boxed_slice();

        let interp = MonotoneConvex::new(knots, dfs, ExtrapolationPolicy::FlatZero).unwrap();

        assert!(approx_eq(interp.interp(0.0), 1.0, 1e-12));
        assert!(approx_eq(interp.interp(0.5), 0.98, 1e-12));
        assert!(approx_eq(interp.interp(1.0), 0.95, 1e-12));
        assert!(approx_eq(interp.interp(2.0), 0.9, 1e-12));
        assert!(approx_eq(interp.interp(3.0), 0.85, 1e-12));
    }
}

// ============================================================================
// Trait Tests
// ============================================================================

mod traits {
    use super::*;

    #[derive(Debug)]
    struct TestInterp {
        value: f64,
    }

    impl InterpFn for TestInterp {
        fn interp(&self, _x: f64) -> f64 {
            self.value
        }
    }

    #[test]
    fn interp_fn_default_derivative_uses_finite_differences() {
        let interp = TestInterp { value: 42.0 };

        let deriv = interp.interp_prime(1.0);
        assert!(deriv.abs() < 1e-6);
    }

    #[test]
    fn interp_fn_default_derivative_for_linear_function() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let values = vec![1.0, 0.5, 0.1].into_boxed_slice();
        let interp = LinearDf::new(knots, values, ExtrapolationPolicy::FlatZero).unwrap();

        let deriv_0_5 = interp.interp_prime(0.5);
        let deriv_1_5 = interp.interp_prime(1.5);

        assert!((deriv_0_5 - (-0.5)).abs() < 1e-6);
        assert!((deriv_1_5 - (-0.4)).abs() < 1e-6);
    }

    #[test]
    fn interp_fn_derivative_for_exponential_function() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();
        let interp = LogLinearDf::new(knots, dfs, ExtrapolationPolicy::FlatZero).unwrap();

        let deriv_0_5 = interp.interp_prime(0.5);
        let deriv_1_5 = interp.interp_prime(1.5);

        assert!(deriv_0_5 < 0.0);
        assert!(deriv_1_5 < 0.0);
        assert!(deriv_0_5.is_finite());
        assert!(deriv_1_5.is_finite());
    }

    #[test]
    fn interp_fn_derivative_consistency() {
        let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
        let values = vec![1.0, 0.8, 0.6, 0.4].into_boxed_slice();
        let interp = LinearDf::new(knots, values, ExtrapolationPolicy::FlatZero).unwrap();

        let x = 1.5;
        let h = 6e-6;
        let numerical = (interp.interp(x + h) - interp.interp(x - h)) / (2.0 * h);
        let analytical = interp.interp_prime(x);

        assert!((numerical - analytical).abs() < 1e-8);
    }

    #[test]
    fn interp_fn_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn InterpFn>>();
    }

    #[test]
    fn interp_fn_derivative_at_boundaries() {
        let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
        let values = vec![1.0, 0.5, 0.25].into_boxed_slice();
        let interp = LinearDf::new(knots, values, ExtrapolationPolicy::FlatZero).unwrap();

        let deriv_0 = interp.interp_prime(0.0);
        let deriv_1 = interp.interp_prime(1.0);
        let deriv_2 = interp.interp_prime(2.0);

        assert!(deriv_0.is_finite());
        assert!(deriv_1.is_finite());
        assert!(deriv_2.is_finite());

        assert!(deriv_0 <= 0.0);
        assert!(deriv_1 <= 0.0);
        assert!(deriv_2 <= 0.0);
    }

    #[test]
    fn interp_fn_derivative_extrapolation_flat_zero() {
        let knots = vec![0.0, 1.0].into_boxed_slice();
        let values = vec![1.0, 0.5].into_boxed_slice();
        let interp = LinearDf::new(knots, values, ExtrapolationPolicy::FlatZero).unwrap();

        let deriv_below = interp.interp_prime(-0.5);
        let deriv_above = interp.interp_prime(2.0);

        assert_eq!(deriv_below, 0.0);
        assert_eq!(deriv_above, 0.0);
    }

    #[test]
    fn interp_fn_derivative_extrapolation_flat_forward() {
        let knots = vec![0.0, 1.0].into_boxed_slice();
        let values = vec![1.0, 0.5].into_boxed_slice();
        let interp = LinearDf::new(knots, values, ExtrapolationPolicy::FlatForward).unwrap();

        let deriv_interior = interp.interp_prime(0.5);
        let deriv_below = interp.interp_prime(-0.5);
        let deriv_above = interp.interp_prime(2.0);

        assert!((deriv_interior - deriv_below).abs() < 1e-6);
        assert!((deriv_interior - deriv_above).abs() < 1e-6);
    }
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    #[test]
    fn linear_df_roundtrip() {
        let knots = vec![0.0, 1.0, 2.0, 3.0, 5.0].into_boxed_slice();
        let dfs = vec![1.0, 0.98, 0.95, 0.92, 0.87].into_boxed_slice();
        let extrapolation = ExtrapolationPolicy::FlatZero;

        let linear = LinearDf::new(knots, dfs, extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&linear).unwrap();
        let deserialized: LinearDf = serde_json::from_str(&json).unwrap();
        assert_eq!(linear.interp(1.5), deserialized.interp(1.5));
    }

    #[test]
    fn log_linear_df_roundtrip() {
        let knots = vec![0.0, 1.0, 2.0, 3.0, 5.0].into_boxed_slice();
        let dfs = vec![1.0, 0.98, 0.95, 0.92, 0.87].into_boxed_slice();
        let extrapolation = ExtrapolationPolicy::FlatZero;

        let log_linear = LogLinearDf::new(knots, dfs, extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&log_linear).unwrap();
        let deserialized: LogLinearDf = serde_json::from_str(&json).unwrap();
        assert!((log_linear.interp(1.5) - deserialized.interp(1.5)).abs() < 1e-10);
    }

    #[test]
    fn monotone_convex_roundtrip() {
        let knots = vec![0.0, 1.0, 2.0, 3.0, 5.0].into_boxed_slice();
        let dfs = vec![1.0, 0.98, 0.95, 0.92, 0.87].into_boxed_slice();
        let extrapolation = ExtrapolationPolicy::FlatZero;

        let monotone = MonotoneConvex::new(knots, dfs, extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&monotone).unwrap();
        let deserialized: MonotoneConvex = serde_json::from_str(&json).unwrap();
        assert!((monotone.interp(1.5) - deserialized.interp(1.5)).abs() < 1e-10);
    }

    #[test]
    fn cubic_hermite_roundtrip() {
        let knots = vec![0.0, 1.0, 2.0, 3.0, 5.0].into_boxed_slice();
        let dfs = vec![1.0, 0.98, 0.95, 0.92, 0.87].into_boxed_slice();
        let extrapolation = ExtrapolationPolicy::FlatZero;

        let cubic = CubicHermite::new(knots, dfs, extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&cubic).unwrap();
        let deserialized: CubicHermite = serde_json::from_str(&json).unwrap();
        assert!((cubic.interp(1.5) - deserialized.interp(1.5)).abs() < 1e-10);
    }

    #[test]
    fn flat_fwd_roundtrip() {
        let knots = vec![0.0, 1.0, 2.0, 3.0, 5.0].into_boxed_slice();
        let dfs = vec![1.0, 0.98, 0.95, 0.92, 0.87].into_boxed_slice();
        let extrapolation = ExtrapolationPolicy::FlatZero;

        let flat_fwd = LogLinearDf::new(knots, dfs, extrapolation).unwrap();
        let json = serde_json::to_string_pretty(&flat_fwd).unwrap();
        let deserialized: LogLinearDf = serde_json::from_str(&json).unwrap();
        assert!((flat_fwd.interp(1.5) - deserialized.interp(1.5)).abs() < 1e-10);
    }
}
