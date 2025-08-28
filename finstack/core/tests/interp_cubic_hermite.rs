//! Tests for cubic Hermite interpolation.

mod common;

use common::approx_eq;
use finstack_core::market_data::interp::{cubic_hermite::CubicHermite, InterpFn};

#[test]
fn test_cubic_hermite_construction() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();
    
    let interp = CubicHermite::new(knots, dfs);
    assert!(interp.is_ok());
}

#[test]
fn test_cubic_hermite_exact_knot_lookup() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();
    
    let interp = CubicHermite::new(knots, dfs).unwrap();
    
    // Exact knot values should return exact discount factors
    assert_eq!(interp.interp(0.0), 1.0);
    assert_eq!(interp.interp(1.0), 0.95);
    assert_eq!(interp.interp(2.0), 0.9);
}

#[test]
fn test_cubic_hermite_interpolation() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();
    
    let interp = CubicHermite::new(knots, dfs).unwrap();
    
    // Interpolated value should be between the surrounding knots
    let mid_value = interp.interp(0.5);
    assert!(mid_value > 0.95 && mid_value < 1.0);
    
    let mid_value2 = interp.interp(1.5);
    assert!(mid_value2 > 0.9 && mid_value2 < 0.95);
}

#[test]
fn test_cubic_hermite_two_point_case() {
    // Two-point case should exercise linear-slope fast path
    let knots = vec![0.0, 1.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95].into_boxed_slice();
    
    let interp = CubicHermite::new(knots, dfs).unwrap();
    
    // Should interpolate linearly between the two points
    let mid_value = interp.interp(0.5);
    assert!(approx_eq(mid_value, 0.975, 1e-10)); // Linear interpolation
}

#[test]
fn test_cubic_hermite_validation_errors() {
    // Test non-increasing knots
    let bad_knots = vec![1.0, 0.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();
    
    let result = CubicHermite::new(bad_knots, dfs);
    assert!(result.is_err());
    
    // Test non-positive discount factors
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let bad_dfs = vec![1.0, -0.95, 0.9].into_boxed_slice();
    
    let result = CubicHermite::new(knots, bad_dfs);
    assert!(result.is_err());
}

#[test]
fn test_cubic_hermite_monotone_shape() {
    // Test that interpolation preserves monotone decreasing shape
    let knots = vec![0.0, 1.0, 2.0, 3.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9, 0.85].into_boxed_slice();
    
    let interp = CubicHermite::new(knots, dfs).unwrap();
    
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
fn test_cubic_hermite_edge_cases() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();
    
    let interp = CubicHermite::new(knots, dfs).unwrap();
    
    // Test values very close to boundaries (within bounds)
    let near_start = interp.interp(0.001);
    let near_end = interp.interp(1.999);
    
    // Values should be reasonable (not NaN or infinite)
    assert!(near_start.is_finite());
    assert!(near_end.is_finite());
    assert!(near_start > 0.0);
    assert!(near_end > 0.0);
}
