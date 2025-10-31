//! Shared helpers for interpolation (pure math, no market_data deps).

use crate::{error::InputError, Error};

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
    debug_assert!(!xs.is_empty(), "knots slice cannot be empty");
    if x < xs[0]
        || x > *xs
            .last()
            .expect("xs should not be empty (checked by debug_assert)")
    {
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
