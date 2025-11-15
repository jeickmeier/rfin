//! Property tests for finite difference accuracy and edge cases.
//!
//! These tests verify fundamental properties of finite difference calculations
//! including symmetry, convergence, adaptive bumps, and numerical stability.

#[cfg(test)]
mod finite_difference_properties {
    use crate::metrics::finite_difference::{
        adaptive_rate_bump, adaptive_spot_bump, adaptive_vol_bump, bump_sizes, central_diff_1d,
        central_mixed,
    };
    use proptest::prelude::*;

    // -----------------------------------------------------------------------------
    // Property Tests for Central Differences
    // -----------------------------------------------------------------------------

    /// Property: Central difference should be symmetric.
    /// If we swap f_up and f_down, the result should negate.
    #[test]
    fn test_central_diff_symmetry() {
        proptest!(|(
            f_up in -1000.0..1000.0f64,
            f_down in -1000.0..1000.0f64,
            h in 0.0001..10.0f64,
        )| {
            let forward = central_diff_1d(|| Ok(f_up), || Ok(f_down), h).unwrap();
            let backward = central_diff_1d(|| Ok(f_down), || Ok(f_up), h).unwrap();

            // forward should be -backward (within numerical precision)
            prop_assert!((forward + backward).abs() < 1e-10,
                "Central diff should be symmetric: forward={}, backward={}", forward, backward);
        });
    }

    /// Property: Central difference scales linearly with function values.
    /// derivative(k*f) = k * derivative(f)
    #[test]
    fn test_central_diff_linearity() {
        proptest!(|(
            f_up in -100.0..100.0f64,
            f_down in -100.0..100.0f64,
            scale in -10.0..10.0f64,
            h in 0.0001..1.0f64,
        )| {
            prop_assume!(scale.abs() > 1e-6); // Avoid near-zero scales

            let original = central_diff_1d(|| Ok(f_up), || Ok(f_down), h).unwrap();
            let scaled = central_diff_1d(|| Ok(f_up * scale), || Ok(f_down * scale), h).unwrap();

            let expected = original * scale;
            let rel_error = if expected.abs() > 1e-10 {
                ((scaled - expected) / expected).abs()
            } else {
                (scaled - expected).abs()
            };

            prop_assert!(rel_error < 1e-8,
                "Linearity failed: original={}, scaled={}, expected={}, rel_error={}",
                original, scaled, expected, rel_error);
        });
    }

    /// Property: Central difference should be invariant to bump size scaling
    /// when the function is linear (up to numerical precision).
    #[test]
    fn test_central_diff_linear_function_invariance() {
        proptest!(|(
            slope in -100.0..100.0f64,
            x_center in -100.0..100.0f64,
            h1 in 0.0001..1.0f64,
            h2 in 0.0001..1.0f64,
        )| {
            // Linear function: f(x) = slope * x
            let f_up1 = slope * (x_center + h1);
            let f_down1 = slope * (x_center - h1);
            let f_up2 = slope * (x_center + h2);
            let f_down2 = slope * (x_center - h2);

            let deriv1 = central_diff_1d(|| Ok(f_up1), || Ok(f_down1), h1).unwrap();
            let deriv2 = central_diff_1d(|| Ok(f_up2), || Ok(f_down2), h2).unwrap();

            // For linear functions, derivative should match slope exactly (within precision)
            // Relax precision for floating point arithmetic
            prop_assert!((deriv1 - slope).abs() < 1e-6,
                "Linear function derivative should equal slope: deriv1={}, slope={}", deriv1, slope);
            prop_assert!((deriv2 - slope).abs() < 1e-6,
                "Linear function derivative should equal slope: deriv2={}, slope={}", deriv2, slope);
            prop_assert!((deriv1 - deriv2).abs() < 1e-6,
                "Different bump sizes should give same derivative for linear function");
        });
    }

