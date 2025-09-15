//! Internal helpers for 1D term structures (knots + values + Interp).
//!
//! This DRYs common builder and serde plumbing across Discount/Forward/Inflation
//! curves while keeping wire formats stable. Hazard curves intentionally do not
//! use the `Interp` engine and are excluded.

use crate::math::interp::types::Interp;
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::{Error, Result, F};

/// Simple holder for a 1D grid (times and values) used to build an `Interp`.
#[derive(Debug, Clone)]
pub(crate) struct OneDGrid {
    pub knots: Box<[F]>,
    pub values: Box<[F]>,
}

impl OneDGrid {
    #[inline]
    pub fn new(knots: Box<[F]>, values: Box<[F]>) -> Self {
        Self { knots, values }
    }

    // Keep minimal API; unused helpers removed to avoid dead_code warnings.
}

/// Build an `Interp` with unified error mapping (crate::Result) for callers
/// whose builders return `crate::Result<T>` (Forward/Inflation).
#[inline]
pub(crate) fn build_interp(
    style: InterpStyle,
    grid: &OneDGrid,
    extrapolation: ExtrapolationPolicy,
) -> Result<Interp> {
    style
        .build_enum(grid.knots.clone(), grid.values.clone(), extrapolation)
        .map_err(|_| Error::Internal)
}

/// Build an `Interp` mapping errors to `super::CurveError` for callers whose
/// builders expose that alias (DiscountCurve).
#[inline]
pub(crate) fn build_interp_curve_error(
    style: InterpStyle,
    grid: &OneDGrid,
    extrapolation: ExtrapolationPolicy,
) -> core::result::Result<Interp, super::CurveError> {
    style
        .build_enum(grid.knots.clone(), grid.values.clone(), extrapolation)
        .map_err(|_| super::CurveError::NonPositiveValue)
}

/// Convenience to split points (t, v) into separate vectors.
#[inline]
pub(crate) fn split_points(points: Vec<(F, F)>) -> (Vec<F>, Vec<F>) {
    points.into_iter().unzip()
}

/// Unified builder trait for 1D term structures.
///
/// This provides a consistent surface for common builder operations used by most
/// curves. Methods have default no-op implementations so builders that don't use
/// a particular setting (e.g., `base_date` on inflation, `set_interp` on hazard)
/// can still implement the trait without extra boilerplate.
pub trait CurveBuilder: Sized {
    /// Concrete curve type produced by this builder
    type Output;

    /// Optional valuation base date (ignored by builders that don't use dates)
    fn base_date(self, _date: crate::dates::Date) -> Self {
        self
    }

    /// Supply knot points `(t, value)`
    fn knots<I>(self, _pts: I) -> Self
    where
        I: IntoIterator<Item = (F, F)>,
    {
        self
    }

    /// Select interpolation style (ignored by builders without an interpolator)
    fn set_interp(self, _style: InterpStyle) -> Self {
        self
    }

    /// Finalize and build the concrete curve
    fn build(self) -> crate::Result<Self::Output>;
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
