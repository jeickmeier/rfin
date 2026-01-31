//! Tests proving deprecated APIs produce identical results to canonical APIs.
//!
//! These tests use `#[allow(deprecated)]` to suppress warnings while verifying
//! functional equivalence between deprecated convenience functions and their
//! canonical replacements.
//!
//! # Purpose
//!
//! These tests ensure that:
//! 1. Deprecated APIs continue to work correctly during the deprecation period
//! 2. Migration to canonical APIs produces identical results
//! 3. No behavior changes were accidentally introduced
//!
//! # Test Structure
//!
//! Each test compares:
//! - **Deprecated path**: The convenience API being deprecated
//! - **Canonical path**: The recommended replacement API

#![allow(deprecated)]

use finstack_core::cashflow::{npv, npv_constant, InternalRateOfReturn};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::term_structures::FlatCurve;
use finstack_core::math::GaussHermiteQuadrature;
use finstack_core::money::Money;
use time::Month;

/// Tolerance for floating-point comparisons.
const TOLERANCE: f64 = 1e-12;

/// Tolerance for IRR comparisons (solver tolerance).
const IRR_TOLERANCE: f64 = 1e-6;

/// Helper to create dates.
fn d(year: i32, month: u8, day: u8) -> Date {
    Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
}

// =============================================================================
// NPV Constant Equivalence Tests
// =============================================================================

mod npv_equivalence {
    use super::*;

    /// Test that `npv_constant` produces identical results to `npv` with `FlatCurve`.
    #[test]
    fn npv_constant_matches_npv_with_flat_curve() {
        let base = d(2024, 1, 1);
        let flows = vec![
            (base, Money::new(-100_000.0, Currency::USD)),
            (d(2025, 1, 1), Money::new(110_000.0, Currency::USD)),
        ];
        let rate = 0.05;
        let dc = DayCount::Act365F;

        // Deprecated path
        let pv_deprecated = npv_constant(&flows, rate, base, dc).unwrap();

        // Canonical path: convert to continuous rate and use FlatCurve
        let continuous_rate = (1.0 + rate).ln();
        let curve = FlatCurve::new(continuous_rate, base, dc, "TEST");
        let pv_canonical = npv(&curve, base, Some(dc), &flows).unwrap();

        assert!(
            (pv_deprecated.amount() - pv_canonical.amount()).abs() < TOLERANCE,
            "npv_constant ({}) should match npv with FlatCurve ({})",
            pv_deprecated.amount(),
            pv_canonical.amount()
        );
    }

    /// Test equivalence across various discount rates.
    #[test]
    fn npv_constant_equivalence_various_rates() {
        let base = d(2024, 1, 1);
        let flows = vec![
            (base, Money::new(-50_000.0, Currency::USD)),
            (d(2024, 7, 1), Money::new(25_000.0, Currency::USD)),
            (d(2025, 1, 1), Money::new(30_000.0, Currency::USD)),
        ];

        for rate in [0.0, 0.01, 0.05, 0.10, 0.25] {
            let dc = DayCount::Act365F;

            let pv_deprecated = npv_constant(&flows, rate, base, dc).unwrap();

            let continuous_rate = (1.0 + rate).ln();
            let curve = FlatCurve::new(continuous_rate, base, dc, "TEST");
            let pv_canonical = npv(&curve, base, Some(dc), &flows).unwrap();

            assert!(
                (pv_deprecated.amount() - pv_canonical.amount()).abs() < TOLERANCE,
                "Mismatch at rate {}: deprecated={}, canonical={}",
                rate,
                pv_deprecated.amount(),
                pv_canonical.amount()
            );
        }
    }

    /// Test equivalence with multiple cashflows.
    #[test]
    fn npv_constant_equivalence_multiple_cashflows() {
        let base = d(2024, 1, 1);
        let flows: Vec<(Date, Money)> = (0..12)
            .map(|i| {
                let date = d(2024, (i + 1) as u8, 1);
                (date, Money::new(1000.0, Currency::USD))
            })
            .collect();

        let rate = 0.08;
        let dc = DayCount::Act365F;

        let pv_deprecated = npv_constant(&flows, rate, base, dc).unwrap();

        let continuous_rate = (1.0 + rate).ln();
        let curve = FlatCurve::new(continuous_rate, base, dc, "TEST");
        let pv_canonical = npv(&curve, base, Some(dc), &flows).unwrap();

        assert!(
            (pv_deprecated.amount() - pv_canonical.amount()).abs() < TOLERANCE,
            "12-month cashflows: deprecated={}, canonical={}",
            pv_deprecated.amount(),
            pv_canonical.amount()
        );
    }

