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

/// Crate-private enum enabling static dispatch for interpolation in hot loops.
///
/// Storing this enum (instead of `Box<dyn InterpFn>`) allows the compiler to
/// inline calls to `interp` and `interp_prime` for each concrete variant.
#[derive(Debug)]
pub(crate) enum Interp {
    Linear(LinearDf),
    LogLinear(LogLinearDf),
    MonotoneConvex(MonotoneConvex),
    CubicHermite(CubicHermite),
    FlatFwd(FlatFwd),
}

/// Serializable representation of Interp enum for persistence
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InterpData {
    Linear {
        knots: Vec<F>,
        values: Vec<F>,
        extrapolation: ExtrapolationPolicy,
    },
    LogLinear {
        knots: Vec<F>,
        values: Vec<F>,
        extrapolation: ExtrapolationPolicy,
    },
    MonotoneConvex {
        knots: Vec<F>,
        values: Vec<F>,
        extrapolation: ExtrapolationPolicy,
    },
    CubicHermite {
        knots: Vec<F>,
        values: Vec<F>,
        extrapolation: ExtrapolationPolicy,
    },
    FlatFwd {
        knots: Vec<F>,
        values: Vec<F>,
        extrapolation: ExtrapolationPolicy,
    },
}

impl Interp {
    #[inline]
    pub(crate) fn interp(&self, x: F) -> F {
        match self {
            Interp::Linear(i) => i.interp(x),
            Interp::LogLinear(i) => i.interp(x),
            Interp::MonotoneConvex(i) => i.interp(x),
            Interp::CubicHermite(i) => i.interp(x),
            Interp::FlatFwd(i) => i.interp(x),
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn interp_prime(&self, x: F) -> F {
        match self {
            Interp::Linear(i) => i.interp_prime(x),
            Interp::LogLinear(i) => i.interp_prime(x),
            Interp::MonotoneConvex(i) => i.interp_prime(x),
            Interp::CubicHermite(i) => i.interp_prime(x),
            Interp::FlatFwd(i) => i.interp_prime(x),
        }
    }

    /// Extract knots and values from the interpolator
    #[cfg(feature = "serde")]
    pub(crate) fn to_interp_data(&self) -> InterpData {
        match self {
            Interp::Linear(i) => InterpData::Linear {
                knots: i.knots().to_vec(),
                values: i.values().to_vec(),
                extrapolation: i.extrapolation(),
            },
            Interp::LogLinear(i) => InterpData::LogLinear {
                knots: i.knots().to_vec(),
                values: i.values(), // already returns Vec<F>
                extrapolation: i.extrapolation(),
            },
            Interp::MonotoneConvex(i) => InterpData::MonotoneConvex {
                knots: i.knots().to_vec(),
                values: i.values().to_vec(),
                extrapolation: i.extrapolation(),
            },
            Interp::CubicHermite(i) => InterpData::CubicHermite {
                knots: i.knots().to_vec(),
                values: i.values().to_vec(),
                extrapolation: i.extrapolation(),
            },
            Interp::FlatFwd(i) => InterpData::FlatFwd {
                knots: i.knots().to_vec(),
                values: i.values(), // already returns Vec<F>
                extrapolation: i.extrapolation(),
            },
        }
    }

    /// Build an Interp from serialized data
    #[cfg(feature = "serde")]
    pub(crate) fn from_interp_data(data: InterpData) -> crate::Result<Self> {
        match data {
            InterpData::Linear { knots, values, extrapolation } => {
                Ok(Interp::Linear(LinearDf::new(
                    knots.into_boxed_slice(),
                    values.into_boxed_slice(),
                    extrapolation,
                )?))
            }
            InterpData::LogLinear { knots, values, extrapolation } => {
                Ok(Interp::LogLinear(LogLinearDf::new(
                    knots.into_boxed_slice(),
                    values.into_boxed_slice(),
                    extrapolation,
                )?))
            }
            InterpData::MonotoneConvex { knots, values, extrapolation } => {
                Ok(Interp::MonotoneConvex(MonotoneConvex::new(
                    knots.into_boxed_slice(),
                    values.into_boxed_slice(),
                    extrapolation,
                )?))
            }
            InterpData::CubicHermite { knots, values, extrapolation } => {
                Ok(Interp::CubicHermite(CubicHermite::new(
                    knots.into_boxed_slice(),
                    values.into_boxed_slice(),
                    extrapolation,
                )?))
            }
            InterpData::FlatFwd { knots, values, extrapolation } => {
                Ok(Interp::FlatFwd(FlatFwd::new(
                    knots.into_boxed_slice(),
                    values.into_boxed_slice(),
                    extrapolation,
                )?))
            }
        }
    }
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

    /// Build an enum-backed interpolator enabling static dispatch.
    #[inline]
    pub(crate) fn build_enum(
        self,
        knots: Box<[F]>,
        values: Box<[F]>,
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
            InterpStyle::FlatFwd => Interp::FlatFwd(FlatFwd::new(knots, values, extrapolation)?),
        };
        Ok(interp)
    }
}
