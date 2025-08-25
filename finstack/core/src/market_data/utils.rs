//! Shared helper functions for the *market-data* layer.
//!
//! These utilities are intentionally **`no_std`–compatible** (they only depend
//! on the parent crate and `core`) so they can be leveraged by WASM builds and
//! other constrained targets.
//!
//! The helpers are kept in a dedicated module so that they can be re-used by
//! several interpolation schemes and term-structure builders without code
//! duplication.
//!
//! # API Overview
//! | Function            | Purpose                                                    |
//! |---------------------|------------------------------------------------------------|
//! | [`validate_knots`]  | Ensures knot times are strictly increasing and non-empty. |
//! | [`locate_segment`]  | Binary-search helper returning the left index of a segment |
//! | [`validate_dfs`]    | Checks discount-factor arrays for positivity/monotonicity. |
//!
//! ## Example – Validating input before bootstrapping a curve
//! ```rust
//! use rfin_core::market_data::utils::{validate_knots, validate_dfs};
//! // Knot times in *years* and corresponding discount factors.
//! let times  = [0.0, 1.0, 2.0, 3.0];
//! let dfs    = [1.0, 0.97, 0.94, 0.90];
//! // Ensure both vectors are well-formed *before* constructing the curve.
//! validate_knots(&times).unwrap();
//! validate_dfs(&dfs, true).unwrap();
//! ```

use crate::error::InputError;
use crate::Error;
use crate::F;

/// Validate that a slice of knot times is strictly ascending (monotonically increasing) and contains at least two points.
///
/// Returns `Ok(())` when the slice is valid; otherwise returns
/// `Err(Error::InvalidInput)`.
pub fn validate_knots(knots: &[F]) -> crate::Result<()> {
    if knots.len() < 2 {
        return Err(InputError::TooFewPoints.into());
    }
    if knots.windows(2).any(|w| w[1] <= w[0]) {
        return Err(InputError::NonMonotonicKnots.into());
    }
    Ok(())
}

/// Locate the segment index `i` such that `xs[i] <= x <= xs[i+1]`.
///
/// The slice `xs` must be non-empty and strictly increasing (typically
/// pre-validated by [`validate_knots`]).
///
/// # Errors
/// * [`Error::InterpOutOfBounds`] – when `x` lies outside the range of
///   `xs`.
#[inline(always)]
pub fn locate_segment(xs: &[F], x: F) -> Result<usize, Error> {
    debug_assert!(!xs.is_empty(), "knots slice cannot be empty");

    if x < xs[0] || x > *xs.last().unwrap() {
        return Err(Error::InterpOutOfBounds);
    }
    // `partition_point` returns the index of the first element that is *not*
    // strictly less than `x`.  For interior points we need the left segment,
    // hence `idx - 1` except when `x` is smaller than every element.
    let idx = xs.partition_point(|k| *k < x);
    Ok(if idx == 0 { 0 } else { idx - 1 })
}

/// Validate a slice of discount factors (DF).
///
/// * Ensures every DF is strictly positive.
/// * When `monotone` is `true`, additionally checks the sequence is
///   non-increasing (arbitrage-free).
///
/// Returns `Ok(())` on success or an [`InputError`] detailing the failure.
pub fn validate_dfs(dfs: &[F], monotone: bool) -> crate::Result<()> {
    use crate::error::InputError;

    if dfs.iter().any(|&d| d <= 0.0) {
        return Err(InputError::NonPositiveValue.into());
    }

    if monotone && dfs.windows(2).any(|w| w[1] > w[0]) {
        return Err(InputError::Invalid.into());
    }

    Ok(())
}
