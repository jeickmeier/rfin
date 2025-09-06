//! Domain-agnostic interpolation framework (moved from market_data::interp).
//!
//! Provides common interpolation traits, policies, and implementations without
//! depending on market-data specific modules. Suitable for use across math and
//! pricing components.

/// Shared helpers (validation and search).
pub mod utils;
/// Monotone cubic-Hermite interpolation (PCHIP / Fritsch-Carlson).
pub mod cubic_hermite;
/// Piecewise-flat instantaneous forward-rate interpolation (log-linear DF).
pub mod flat_fwd;
/// Simple piecewise-linear interpolation on positive values.
pub mod linear;
/// Linear interpolation in log(values) (constant zero-yield behaviour for DFs).
pub mod log_linear;
/// Hagan–West monotone-convex cubic interpolation in log-space.
pub mod monotone_convex;

/// Epsilon for finite difference derivative calculations.
const DERIVATIVE_EPSILON: crate::F = 1e-6;

/// Object-safe interpolation trait.
pub trait InterpFn: Send + Sync + core::fmt::Debug {
    /// Interpolate at coordinate `x`.
    fn interp(&self, x: crate::F) -> crate::F;

    /// First derivative at `x`. Default via central finite differences.
    fn interp_prime(&self, x: crate::F) -> crate::F {
        let h = (x.abs() * DERIVATIVE_EPSILON).max(1e-8);
        (self.interp(x + h) - self.interp(x - h)) / (2.0 * h)
    }
}

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

// Re-exports for ergonomic access
pub use cubic_hermite::CubicHermite;
pub use flat_fwd::FlatFwd;
pub use linear::LinearDf;
pub use log_linear::LogLinearDf;
pub use monotone_convex::MonotoneConvex;

extern crate alloc;
use alloc::boxed::Box;

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
        knots: Box<[crate::F]>,
        values: Box<[crate::F]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Box<dyn InterpFn>> {
        match self {
            InterpStyle::Linear => Ok(Box::new(LinearDf::new(knots, values, extrapolation)?)),
            InterpStyle::LogLinear => Ok(Box::new(LogLinearDf::new(knots, values, extrapolation)?)),
            InterpStyle::MonotoneConvex => Ok(Box::new(MonotoneConvex::new(
                knots, values, extrapolation,
            )?)),
            InterpStyle::CubicHermite => Ok(Box::new(CubicHermite::new(
                knots, values, extrapolation,
            )?)),
            InterpStyle::FlatFwd => Ok(Box::new(FlatFwd::new(knots, values, extrapolation)?)),
        }
    }
}


