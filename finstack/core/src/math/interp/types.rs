//! Interpolation types and style enumeration.
//!
//! Defines the `InterpStyle` enum for selecting interpolation methods and
//! the internal `Interp` enum for static dispatch in hot paths.

use super::traits::InterpFn;
use super::wrappers::{
    CubicHermite, LinearDf, LogLinearDf, MonotoneConvex, PiecewiseQuadraticForward,
};

/// Epsilon for finite difference derivative calculations.
///
/// This is the default value used in [`InterpFn::interp_prime`] for numerical
/// derivatives. For more control, use [`InterpConfig`](crate::interp_config::InterpConfig)
/// and implement custom derivative logic.
pub const DERIVATIVE_EPSILON: f64 = 1e-6;

/// Extrapolation policy for evaluation outside the knot range.
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
pub enum ExtrapolationPolicy {
    /// Constant value extension.
    #[default]
    FlatZero,
    /// Tangent/forward extension using boundary slope.
    FlatForward,
}

/// Enum of supported interpolation styles. The default is `Linear`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[non_exhaustive]
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
    /// Piecewise quadratic forwards (smooth forward curve, C²).
    PiecewiseQuadraticForward,
}

/// Crate-private enum enabling static dispatch for interpolation in hot loops.
///
/// Storing this enum (instead of `Box<dyn InterpFn>`) allows the compiler to
/// inline calls to `interp` and `interp_prime` for each concrete variant.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub(crate) enum Interp {
    Linear(LinearDf),
    LogLinear(LogLinearDf),
    MonotoneConvex(MonotoneConvex),
    CubicHermite(CubicHermite),
    PiecewiseQuadraticForward(PiecewiseQuadraticForward),
}

impl Interp {
    #[inline]
    pub(crate) fn interp(&self, x: f64) -> f64 {
        match self {
            Interp::Linear(i) => i.interp(x),
            Interp::LogLinear(i) => i.interp(x),
            Interp::MonotoneConvex(i) => i.interp(x),
            Interp::CubicHermite(i) => i.interp(x),
            Interp::PiecewiseQuadraticForward(i) => i.interp(x),
        }
    }

    /// Get the interpolation style of this Interp
    #[cfg(feature = "serde")]
    pub(crate) fn style(&self) -> InterpStyle {
        match self {
            Interp::Linear(_) => InterpStyle::Linear,
            Interp::LogLinear(_) => InterpStyle::LogLinear,
            Interp::MonotoneConvex(_) => InterpStyle::MonotoneConvex,
            Interp::CubicHermite(_) => InterpStyle::CubicHermite,
            Interp::PiecewiseQuadraticForward(_) => InterpStyle::PiecewiseQuadraticForward,
        }
    }

    /// Get the extrapolation policy of this Interp
    #[cfg(feature = "serde")]
    pub fn extrapolation(&self) -> ExtrapolationPolicy {
        match self {
            Interp::Linear(i) => i.extrapolation(),
            Interp::LogLinear(i) => i.extrapolation(),
            Interp::MonotoneConvex(i) => i.extrapolation(),
            Interp::CubicHermite(i) => i.extrapolation(),
            Interp::PiecewiseQuadraticForward(i) => i.extrapolation(),
        }
    }
}

impl InterpStyle {
    /// Build a boxed interpolator implementing [`InterpFn`].
    pub fn build(
        self,
        knots: Box<[f64]>,
        values: Box<[f64]>,
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
            InterpStyle::PiecewiseQuadraticForward => Ok(Box::new(PiecewiseQuadraticForward::new(
                knots,
                values,
                extrapolation,
            )?)),
        }
    }

    /// Build an enum-backed interpolator enabling static dispatch.
    #[inline]
    pub(crate) fn build_enum(
        self,
        knots: Box<[f64]>,
        values: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Interp> {
        let interp = match self {
            InterpStyle::Linear => Interp::Linear(LinearDf::new(knots, values, extrapolation)?),
            InterpStyle::LogLinear => {
                Interp::LogLinear(LogLinearDf::new(knots, values, extrapolation)?)
            }
            InterpStyle::MonotoneConvex => {
                Interp::MonotoneConvex(MonotoneConvex::new(knots, values, extrapolation)?)
            }
            InterpStyle::CubicHermite => {
                Interp::CubicHermite(CubicHermite::new(knots, values, extrapolation)?)
            }
            InterpStyle::PiecewiseQuadraticForward => Interp::PiecewiseQuadraticForward(
                PiecewiseQuadraticForward::new(knots, values, extrapolation)?,
            ),
        };
        Ok(interp)
    }

    /// Build an enum-backed interpolator allowing any values (including negative).
    ///
    /// This is useful for forward rate curves where negative rates are allowed
    /// (e.g., EUR, CHF, JPY markets since 2014).
    ///
    /// **Note:** LogLinear and MonotoneConvex require positive values for mathematical
    /// reasons (log transform, monotonicity). Using them with negative values will fail.
    #[inline]
    pub(crate) fn build_enum_allow_any_values(
        self,
        knots: Box<[f64]>,
        values: Box<[f64]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Interp> {
        let interp =
            match self {
                InterpStyle::Linear => Interp::Linear(LinearDf::new_allow_any_values(
                    knots,
                    values,
                    extrapolation,
                )?),
                InterpStyle::LogLinear => {
                    // LogLinear requires positive values for log transform
                    Interp::LogLinear(LogLinearDf::new(knots, values, extrapolation)?)
                }
                InterpStyle::MonotoneConvex => {
                    // MonotoneConvex typically requires positive values
                    Interp::MonotoneConvex(MonotoneConvex::new(knots, values, extrapolation)?)
                }
                InterpStyle::CubicHermite => Interp::CubicHermite(
                    CubicHermite::new_allow_any_values(knots, values, extrapolation)?,
                ),
                InterpStyle::PiecewiseQuadraticForward => Interp::PiecewiseQuadraticForward(
                    PiecewiseQuadraticForward::new(knots, values, extrapolation)?,
                ),
            };
        Ok(interp)
    }
}
