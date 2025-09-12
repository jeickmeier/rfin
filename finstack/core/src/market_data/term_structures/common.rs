//! Internal helpers for 1D term structures (knots + values + Interp).
//!
//! This DRYs common builder and serde plumbing across Discount/Forward/Inflation
//! curves while keeping wire formats stable. Hazard curves intentionally do not
//! use the `Interp` engine and are excluded.

use crate::market_data::interp::{ExtrapolationPolicy, InterpStyle};
use crate::market_data::interp::types::Interp;
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


