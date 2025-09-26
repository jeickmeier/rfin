//! Internal helpers shared by one-dimensional term-structure builders.
//!
//! The module keeps curve builders small by extracting common logic for
//! interpolation setup, knot splitting, and serde state. Hazard curves do not
//! rely on the interpolation engine and therefore only reuse the serde helpers.

use crate::math::interp::types::Interp;
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{Error, Result, F};

/// Build an `Interp` with unified error mapping (crate::Result) for callers
/// whose builders return `crate::Result<T>` (Forward/Inflation).
#[inline]
pub(crate) fn build_interp(
    style: InterpStyle,
    knots: Box<[F]>,
    values: Box<[F]>,
    extrapolation: ExtrapolationPolicy,
) -> Result<Interp> {
    style
        .build_enum(knots, values, extrapolation)
        .map_err(|_| Error::Internal)
}

/// Build an `Interp` mapping errors to `super::CurveError` for callers whose
/// builders expose that alias (DiscountCurve).
#[inline]
pub(crate) fn build_interp_curve_error(
    style: InterpStyle,
    knots: Box<[F]>,
    values: Box<[F]>,
    extrapolation: ExtrapolationPolicy,
) -> core::result::Result<Interp, super::CurveError> {
    style
        .build_enum(knots, values, extrapolation)
        .map_err(|_| super::CurveError::NonPositiveValue)
}

/// Convenience to split points (t, v) into separate vectors.
#[inline]
pub(crate) fn split_points(points: Vec<(F, F)>) -> (Vec<F>, Vec<F>) {
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
    pub knot_points: Vec<(F, F)>,
}

#[cfg(feature = "serde")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct StateInterp {
    /// Interpolation style
    pub interp_style: InterpStyle,
    /// Extrapolation policy
    pub extrapolation: ExtrapolationPolicy,
}