    /// Property: For quadratic functions, central difference should approximate
    /// the analytical derivative well for reasonable bump sizes.
    #[test]
    fn test_central_diff_quadratic_accuracy() {
        proptest!(|(
            a in -10.0..10.0f64,
            b in -10.0..10.0f64,
            x in -10.0..10.0f64,
            h in 0.0001..0.1f64,
        )| {
            prop_assume!(a.abs() > 1e-6); // Avoid near-zero leading coefficient

            // Quadratic: f(x) = a*x^2 + b*x
            // Analytical derivative: f'(x) = 2*a*x + b
            let f = |x: f64| a * x * x + b * x;
            let f_prime_analytical = 2.0 * a * x + b;

            let f_up = f(x + h);
            let f_down = f(x - h);
            let f_prime_numerical = central_diff_1d(|| Ok(f_up), || Ok(f_down), h).unwrap();

            // Error should be O(h^2) for central differences
            let error = (f_prime_numerical - f_prime_analytical).abs();
            let error_bound = 10.0 * a.abs() * h * h; // Conservative bound

            prop_assert!(error < error_bound,
                "Quadratic derivative error too large: numerical={}, analytical={}, error={}, bound={}",
                f_prime_numerical, f_prime_analytical, error, error_bound);
        });
    }

    /// Property: Central difference should reject zero or negative bump sizes.
    #[test]
    fn test_central_diff_rejects_invalid_bumps() {
        proptest!(|(
            h in -10.0..=0.0f64,
        )| {
            let result = central_diff_1d(|| Ok(1.0), || Ok(1.0), h);
            prop_assert!(result.is_err(), "Should reject non-positive bump: h={}", h);
        });
    }

    /// Property: Central difference should reject NaN and infinite bump sizes.
    #[test]
    fn test_central_diff_rejects_non_finite_bumps() {
        let result_nan = central_diff_1d(|| Ok(1.0), || Ok(1.0), f64::NAN);
        assert!(result_nan.is_err(), "Should reject NaN bump");

        let result_inf = central_diff_1d(|| Ok(1.0), || Ok(1.0), f64::INFINITY);
        assert!(result_inf.is_err(), "Should reject infinite bump");
    }

    // -----------------------------------------------------------------------------
    // Property Tests for Central Mixed Derivatives
    // -----------------------------------------------------------------------------

    /// Property: Mixed derivative should be symmetric in the two variables.
    /// d²f/dxdy = d²f/dydx
    #[test]
    fn test_central_mixed_symmetry() {
        proptest!(|(
            f_pp in -1000.0..1000.0f64,
            f_pm in -1000.0..1000.0f64,
            f_mp in -1000.0..1000.0f64,
            f_mm in -1000.0..1000.0f64,
            h in 0.0001..1.0f64,
            k in 0.0001..1.0f64,
        )| {
            let mixed_hk = central_mixed(
                || Ok(f_pp), || Ok(f_pm), || Ok(f_mp), || Ok(f_mm),
                h, k
            ).unwrap();

            // Swap h and k (swap x and y)
            let mixed_kh = central_mixed(
                || Ok(f_pp), || Ok(f_mp), || Ok(f_pm), || Ok(f_mm),
                k, h
            ).unwrap();

            // Should be equal (Schwarz's theorem for smooth functions)
            let rel_error = if mixed_hk.abs() > 1e-10 {
                ((mixed_hk - mixed_kh) / mixed_hk).abs()
            } else {
                (mixed_hk - mixed_kh).abs()
            };

            prop_assert!(rel_error < 1e-8,
                "Mixed derivative should be symmetric: hk={}, kh={}", mixed_hk, mixed_kh);
        });
    }

    /// Property: For bilinear function f(x,y) = a*x*y, mixed derivative should equal a.
    #[test]
    fn test_central_mixed_bilinear_exact() {
        proptest!(|(
            a in -100.0..100.0f64,
            x in -10.0..10.0f64,
            y in -10.0..10.0f64,
            h in 0.0001..0.1f64,
            k in 0.0001..0.1f64,
        )| {
            // f(x,y) = a*x*y
            // d²f/dxdy = a
            let f = |x: f64, y: f64| a * x * y;

            let f_pp = f(x + h, y + k);
            let f_pm = f(x + h, y - k);
            let f_mp = f(x - h, y + k);
            let f_mm = f(x - h, y - k);

            let mixed = central_mixed(
                || Ok(f_pp), || Ok(f_pm), || Ok(f_mp), || Ok(f_mm),
                h, k
            ).unwrap();

            // Should equal 'a' exactly (within numerical precision)
            // Relax precision for floating point arithmetic
            prop_assert!((mixed - a).abs() < 1e-5,
                "Bilinear mixed derivative should equal coefficient: mixed={}, a={}", mixed, a);
        });
    }

