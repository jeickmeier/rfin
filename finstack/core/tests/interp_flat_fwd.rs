//! Tests for flat forward interpolation.

mod common;

use common::approx_eq;
use finstack_core::market_data::interp::{flat_fwd::FlatFwd, log_linear::LogLinearDf, InterpFn};

#[test]
fn test_flat_fwd_construction() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let interp = FlatFwd::new(knots, dfs);
    assert!(interp.is_ok());
}

#[test]
fn test_flat_fwd_exact_knot_lookup() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let interp = FlatFwd::new(knots, dfs).unwrap();

    // Exact knot values should return exact discount factors
    assert_eq!(interp.interp(0.0), 1.0);
    assert_eq!(interp.interp(1.0), 0.95);
    assert_eq!(interp.interp(2.0), 0.9);
}

#[test]
fn test_flat_fwd_equals_log_linear() {
    // FlatFwd should behave identically to LogLinearDf
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let flat_fwd = FlatFwd::new(knots.clone(), dfs.clone()).unwrap();
    let log_linear = LogLinearDf::new(knots, dfs).unwrap();

    // Test several interpolation points
    let test_points = [0.25, 0.5, 0.75, 1.25, 1.5, 1.75];

    for &t in &test_points {
        let flat_val = flat_fwd.interp(t);
        let log_val = log_linear.interp(t);
        assert!(
            approx_eq(flat_val, log_val, 1e-15),
            "Mismatch at t={}: flat_fwd={}, log_linear={}",
            t,
            flat_val,
            log_val
        );
    }
}

#[test]
fn test_flat_fwd_interpolation() {
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let interp = FlatFwd::new(knots, dfs).unwrap();

    // Interpolated value should be between the surrounding knots
    let mid_value = interp.interp(0.5);
    assert!(mid_value > 0.95 && mid_value < 1.0);

    let mid_value2 = interp.interp(1.5);
    assert!(mid_value2 > 0.9 && mid_value2 < 0.95);
}

#[test]
fn test_flat_fwd_validation_via_underlying() {
    // Test that validation errors are properly propagated from LogLinearDf

    // Test non-increasing knots
    let bad_knots = vec![1.0, 0.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let result = FlatFwd::new(bad_knots, dfs);
    assert!(result.is_err());

    // Test non-positive discount factors
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let bad_dfs = vec![1.0, -0.95, 0.9].into_boxed_slice();

    let result = FlatFwd::new(knots, bad_dfs);
    assert!(result.is_err());
}

#[test]
fn test_flat_fwd_constant_forward_rate_property() {
    // Test that flat forward interpolation maintains constant instantaneous forward rates
    let knots = vec![0.0, 1.0, 2.0].into_boxed_slice();
    let dfs = vec![1.0, 0.95, 0.9].into_boxed_slice();

    let interp = FlatFwd::new(knots, dfs).unwrap();

    // For flat forward, the forward rate between any two points in a segment should be constant
    // This is equivalent to log-linear interpolation of discount factors
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
