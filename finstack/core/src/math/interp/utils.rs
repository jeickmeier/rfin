//! Shared validation and search utilities for interpolators.
//!
//! Provides common helpers for knot validation, segment location, and
//! monotonicity checking used across all interpolation implementations.

use super::types::ExtrapolationPolicy;
use crate::{error::InputError, Error};

/// Helper to check and apply extrapolation if x is out of bounds.
///
/// If `x` is before the first knot, calls `on_left`.
/// If `x` is after the last knot, calls `on_right`.
/// Otherwise returns `None`.
#[inline]
pub fn check_extrapolation<F1, F2>(
    x: f64,
    knots: &[f64],
    extrapolation: ExtrapolationPolicy,
    on_left: F1,
    on_right: F2,
) -> Option<f64>
where
    F1: FnOnce(ExtrapolationPolicy) -> f64,
    F2: FnOnce(ExtrapolationPolicy) -> f64,
{
    if knots.is_empty() {
        return None;
    }

    // Left extrapolation
    if x <= knots[0] {
        return Some(on_left(extrapolation));
    }

    // Right extrapolation
    if let Some(&last_knot) = knots.last() {
        if x >= last_knot {
            return Some(on_right(extrapolation));
        }
    }

    None
}

/// Validate strictly increasing knots with length >= 2.
pub fn validate_knots(knots: &[f64]) -> crate::Result<()> {
    if knots.len() < 2 {
        return Err(InputError::TooFewPoints.into());
    }
    if knots.windows(2).any(|w| w[1] <= w[0]) {
        return Err(InputError::NonMonotonicKnots.into());
    }
    Ok(())
}

/// Locate segment index i such that xs[i] <= x <= xs[i+1].
#[inline(always)]
pub fn locate_segment(xs: &[f64], x: f64) -> Result<usize, Error> {
    if xs.is_empty() {
        return Err(InputError::TooFewPoints.into());
    }
    if x < xs[0] || x > *xs.last().expect("xs should not be empty (checked above)") {
        return Err(Error::InterpOutOfBounds);
    }
    let idx = xs.partition_point(|k| *k < x);
    Ok(if idx == 0 { 0 } else { idx - 1 })
}

/// Validate that all values are strictly positive.
pub fn validate_positive_series(values: &[f64]) -> crate::Result<()> {
    if values.iter().any(|&v| v <= 0.0) {
        return Err(InputError::NonPositiveValue.into());
    }
    Ok(())
}

/// Validate sequence is non-increasing (monotone) in addition to positivity.
pub fn validate_monotone_nonincreasing(values: &[f64]) -> crate::Result<()> {
    validate_positive_series(values)?;
    if values.windows(2).any(|w| w[1] > w[0]) {
        return Err(InputError::Invalid.into());
    }
    Ok(())
}