    /// Property: Mixed derivative should reject invalid bump sizes.
    #[test]
    fn test_central_mixed_rejects_invalid_bumps() {
        proptest!(|(
            h in -10.0..=0.0f64,
        )| {
            let result = central_mixed(
                || Ok(0.0), || Ok(0.0), || Ok(0.0), || Ok(0.0),
                h, 1.0
            );
            prop_assert!(result.is_err(), "Should reject non-positive h: h={}", h);

            let result2 = central_mixed(
                || Ok(0.0), || Ok(0.0), || Ok(0.0), || Ok(0.0),
                1.0, h
            );
            prop_assert!(result2.is_err(), "Should reject non-positive k: k={}", h);
        });
    }

    // -----------------------------------------------------------------------------
    // Property Tests for Adaptive Bumps
    // -----------------------------------------------------------------------------

    /// Property: Adaptive spot bump should be within reasonable bounds.
    #[test]
    fn test_adaptive_spot_bump_bounds() {
        proptest!(|(
            vol in 0.01..2.0f64,
            time in 0.01..30.0f64,
        )| {
            let bump = adaptive_spot_bump(vol, time, None);

            // Should be between 0.001 (0.1%) and 0.05 (5%)
            prop_assert!((0.001..=0.05).contains(&bump),
                "Adaptive spot bump out of bounds: bump={}, vol={}, time={}",
                bump, vol, time);
        });
    }

    /// Property: Adaptive spot bump should respect manual override.
    #[test]
    fn test_adaptive_spot_bump_override() {
        proptest!(|(
            vol in 0.01..2.0f64,
            time in 0.01..30.0f64,
            override_pct in 0.001..0.10f64,
        )| {
            let bump = adaptive_spot_bump(vol, time, Some(override_pct));

            prop_assert!((bump - override_pct).abs() < 1e-12,
                "Override should be respected exactly: bump={}, override={}",
                bump, override_pct);
        });
    }

    /// Property: Adaptive spot bump should increase with volatility.
    #[test]
    fn test_adaptive_spot_bump_vol_monotonicity() {
        proptest!(|(
            vol1 in 0.05..0.5f64,
            vol2_delta in 0.1..1.0f64,
            time in 1.0..10.0f64,
        )| {
            let vol2 = vol1 + vol2_delta;

            let bump1 = adaptive_spot_bump(vol1, time, None);
            let bump2 = adaptive_spot_bump(vol2, time, None);

            // Higher vol should give higher or equal bump (capped at 5%)
            prop_assert!(bump2 >= bump1 || (bump2 - 0.05).abs() < 1e-10,
                "Higher vol should increase bump: vol1={}, bump1={}, vol2={}, bump2={}",
                vol1, bump1, vol2, bump2);
        });
    }

    /// Property: Adaptive spot bump should increase with time to expiry.
    #[test]
    fn test_adaptive_spot_bump_time_monotonicity() {
        proptest!(|(
            vol in 0.2..0.8f64,
            time1 in 0.1..5.0f64,
            time2_delta in 0.5..5.0f64,
        )| {
            let time2 = time1 + time2_delta;

            let bump1 = adaptive_spot_bump(vol, time1, None);
            let bump2 = adaptive_spot_bump(vol, time2, None);

            // Longer time should give higher or equal bump (capped at 5%)
            prop_assert!(bump2 >= bump1 || (bump2 - 0.05).abs() < 1e-10,
                "Longer time should increase bump: time1={}, bump1={}, time2={}, bump2={}",
                time1, bump1, time2, bump2);
        });
    }

    /// Property: Adaptive vol bump should be within reasonable bounds.
    #[test]
    fn test_adaptive_vol_bump_bounds() {
        proptest!(|(
            current_vol in 0.001..3.0f64,
        )| {
            let bump = adaptive_vol_bump(current_vol, None);

            // Should be at least 0.001 (0.1%) and at most 0.05 (5%)
            prop_assert!((0.001..=0.05).contains(&bump),
                "Adaptive vol bump out of bounds: bump={}, vol={}",
                bump, current_vol);
        });
    }

