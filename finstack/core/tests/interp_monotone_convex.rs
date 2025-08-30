//! Tests for monotone convex interpolation.

mod common;
use finstack_core::market_data::interp::{monotone_convex::MonotoneConvex, InterpFn};

#[test]
fn test_monotone_convex_construction() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let interp = MonotoneConvex::new(knots, dfs);
    assert!(interp.is_ok());
}

#[test]
fn test_monotone_convex_exact_knot_lookup() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let interp = MonotoneConvex::new(knots, dfs).unwrap();

    // Exact knot values should return exact discount factors
    assert_eq!(interp.interp(0.0), 1.0);
    assert_eq!(interp.interp(1.0), 0.95);
    assert_eq!(interp.interp(2.0), 0.9);
}

#[test]
fn test_monotone_convex_interpolation() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let interp = MonotoneConvex::new(knots, dfs).unwrap();

    // Interpolated value should be between the surrounding knots
    let mid_value = interp.interp(0.5);
    assert!(mid_value > 0.95 && mid_value < 1.0);

    let mid_value2 = interp.interp(1.5);
    assert!(mid_value2 > 0.9 && mid_value2 < 0.95);
}

#[test]
fn test_monotone_convex_validation_errors() {
    // Test non-increasing knots
    let bad_knots = vec![1.0, 0.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let result = MonotoneConvex::new(bad_knots, dfs);
    assert!(result.is_err());

    // Test non-positive discount factors
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let bad_dfs = vec![1.0, -0.95, 0.9].into_boxed_slice();

    let result = MonotoneConvex::new(knots, bad_dfs);
    assert!(result.is_err());

    // Test increasing discount factors (arbitrage)
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let bad_dfs = vec![0.9, 0.95, 1.0].into_boxed_slice(); // Increasing

    let result = MonotoneConvex::new(knots, bad_dfs);
    assert!(result.is_err());
}

#[test]
fn test_monotone_convex_monotonicity() {
    // Test that interpolation preserves monotone decreasing shape
    let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9, 0.85].into_boxed_slice();

    let interp = MonotoneConvex::new(knots, dfs).unwrap();

    // Check that interpolated values maintain monotone decreasing property
    let val_0_5 = interp.interp(0.5);
    let val_1_5 = interp.interp(1.5);
    let val_2_5 = interp.interp(2.5);

    assert!(val_0_5 >= val_1_5);
    assert!(val_1_5 >= val_2_5);

    // All values should be positive
    assert!(val_0_5 > 0.0);
    assert!(val_1_5 > 0.0);
    assert!(val_2_5 > 0.0);
}

#[test]
fn test_monotone_convex_shape_preservation() {
    // Test that the interpolator preserves the general shape of the input curve
    let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9, 0.85].into_boxed_slice();

    let interp = MonotoneConvex::new(knots, dfs).unwrap();

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

#[test]
fn test_monotone_convex_near_flat_curve() {
    // Test the EPS path with near-flat discount factors
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.999999, 0.999998].into_boxed_slice(); // Very small slopes

    let interp = MonotoneConvex::new(knots, dfs).unwrap();

    // Should handle near-zero slopes gracefully
    let mid_value = interp.interp(0.5);
    assert!(mid_value > 0.999998 && mid_value <= 1.0);
    assert!(mid_value.is_finite());
}

#[test]
fn test_monotone_convex_edge_cases() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let interp = MonotoneConvex::new(knots, dfs).unwrap();

    // Test values very close to boundaries (within bounds)
    let near_start = interp.interp(0.001);
    let near_end = interp.interp(1.999);

    // Values should be reasonable (not NaN or infinite)
    assert!(near_start.is_finite());
    assert!(near_end.is_finite());
    assert!(near_start > 0.0);
    assert!(near_end > 0.0);
}

#[test]
fn test_monotone_convex_two_point_case() {
    // Test with minimum number of points
    let knots = vec![0.0, 1.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95].into_boxed_slice();

    let interp = MonotoneConvex::new(knots, dfs).unwrap();

    // Should interpolate reasonably between the two points
    let mid_value = interp.interp(0.5);
    assert!(mid_value > 0.95 && mid_value < 1.0);
    assert!(mid_value.is_finite());
}