    /// Test equivalence with different day count conventions.
    #[test]
    fn npv_constant_equivalence_different_day_counts() {
        let base = d(2024, 1, 1);
        let flows = vec![
            (base, Money::new(-100.0, Currency::USD)),
            (d(2024, 7, 1), Money::new(105.0, Currency::USD)),
        ];
        let rate = 0.05;

        for dc in [DayCount::Act365F, DayCount::Act360, DayCount::Thirty360] {
            let pv_deprecated = npv_constant(&flows, rate, base, dc).unwrap();

            let continuous_rate = (1.0 + rate).ln();
            let curve = FlatCurve::new(continuous_rate, base, dc, "TEST");
            let pv_canonical = npv(&curve, base, Some(dc), &flows).unwrap();

            assert!(
                (pv_deprecated.amount() - pv_canonical.amount()).abs() < TOLERANCE,
                "Day count {:?}: deprecated={}, canonical={}",
                dc,
                pv_deprecated.amount(),
                pv_canonical.amount()
            );
        }
    }
}

// =============================================================================
// IRR API Tests
// =============================================================================
//
// Note: The `irr()` method on `[(Date, f64)]` is NOT deprecated because Rust
// doesn't support deprecating trait method implementations selectively.
// Instead, documentation recommends using `irr_with_daycount()` for explicit
// day count control. These tests verify that `irr()` uses Act365F by default.

mod irr_api {
    use super::*;

    /// Test that `irr()` on dated flows uses Act365F default and matches explicit call.
    #[test]
    fn dated_irr_uses_act365f_default() {
        let flows: Vec<(Date, f64)> = vec![
            (d(2024, 1, 1), -100_000.0),
            (d(2024, 7, 1), 5_000.0),
            (d(2025, 1, 1), 105_000.0),
        ];

        // irr() uses hidden Act365F default
        let irr_default = flows.as_slice().irr(None).unwrap();

        // Explicit day count should match
        let irr_explicit = flows
            .as_slice()
            .irr_with_daycount(DayCount::Act365F, None)
            .unwrap();

        assert!(
            (irr_default - irr_explicit).abs() < IRR_TOLERANCE,
            "irr() should use Act365F default: irr()={}, irr_with_daycount(Act365F)={}",
            irr_default,
            irr_explicit
        );
    }

    /// Test that periodic IRR is the canonical path for `[f64]`.
    #[test]
    fn periodic_irr_is_canonical() {
        // For [f64] (periodic flows), irr() is the canonical path
        let flows = [-100.0, 10.0, 10.0, 10.0, 110.0];

        let irr1 = flows.as_slice().irr(None).unwrap();
        let irr2 = flows
            .as_slice()
            .irr_with_daycount(DayCount::Act365F, None)
            .unwrap();

        // For periodic flows, both should give same result (day count is ignored)
        assert!(
            (irr1 - irr2).abs() < IRR_TOLERANCE,
            "Periodic IRR should be consistent: irr()={}, irr_with_daycount()={}",
            irr1,
            irr2
        );
    }

    /// Test IRR consistency with various cashflow patterns.
    #[test]
    fn dated_irr_consistency_various_patterns() {
        let test_cases = vec![
            // Simple: invest, receive
            vec![(d(2024, 1, 1), -100_000.0), (d(2025, 1, 1), 110_000.0)],
            // Multiple distributions
            vec![
                (d(2024, 1, 1), -100_000.0),
                (d(2024, 6, 1), 30_000.0),
                (d(2025, 1, 1), 80_000.0),
            ],
            // High return
            vec![(d(2024, 1, 1), -50_000.0), (d(2025, 1, 1), 150_000.0)],
            // Low return
            vec![(d(2024, 1, 1), -100_000.0), (d(2025, 1, 1), 101_000.0)],
        ];

        for flows in test_cases {
            let irr_default = flows.as_slice().irr(None).unwrap();
            let irr_explicit = flows
                .as_slice()
                .irr_with_daycount(DayCount::Act365F, None)
                .unwrap();

            assert!(
                (irr_default - irr_explicit).abs() < IRR_TOLERANCE,
                "IRR mismatch for flows {:?}: irr()={}, irr_with_daycount()={}",
                flows.len(),
                irr_default,
                irr_explicit
            );
        }
    }