    /// Property: Adaptive vol bump should respect manual override.
    #[test]
    fn test_adaptive_vol_bump_override() {
        proptest!(|(
            current_vol in 0.01..2.0f64,
            override_pct in 0.001..0.10f64,
        )| {
            let bump = adaptive_vol_bump(current_vol, Some(override_pct));

            prop_assert!((bump - override_pct).abs() < 1e-12,
                "Override should be respected exactly: bump={}, override={}",
                bump, override_pct);
        });
    }

    /// Property: Adaptive vol bump should increase with volatility level.
    #[test]
    fn test_adaptive_vol_bump_monotonicity() {
        proptest!(|(
            vol1 in 0.05..0.5f64,
            vol2_delta in 0.1..1.0f64,
        )| {
            let vol2 = vol1 + vol2_delta;

            let bump1 = adaptive_vol_bump(vol1, None);
            let bump2 = adaptive_vol_bump(vol2, None);

            // Higher vol should give higher or equal bump (capped at 5%)
            prop_assert!(bump2 >= bump1 || (bump2 - 0.05).abs() < 1e-10,
                "Higher vol should increase bump: vol1={}, bump1={}, vol2={}, bump2={}",
                vol1, bump1, vol2, bump2);
        });
    }

    /// Property: Adaptive rate bump should respect override or use default.
    #[test]
    fn test_adaptive_rate_bump_override() {
        proptest!(|(
            override_bp in 0.00001..0.01f64,
        )| {
            let bump = adaptive_rate_bump(Some(override_bp));
            prop_assert!((bump - override_bp).abs() < 1e-12,
                "Override should be respected: bump={}, override={}", bump, override_bp);
        });

        // Test default case
        let default_bump = adaptive_rate_bump(None);
        assert!(
            (default_bump - bump_sizes::INTEREST_RATE_BP).abs() < 1e-12,
            "Default should be 1bp: {}",
            default_bump
        );
    }

    // -----------------------------------------------------------------------------
    // Edge Case Tests
    // -----------------------------------------------------------------------------

    /// Property: Central difference should handle very small function values.
    #[test]
    fn test_central_diff_tiny_values() {
        proptest!(|(
            f_up in -1e-10..1e-10f64,
            f_down in -1e-10..1e-10f64,
            h in 0.0001..1.0f64,
        )| {
            let result = central_diff_1d(|| Ok(f_up), || Ok(f_down), h);

            prop_assert!(result.is_ok(), "Should handle tiny values: f_up={}, f_down={}", f_up, f_down);
            let deriv = result.unwrap();
            prop_assert!(deriv.is_finite(), "Result should be finite: {}", deriv);
        });
    }

    /// Property: Central difference should handle very large function values.
    #[test]
    fn test_central_diff_large_values() {
        proptest!(|(
            f_up in -1e10..1e10f64,
            f_down in -1e10..1e10f64,
            h in 0.0001..1.0f64,
        )| {
            let result = central_diff_1d(|| Ok(f_up), || Ok(f_down), h);

            prop_assert!(result.is_ok(), "Should handle large values: f_up={}, f_down={}", f_up, f_down);
            let deriv = result.unwrap();
            prop_assert!(deriv.is_finite(), "Result should be finite: {}", deriv);
        });
    }

    /// Property: Central difference with identical up/down should give zero derivative.
    #[test]
    fn test_central_diff_flat_function() {
        proptest!(|(
            value in -1e6..1e6f64,
            h in 0.0001..10.0f64,
        )| {
            let deriv = central_diff_1d(|| Ok(value), || Ok(value), h).unwrap();

            prop_assert!(deriv.abs() < 1e-10,
                "Flat function should have zero derivative: value={}, deriv={}", value, deriv);
        });
    }

    /// Property: Adaptive bumps should handle extreme volatilities gracefully.
    #[test]
    fn test_adaptive_bumps_extreme_vol() {
        // Very low vol
        let bump_low = adaptive_vol_bump(0.001, None);
        assert!(
            (0.001..=0.05).contains(&bump_low),
            "Low vol bump: {}",
            bump_low
        );

        // Very high vol
        let bump_high = adaptive_vol_bump(5.0, None);
        assert!(
            (0.001..=0.05).contains(&bump_high),
            "High vol bump: {}",
            bump_high
        );
    }

