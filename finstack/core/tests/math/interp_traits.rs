//! Tests for interpolation traits and default implementations

use finstack_core::math::interp::{InterpFn, LinearDf, LogLinearDf, ExtrapolationPolicy};

// Test interpolator using default derivative implementation
#[derive(Debug)]
struct TestInterp {
    value: f64,
}

impl InterpFn for TestInterp {
    fn interp(&self, _x: f64) -> f64 {
        self.value
    }
    // Uses default interp_prime implementation
}

#[test]
fn interp_fn_default_derivative_uses_finite_differences() {
    let interp = TestInterp { value: 42.0 };

    // Default derivative of constant function should be ~0
    let deriv = interp.interp_prime(1.0);
    assert!(deriv.abs() < 1e-6);
}

#[test]
fn interp_fn_default_derivative_for_linear_function() {
    // Create a simple decreasing linear interpolator (valid for discount factors)
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let values = vec![1.0, 0.5, 0.1].into_boxed_slice(); // Positive decreasing
    let interp = LinearDf::new(knots, values, ExtrapolationPolicy::FlatZero).unwrap();

    // Derivative of linear function should be constant (slope = -0.5)
    let deriv_0_5 = interp.interp_prime(0.5);
    let deriv_1_5 = interp.interp_prime(1.5);

    // Linear segment has constant slope
    assert!((deriv_0_5 - (-0.5)).abs() < 1e-6);
    assert!((deriv_1_5 - (-0.4)).abs() < 1e-6);
}

#[test]
fn interp_fn_derivative_for_exponential_function() {
    // Log-linear interpolation creates exponential decay
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();
    let interp = LogLinearDf::new(knots, dfs, ExtrapolationPolicy::FlatZero).unwrap();

    // Derivative should be negative (decreasing function)
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
    let values = vec![1.0, 0.8, 0.6, 0.4].into_boxed_slice(); // Positive decreasing
    let interp = LinearDf::new(knots, values, ExtrapolationPolicy::FlatZero).unwrap();

    // Test that derivative is approximately consistent with finite differences
    let x = 1.5;
    let h = 1e-8;
    let numerical = (interp.interp(x + h) - interp.interp(x - h)) / (2.0 * h);
    let analytical = interp.interp_prime(x);

    assert!((numerical - analytical).abs() < 1e-6);
}

#[test]
fn interp_fn_send_sync_bounds() {
    // Verify that InterpFn is Send + Sync
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn InterpFn>>();
}

#[test]
fn interp_fn_derivative_at_boundaries() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let values = vec![1.0, 0.5, 0.25].into_boxed_slice();
    let interp = LinearDf::new(knots, values, ExtrapolationPolicy::FlatZero).unwrap();

    // Test derivative at exact knot points
    let deriv_0 = interp.interp_prime(0.0);
    let deriv_1 = interp.interp_prime(1.0);
    let deriv_2 = interp.interp_prime(2.0);

    // All derivatives should be finite
    assert!(deriv_0.is_finite());
    assert!(deriv_1.is_finite());
    assert!(deriv_2.is_finite());

    // For decreasing function, derivatives should be non-positive
    assert!(deriv_0 <= 0.0);
    assert!(deriv_1 <= 0.0);
    assert!(deriv_2 <= 0.0);
}

#[test]
fn interp_fn_derivative_extrapolation_flat_zero() {
    let knots = vec![0.0, 1.0].into_boxed_slice();
    let values = vec![1.0, 0.5].into_boxed_slice();
    let interp = LinearDf::new(knots, values, ExtrapolationPolicy::FlatZero).unwrap();

    // Beyond boundaries with FlatZero, derivative should be 0
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

    // Beyond boundaries with FlatForward, derivative should match boundary slope
    let deriv_interior = interp.interp_prime(0.5);
    let deriv_below = interp.interp_prime(-0.5);
    let deriv_above = interp.interp_prime(2.0);

    // All should have same slope for linear interpolation
    assert!((deriv_interior - deriv_below).abs() < 1e-6);
    assert!((deriv_interior - deriv_above).abs() < 1e-6);
}