    /// Test that different day counts produce different (but related) results.
    #[test]
    fn irr_day_count_affects_result() {
        let flows: Vec<(Date, f64)> = vec![(d(2024, 1, 1), -100_000.0), (d(2024, 7, 1), 102_500.0)];

        let irr_365f = flows
            .as_slice()
            .irr_with_daycount(DayCount::Act365F, None)
            .unwrap();
        let irr_360 = flows
            .as_slice()
            .irr_with_daycount(DayCount::Act360, None)
            .unwrap();

        // Different day counts should produce different rates
        assert!(
            (irr_365f - irr_360).abs() > 1e-4,
            "Different day counts should produce different IRRs"
        );

        // Both should be positive for this profitable trade
        assert!(irr_365f > 0.0);
        assert!(irr_360 > 0.0);
    }
}

// =============================================================================
// Gauss-Hermite Quadrature Equivalence Tests
// =============================================================================

mod quadrature_equivalence {
    use super::*;

    /// Test that deprecated `order_N()` constructors match `new(N)`.
    #[test]
    fn order_constructors_match_new() {
        let orders = [5, 7, 10, 15, 20];

        for order in orders {
            let from_new = GaussHermiteQuadrature::new(order).expect("valid order");

            let from_constructor = match order {
                5 => GaussHermiteQuadrature::order_5(),
                7 => GaussHermiteQuadrature::order_7(),
                10 => GaussHermiteQuadrature::order_10(),
                15 => GaussHermiteQuadrature::order_15(),
                20 => GaussHermiteQuadrature::order_20(),
                _ => unreachable!(),
            };

            // Test with a known integral: E[X²] = 1 for standard normal
            let f = |x: f64| x * x;

            let result_new = from_new.integrate(f);
            let result_constructor = from_constructor.integrate(f);

            assert!(
                (result_new - result_constructor).abs() < 1e-12,
                "order_{} should match new({}): constructor={}, new={}",
                order,
                order,
                result_constructor,
                result_new
            );
        }
    }

    /// Test integration correctness with known integrals.
    #[test]
    fn quadrature_integration_correctness() {
        // E[X²] = 1 for standard normal
        let expected_x2 = 1.0;
        // E[X⁴] = 3 for standard normal
        let expected_x4 = 3.0;

        for order in [5, 7, 10, 15, 20] {
            let quad = GaussHermiteQuadrature::new(order).expect("valid order");

            let result_x2 = quad.integrate(|x| x * x);
            let result_x4 = quad.integrate(|x| x.powi(4));

            // Higher order should be more accurate
            let tolerance_x2 = if order >= 7 { 1e-6 } else { 0.1 };
            let tolerance_x4 = if order >= 10 { 1e-6 } else { 0.5 };

            assert!(
                (result_x2 - expected_x2).abs() < tolerance_x2,
                "Order {}: E[X²] should be ~1.0, got {}",
                order,
                result_x2
            );
            assert!(
                (result_x4 - expected_x4).abs() < tolerance_x4,
                "Order {}: E[X⁴] should be ~3.0, got {}",
                order,
                result_x4
            );
        }
    }

    /// Test that serialization/deserialization preserves behavior.
    #[test]
    fn quadrature_serde_equivalence() {
        for order in [5, 7, 10, 15, 20] {
            let original = GaussHermiteQuadrature::new(order).expect("valid order");

            // Serialize and deserialize
            let json = serde_json::to_string(&original).unwrap();
            let restored: GaussHermiteQuadrature = serde_json::from_str(&json).unwrap();

            // Test with integration
            let f = |x: f64| x * x;
            let result_original = original.integrate(f);
            let result_restored = restored.integrate(f);

            assert!(
                (result_original - result_restored).abs() < 1e-12,
                "Serde roundtrip should preserve behavior for order {}: original={}, restored={}",
                order,
                result_original,
                result_restored
            );
        }
    }

    /// Test that `new()` returns `None` for unsupported orders.
    #[test]
    fn new_rejects_unsupported_orders() {
        let unsupported = [
            0, 1, 2, 3, 4, 6, 8, 9, 11, 12, 13, 14, 16, 17, 18, 19, 21, 100,
        ];

        for order in unsupported {
            assert!(
                GaussHermiteQuadrature::new(order).is_err(),
                "Order {} should be unsupported",
                order
            );
        }
    }
}