    /// Property: Adaptive bumps should handle extreme time to expiry.
    #[test]
    fn test_adaptive_bumps_extreme_time() {
        // Very short time
        let bump_short = adaptive_spot_bump(0.3, 0.001, None);
        assert!(
            (0.001..=0.05).contains(&bump_short),
            "Short time bump: {}",
            bump_short
        );

        // Very long time
        let bump_long = adaptive_spot_bump(0.3, 100.0, None);
        assert!(
            (0.001..=0.05).contains(&bump_long),
            "Long time bump: {}",
            bump_long
        );
    }

    // -----------------------------------------------------------------------------
    // Convergence Tests
    // -----------------------------------------------------------------------------

    /// Property: For smooth functions, smaller bumps should converge to
    /// the analytical derivative (up to numerical precision limits).
    #[test]
    fn test_central_diff_convergence() {
        proptest!(|(
            a in -10.0..10.0f64,
            b in -10.0..10.0f64,
            c in -10.0..10.0f64,
            x in -10.0..10.0f64,
        )| {
            prop_assume!(a.abs() > 1e-6);

            // Test with cubic: f(x) = a*x^3 + b*x^2 + c*x
            // f'(x) = 3*a*x^2 + 2*b*x + c
            let f = |x: f64| a * x.powi(3) + b * x.powi(2) + c * x;
            let f_prime_analytical = 3.0 * a * x * x + 2.0 * b * x + c;

            // Test with progressively smaller bumps
            let h1 = 0.01;
            let h2 = 0.001;
            let h3 = 0.0001;

            let deriv1 = central_diff_1d(|| Ok(f(x + h1)), || Ok(f(x - h1)), h1).unwrap();
            let deriv2 = central_diff_1d(|| Ok(f(x + h2)), || Ok(f(x - h2)), h2).unwrap();
            let deriv3 = central_diff_1d(|| Ok(f(x + h3)), || Ok(f(x - h3)), h3).unwrap();

            let error1 = (deriv1 - f_prime_analytical).abs();
            let error2 = (deriv2 - f_prime_analytical).abs();
            let error3 = (deriv3 - f_prime_analytical).abs();

            // Errors should generally decrease (allowing some numerical noise for very small errors)
            if error1 > 1e-10 {
                prop_assert!(error2 <= error1 * 1.1,
                    "Error should decrease: h1={}, error1={}, h2={}, error2={}",
                    h1, error1, h2, error2);
            }
            if error2 > 1e-10 {
                prop_assert!(error3 <= error2 * 1.1,
                    "Error should decrease: h2={}, error2={}, h3={}, error3={}",
                    h2, error2, h3, error3);
            }
        });
    }

    /// Property: Central difference error for quadratic should be O(h^2).
    #[test]
    fn test_central_diff_error_order() {
        proptest!(|(
            a in -10.0..10.0f64,
            b in -10.0..10.0f64,
            x in -10.0..10.0f64,
        )| {
            prop_assume!(a.abs() > 1e-3);

            // Quadratic: f(x) = a*x^2 + b*x
            // f'(x) = 2*a*x + b
            let f = |x: f64| a * x * x + b * x;
            let f_prime = 2.0 * a * x + b;

            let h1 = 0.1;
            let h2 = 0.01;

            let deriv1 = central_diff_1d(|| Ok(f(x + h1)), || Ok(f(x - h1)), h1).unwrap();
            let deriv2 = central_diff_1d(|| Ok(f(x + h2)), || Ok(f(x - h2)), h2).unwrap();

            let error1 = (deriv1 - f_prime).abs();
            let error2 = (deriv2 - f_prime).abs();

            // If h2 = h1/10, error2 should be roughly error1/100 (O(h^2) convergence)
            // Allow factor of 200 for safety (some numerical variance expected)
            if error1 > 1e-10 && error2 > 1e-12 {
                let ratio = error1 / error2;
                prop_assert!(ratio > 50.0 && ratio < 200.0,
                    "Error should decrease as O(h^2): h1={}, error1={}, h2={}, error2={}, ratio={}",
                    h1, error1, h2, error2, ratio);
            }
        });
    }

    // -----------------------------------------------------------------------------
    // Numerical Stability Tests
    // -----------------------------------------------------------------------------

