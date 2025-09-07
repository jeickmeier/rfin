//! Interpolation types, enums, and factory.

extern crate alloc;

use alloc::boxed::Box;

use crate::F;

use super::traits::InterpFn;
use super::{
    cubic_hermite::CubicHermite, flat_fwd::FlatFwd, linear::LinearDf, log_linear::LogLinearDf,
    monotone_convex::MonotoneConvex,
};

/// Epsilon for finite difference derivative calculations.
pub const DERIVATIVE_EPSILON: F = 1e-6;

/// Extrapolation policy for evaluation outside the knot range.
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ExtrapolationPolicy {
    /// Constant value extension.
    #[default]
    FlatZero,
    /// Tangent/forward extension using boundary slope.
    FlatForward,
}

/// Enum of supported interpolation styles. The default is `Linear`.
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum InterpStyle {
    /// Linear interpolation in values.
    #[default]
    Linear,
    /// Log‐linear interpolation of positive values.
    LogLinear,
    /// Monotone‐convex interpolation of positive, non-increasing values.
    MonotoneConvex,
    /// Cubic Hermite interpolation (monotone-preserving slopes).
    CubicHermite,
    /// Piecewise‐flat forward‐rate interpolation (via log-linear).
    FlatFwd,
}

impl InterpStyle {
    /// Build a boxed interpolator implementing [`InterpFn`].
    pub fn build(
        self,
        knots: Box<[F]>,
        values: Box<[F]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Box<dyn InterpFn>> {
        match self {
            InterpStyle::Linear => Ok(Box::new(LinearDf::new(knots, values, extrapolation)?)),
            InterpStyle::LogLinear => Ok(Box::new(LogLinearDf::new(knots, values, extrapolation)?)),
            InterpStyle::MonotoneConvex => {
                Ok(Box::new(MonotoneConvex::new(knots, values, extrapolation)?))
            }
            InterpStyle::CubicHermite => {
                Ok(Box::new(CubicHermite::new(knots, values, extrapolation)?))
            }
            InterpStyle::FlatFwd => Ok(Box::new(FlatFwd::new(knots, values, extrapolation)?)),
        }
    }
}


