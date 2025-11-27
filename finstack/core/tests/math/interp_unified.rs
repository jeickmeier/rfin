//! Unified interpolation tests using declarative macros
//!
//! This module consolidates the previously separate interpolator test files
//! (interp_linear.rs, interp_log_linear.rs, interp_cubic_hermite.rs,
//! interp_monotone_convex.rs, interp_flat_fwd.rs) into a single file using
//! macros to reduce duplication.

use super::common::approx_eq;
use finstack_core::math::interp::*;

// ============================================
// Shared Test Data
// ============================================

fn standard_knots() -> Box<[f64]> {
    vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice()
}

fn standard_dfs() -> Box<[f64]> {
    vec![1.0, 0.95, 0.9, 0.85].into_boxed_slice()
}

fn two_point_knots() -> Box<[f64]> {
    vec![0.0, 1.0].into_boxed_slice()
}

fn two_point_dfs() -> Box<[f64]> {
    vec![1.0, 0.95].into_boxed_slice()
}

// ============================================
// Generic Test Macro
// ============================================

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

                assert!((interp.interp(0.0) - 1.0).abs() < 1e-12);
                assert!((interp.interp(1.0) - 0.95).abs() < 1e-12);
                assert!((interp.interp(2.0) - 0.9).abs() < 1e-12);
                assert!((interp.interp(3.0) - 0.85).abs() < 1e-12);
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
        }
    };
}

// Apply macro to all interpolator types
interp_basic_tests!(linear_df, LinearDf::new);
interp_basic_tests!(log_linear_df, LogLinearDf::new);
interp_basic_tests!(flat_fwd, FlatFwd::new);
interp_basic_tests!(cubic_hermite, CubicHermite::new);
interp_basic_tests!(monotone_convex, MonotoneConvex::new);

// ============================================
// Type-Specific Tests (not macro-generated)
// ============================================

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
            // Expected via linear on log => geometric mean of dfs
            let expected = (dfs[seg].ln() * 0.5 + dfs[seg + 1].ln() * 0.5).exp();
            assert!((interp.interp(t_mid) - expected).abs() < 1e-12);
        }
    }
}

mod flat_fwd_specific {
    use super::*;

    #[test]
    fn matches_log_linear_exactly() {
        let knots = standard_knots();
        let dfs = standard_dfs();

        let flat =
            FlatFwd::new(knots.clone(), dfs.clone(), ExtrapolationPolicy::default()).unwrap();
        let log = LogLinearDf::new(knots, dfs, ExtrapolationPolicy::default()).unwrap();

        for t in [0.1, 0.25, 0.5, 1.5, 2.5] {
            assert!(
                approx_eq(flat.interp(t), log.interp(t), 1e-15),
                "FlatFwd != LogLinear at t={}",
                t
            );
        }
    }

    #[test]
    fn constant_forward_rate_property() {
        let knots = standard_knots();
        let dfs = standard_dfs();
        let interp = FlatFwd::new(knots, dfs, ExtrapolationPolicy::default()).unwrap();

        // For flat forward, the forward rate between any two points in a segment should be constant
        let t1 = 0.25;
        let t2 = 0.75;
        let df1 = interp.interp(t1);
        let df2 = interp.interp(t2);

        // Forward rate should be constant: f = -d(ln(DF))/dt
        let implied_rate = -(df2.ln() - df1.ln()) / (t2 - t1);

        // Check another pair in the same segment
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
        let interp =
            CubicHermite::new(standard_knots(), standard_dfs(), ExtrapolationPolicy::default())
                .unwrap();

        let h = 1e-8;
        let x = 1.5;
        let numerical = (interp.interp(x + h) - interp.interp(x - h)) / (2.0 * h);
        let analytical = interp.interp_prime(x);

        let relative_error = (analytical - numerical).abs() / numerical.abs();
        assert!(
            relative_error < 1e-6,
            "Derivative error {} too large",
            relative_error
        );
    }

    #[test]
    fn derivative_monotonicity_preserved() {
        let interp =
            CubicHermite::new(standard_knots(), standard_dfs(), ExtrapolationPolicy::default())
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
        let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
        let dfs = vec![1.0, 0.95, 0.9, 0.85].into_boxed_slice();

        let interp = CubicHermite::new(knots, dfs, ExtrapolationPolicy::default()).unwrap();

        // Test derivative at knot points returns precomputed slopes
        let derivative_at_knots = vec![
            interp.interp_prime(0.0),
            interp.interp_prime(1.0),
            interp.interp_prime(2.0),
            interp.interp_prime(3.0),
        ];

        // All derivatives should be finite and non-positive (monotone-preserving)
        for &deriv in &derivative_at_knots {
            assert!(deriv.is_finite());
            assert!(deriv <= 0.0); // Non-positive for decreasing sequence
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
        // Test that the interpolator preserves the general shape of the input curve
        let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
        let dfs = vec![1.0, 0.95, 0.9, 0.85].into_boxed_slice();

        let interp = MonotoneConvex::new(knots, dfs, ExtrapolationPolicy::default()).unwrap();

        // Sample several points and verify reasonable behavior
        let test_points = [0.25, 0.5, 0.75, 1.25, 1.5, 1.75, 2.25, 2.5, 2.75];
        let mut values = Vec::new();

        for &t in &test_points {
            let df = interp.interp(t);
            values.push(df);

            // All values should be positive and finite
            assert!(
                df > 0.0 && df.is_finite(),
                "Invalid value at t={}: {}",
                t,
                df
            );
        }

        // Values should generally decrease (monotone property)
        for i in 1..values.len() {
            // Allow for small numerical variations but ensure general decreasing trend
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
}