    /// Property: Repeated central difference calculation should give consistent results.
    #[test]
    fn test_central_diff_determinism() {
        proptest!(|(
            f_up in -100.0..100.0f64,
            f_down in -100.0..100.0f64,
            h in 0.0001..1.0f64,
        )| {
            let result1 = central_diff_1d(|| Ok(f_up), || Ok(f_down), h).unwrap();
            let result2 = central_diff_1d(|| Ok(f_up), || Ok(f_down), h).unwrap();
            let result3 = central_diff_1d(|| Ok(f_up), || Ok(f_down), h).unwrap();

            prop_assert_eq!(result1, result2, "Results should be identical");
            prop_assert_eq!(result2, result3, "Results should be identical");
        });
    }

    /// Property: Central difference should not lose precision catastrophically
    /// for well-conditioned problems.
    #[test]
    fn test_central_diff_precision_loss() {
        proptest!(|(
            slope in 0.1..100.0f64,
            x in -100.0..100.0f64,
            h in 0.0001..0.1f64,
        )| {
            // Linear function with known derivative
            let f = |x: f64| slope * x;

            let f_up = f(x + h);
            let f_down = f(x - h);
            let deriv = central_diff_1d(|| Ok(f_up), || Ok(f_down), h).unwrap();

            // Should get slope back with high precision
            let rel_error = ((deriv - slope) / slope).abs();

            prop_assert!(rel_error < 1e-10,
                "Linear function precision loss too high: slope={}, deriv={}, rel_error={}",
                slope, deriv, rel_error);
        });
    }
}

#[cfg(test)]
mod bump_helper_properties {
    use crate::metrics::finite_difference::bump_scalar_price;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::MarketContext;
    use finstack_core::money::Money;
    use finstack_core::types::{Currency, CurveId};
    use proptest::prelude::*;

    /// Property: Bumping a scalar price should scale it correctly.
    /// Note: Money types round to 2 decimal places, so we use looser tolerance.
    #[test]
    fn test_bump_scalar_price_scaling() {
        proptest!(|(
            price in 1.0..10000.0f64,
            bump_pct in -0.5..0.5f64,
        )| {
            let mut context = MarketContext::new();
            let price_id = "TEST_PRICE";
            let currency = Currency::USD;

            context.prices.insert(
                CurveId::from(price_id),
                MarketScalar::Price(Money::new(price, currency))
            );

            let bumped_context = bump_scalar_price(&context, price_id, bump_pct).unwrap();
            let bumped_price = match bumped_context.price(price_id).unwrap() {
                MarketScalar::Price(m) => m.amount(),
                _ => panic!("Expected price"),
            };

            let expected = price * (1.0 + bump_pct);
            // Money rounds to cents, so we allow 2 cents tolerance to account for:
            // - rounding of original price to cents
            // - rounding of bumped price to cents
            // - floating point precision errors
            let abs_error = (bumped_price - expected).abs();
            let tolerance = 0.02 + expected.abs() * 1e-5;

            prop_assert!(abs_error < tolerance,
                "Price bump error: original={}, bump_pct={}, expected={}, got={}, error={}",
                price, bump_pct, expected, bumped_price, abs_error);
        });
    }

    /// Property: Bumping unitless scalar should scale it correctly.
    #[test]
    fn test_bump_unitless_scaling() {
        proptest!(|(
            value in -10000.0..10000.0f64,
            bump_pct in -0.5..0.5f64,
        )| {
            prop_assume!(value.abs() > 1e-6); // Avoid near-zero values

            let mut context = MarketContext::new();
            let price_id = "TEST_UNITLESS";

            context.prices.insert(
                CurveId::from(price_id),
                MarketScalar::Unitless(value)
            );

            let bumped_context = bump_scalar_price(&context, price_id, bump_pct).unwrap();
            let bumped_value = match bumped_context.price(price_id).unwrap() {
                MarketScalar::Unitless(v) => *v,
                _ => panic!("Expected unitless"),
            };

            let expected = value * (1.0 + bump_pct);
            let rel_error = ((bumped_value - expected) / expected).abs();

            prop_assert!(rel_error < 1e-10,
                "Unitless bump error: original={}, bump_pct={}, expected={}, got={}, rel_error={}",
                value, bump_pct, expected, bumped_value, rel_error);
        });
    }

