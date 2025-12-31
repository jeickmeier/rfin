//! Custom assertion helpers for risk tests.
//!
//! These helpers provide better error messages and consistent tolerance handling.

use super::tolerances;

/// Assert that two f64 values are approximately equal within an absolute tolerance.
///
/// Uses `#[track_caller]` to report the actual test location on failure.
///
/// # Panics
///
/// Panics if `|actual - expected| > tolerance`.
#[track_caller]
pub fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "assertion failed: values not approximately equal\n  \
         expected: {expected}\n  \
         actual:   {actual}\n  \
         diff:     {diff}\n  \
         tol:      {tolerance}"
    );
}

/// Assert that two f64 values are approximately equal within the standard tolerance.
///
/// Uses `tolerances::STANDARD` (1e-6) as the default tolerance.
#[track_caller]
pub fn assert_approx_eq_std(actual: f64, expected: f64) {
    assert_approx_eq(actual, expected, tolerances::STANDARD);
}

/// Assert that two f64 values are approximately equal within a relative tolerance.
///
/// The comparison is: `|actual - expected| / |expected| <= rel_tolerance`
/// Falls back to absolute comparison if `expected` is near zero.
///
/// # Panics
///
/// Panics if the relative difference exceeds `rel_tolerance`.
#[track_caller]
pub fn assert_relative_eq(actual: f64, expected: f64, rel_tolerance: f64) {
    if expected.abs() < tolerances::NEAR_ZERO {
        // Fall back to absolute comparison for near-zero expected values
        assert_approx_eq(actual, expected, tolerances::STANDARD);
    } else {
        let rel_diff = ((actual - expected) / expected).abs();
        assert!(
            rel_diff <= rel_tolerance,
            "assertion failed: relative difference too large\n  \
             expected:  {expected}\n  \
             actual:    {actual}\n  \
             rel_diff:  {:.4}%\n  \
             tolerance: {:.4}%",
            rel_diff * 100.0,
            rel_tolerance * 100.0
        );
    }
}

/// Assert that a value is positive.
#[track_caller]
pub fn assert_positive(value: f64, name: &str) {
    assert!(
        value > 0.0,
        "assertion failed: {name} should be positive, got {value}"
    );
}

/// Assert that a value is negative.
#[track_caller]
pub fn assert_negative(value: f64, name: &str) {
    assert!(
        value < 0.0,
        "assertion failed: {name} should be negative, got {value}"
    );
}

/// Assert that a value is non-negative.
#[track_caller]
pub fn assert_non_negative(value: f64, name: &str) {
    assert!(
        value >= 0.0,
        "assertion failed: {name} should be non-negative, got {value}"
    );
}

/// Assert that a value is finite (not NaN or infinity).
#[track_caller]
pub fn assert_finite(value: f64, name: &str) {
    assert!(
        value.is_finite(),
        "assertion failed: {name} should be finite, got {value}"
    );
}

/// Assert that a value is within a given range [min, max].
#[track_caller]
pub fn assert_in_range(value: f64, min: f64, max: f64, name: &str) {
    assert!(
        (min..=max).contains(&value),
        "assertion failed: {name} should be in range [{min}, {max}], got {value}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_approx_eq_passes() {
        assert_approx_eq(1.0, 1.0 + 1e-11, tolerances::TIGHT);
    }

    #[test]
    #[should_panic(expected = "values not approximately equal")]
    fn test_assert_approx_eq_fails() {
        assert_approx_eq(1.0, 2.0, tolerances::TIGHT);
    }

    #[test]
    fn test_assert_relative_eq_passes() {
        assert_relative_eq(100.5, 100.0, tolerances::PERCENT_1);
    }

    #[test]
    #[should_panic(expected = "relative difference too large")]
    fn test_assert_relative_eq_fails() {
        assert_relative_eq(110.0, 100.0, tolerances::PERCENT_1);
    }
}
