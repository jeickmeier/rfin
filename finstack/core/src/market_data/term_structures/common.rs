//! Internal helpers shared by one-dimensional term-structure builders.
//!
//! The module keeps curve builders small by extracting common logic for
//! interpolation setup, knot splitting, and serde state. Hazard curves do not
//! rely on the interpolation engine and therefore only reuse the serde helpers.

use crate::math::interp::types::Interp;
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{Error, Result};

/// Build an `Interp` with unified error mapping (crate::Result) for callers
/// whose builders return `crate::Result<T>` (Forward/Inflation).
#[inline]
pub(crate) fn build_interp(
    style: InterpStyle,
    knots: Box<[f64]>,
    values: Box<[f64]>,
    extrapolation: ExtrapolationPolicy,
) -> Result<Interp> {
    style
        .build_enum(knots, values, extrapolation)
        .map_err(|_| Error::Internal)
}

/// Build an `Interp` allowing any values (including negative forward rates).
///
/// This is used by forward curves where negative rates are allowed
/// (e.g., EUR, CHF, JPY markets since 2014).
#[inline]
pub(crate) fn build_interp_allow_any_values(
    style: InterpStyle,
    knots: Box<[f64]>,
    values: Box<[f64]>,
    extrapolation: ExtrapolationPolicy,
) -> Result<Interp> {
    style
        .build_enum_allow_any_values(knots, values, extrapolation)
        .map_err(|_| Error::Internal)
}

/// Build an `Interp` mapping errors to `InputError` for discount curve builders.
#[inline]
pub(crate) fn build_interp_input_error(
    style: InterpStyle,
    knots: Box<[f64]>,
    values: Box<[f64]>,
    extrapolation: ExtrapolationPolicy,
) -> crate::Result<Interp> {
    // Preserve the original interpolation error (usually an InputError).
    style.build_enum(knots, values, extrapolation)
}

/// Convenience to split points (t, v) into separate vectors.
#[inline]
pub(crate) fn split_points(points: Vec<(f64, f64)>) -> (Vec<f64>, Vec<f64>) {
    points.into_iter().unzip()
}

// -----------------------------------------------------------------------------
// Shared serde state fragments to DRY curve state definitions
// -----------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateId {
    /// Curve identifier
    pub id: String,
}

#[cfg(feature = "serde")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateKnotPoints {
    /// Time/value pairs used to construct the curve
    pub knot_points: Vec<(f64, f64)>,
}

#[cfg(feature = "serde")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateInterp {
    /// Interpolation style
    pub interp_style: InterpStyle,
    /// Extrapolation policy
    pub extrapolation: ExtrapolationPolicy,
}