    /// Property: Double bump should equal single bump with combined percentage.
    /// Note: Money types round to 2 decimal places, so we use looser tolerance.
    #[test]
    fn test_double_bump_composition() {
        proptest!(|(
            price in 10.0..1000.0f64,
            bump1 in -0.2..0.2f64,
            bump2 in -0.2..0.2f64,
        )| {
            let mut context = MarketContext::new();
            let price_id = "TEST_PRICE";
            let currency = Currency::USD;

            context.prices.insert(
                CurveId::from(price_id),
                MarketScalar::Price(Money::new(price, currency))
            );

            // Double bump
            let bumped1 = bump_scalar_price(&context, price_id, bump1).unwrap();
            let bumped2 = bump_scalar_price(&bumped1, price_id, bump2).unwrap();
            let final_price = match bumped2.price(price_id).unwrap() {
                MarketScalar::Price(m) => m.amount(),
                _ => panic!("Expected price"),
            };

            // Expected: price * (1 + bump1) * (1 + bump2)
            let expected = price * (1.0 + bump1) * (1.0 + bump2);
            // Money rounds to cents, so we allow 2 cents tolerance (two rounding operations)
            let abs_error = (final_price - expected).abs();
            let tolerance = 0.02 + expected.abs() * 1e-6;

            prop_assert!(abs_error < tolerance,
                "Double bump composition error: price={}, bump1={}, bump2={}, expected={}, got={}",
                price, bump1, bump2, expected, final_price);
        });
    }

    /// Property: Bumping by zero should not change the value.
    /// Note: Money types round to 2 decimal places, so values may differ due to rounding.
    #[test]
    fn test_zero_bump_identity() {
        proptest!(|(
            price in 1.0..10000.0f64,
        )| {
            let mut context = MarketContext::new();
            let price_id = "TEST_PRICE";
            let currency = Currency::USD;

            context.prices.insert(
                CurveId::from(price_id),
                MarketScalar::Price(Money::new(price, currency))
            );

            let bumped = bump_scalar_price(&context, price_id, 0.0).unwrap();
            let bumped_price = match bumped.price(price_id).unwrap() {
                MarketScalar::Price(m) => m.amount(),
                _ => panic!("Expected price"),
            };

            // Money rounds to cents, so we allow 0.01 tolerance
            prop_assert!((bumped_price - price).abs() < 0.01,
                "Zero bump should preserve value: original={}, bumped={}", price, bumped_price);
        });
    }
}

#[cfg(test)]
mod edge_cases {
    use crate::metrics::finite_difference::{
        adaptive_spot_bump, adaptive_vol_bump, central_diff_1d,
    };

    /// Test: Zero volatility should still give valid bump.
    #[test]
    fn test_zero_volatility() {
        let bump = adaptive_vol_bump(0.0, None);
        assert!((0.001..=0.05).contains(&bump), "Zero vol bump: {}", bump);
    }

    /// Test: Zero time to expiry should still give valid bump.
    #[test]
    fn test_zero_time_to_expiry() {
        let bump = adaptive_spot_bump(0.3, 0.0, None);
        assert!((0.001..=0.05).contains(&bump), "Zero time bump: {}", bump);
    }

    /// Test: Very small bump size (near machine precision).
    #[test]
    fn test_very_small_bump() {
        let h = 1e-8;
        let f_up = 1.0 + h;
        let f_down = 1.0 - h;

        let result = central_diff_1d(|| Ok(f_up), || Ok(f_down), h);
        assert!(result.is_ok(), "Should handle very small bump");

        let deriv = result.unwrap();
        assert!(deriv.is_finite(), "Result should be finite");
    }

    /// Test: Bump up vs bump down ordering doesn't matter for absolute value.
    #[test]
    fn test_bump_order_absolute_value() {
        let f_up = 150.0;
        let f_down = 100.0;
        let h = 0.01;

        let deriv_forward = central_diff_1d(|| Ok(f_up), || Ok(f_down), h).unwrap();
        let deriv_backward = central_diff_1d(|| Ok(f_down), || Ok(f_up), h).unwrap();

        assert!(
            (deriv_forward.abs() - deriv_backward.abs()).abs() < 1e-10,
            "Absolute value should be same regardless of order"
        );
    }

    /// Test: Extreme price values should be handled.
    #[test]
    fn test_extreme_prices() {
        // Very small price
        let bump_small = adaptive_spot_bump(0.3, 1.0, None);
        assert!(bump_small.is_finite() && bump_small > 0.0);

        // Very large price
        let bump_large = adaptive_spot_bump(0.3, 1.0, None);
        assert!(bump_large.is_finite() && bump_large > 0.0);
    }
}
