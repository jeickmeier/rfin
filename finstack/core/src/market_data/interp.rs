//! Generic interpolation framework used by term structures.
//!
//! At runtime *any* interpolator is referenced via the object-safe [`InterpFn`]
//! trait.  This makes it possible to store heterogeneous interpolation styles
//! behind a single `dyn` trait object (e.g. inside a [`Box`]).
//!
//! For ergonomics an enum [`InterpStyle`] acts as a factory so that builder
//! code can simply choose a style and obtain a ready-to-use interpolator via
//! [`InterpStyle::build`].  The enum also documents all algorithms that
//! ship with *rustfin-core*.
//!
//! ## When to pick which style?
//! | Style                | Properties                                            |
//! |----------------------|-------------------------------------------------------|
//! | `Linear`             | Fast, no shape guarantees; use for quick prototypes.  |
//! | `LogLinear`          | Constant zero-rate between knots; arbitrage friendly. |
//! | `MonotoneConvex`     | Hagan-West shape preserving; preferred for yields.    |
//! | `CubicHermite`       | Smooth C¹ spline; maintains monotonicity if data do.  |
//! | `FlatFwd`            | Piece-wise constant forward; common for swaps.        |
//!
//! ## Quick example
//! ```rust
//! use finstack_core::market_data::interp::{InterpStyle, InterpFn, ExtrapolationPolicy};
//! let times = vec![0.0, 1.0, 2.0].into_boxed_slice();
//! let dfs   = vec![1.0, 0.97, 0.94].into_boxed_slice();
//! let interp = InterpStyle::MonotoneConvex
//!     .build(times, dfs, ExtrapolationPolicy::FlatZero)
//!     .expect("valid input");
//! assert!(interp.interp(1.5) < 1.0);
//! ```

/// Epsilon for finite difference derivative calculations.
const DERIVATIVE_EPSILON: crate::F = 1e-6;

/// Internal helper trait implemented by each concrete interpolation struct.
/// Renamed from `Interpolator` to avoid a naming clash with the new enum of
/// the same name.
pub trait InterpFn: Send + Sync + core::fmt::Debug {
    /// Interpolate at coordinate `x`.
    fn interp(&self, x: crate::F) -> crate::F;

    /// Compute the first derivative at coordinate `x`.
    /// Essential for calculating hedge sensitivities (Delta, Rho, etc.).
    ///
    /// Default implementation uses central finite differences. Override for
    /// analytical derivatives when available.
    fn interp_prime(&self, x: crate::F) -> crate::F {
        let h = (x.abs() * DERIVATIVE_EPSILON).max(1e-8);
        (self.interp(x + h) - self.interp(x - h)) / (2.0 * h)
    }
}

// -----------------------------------------------------------------------------
// Concrete interpolator modules
// -----------------------------------------------------------------------------
/// Monotone cubic-Hermite interpolation (PCHIP).
pub mod cubic_hermite;
/// Piecewise‐flat instantaneous forward‐rate interpolation (log‐linear DF).
pub mod flat_fwd;
/// Simple piecewise‐linear interpolation on discount factors.
pub mod linear;
/// Linear interpolation in *log* discount factors (constant zero yields).
pub mod log_linear;
/// Hagan–West monotone‐convex cubic interpolation (shape preserving).
pub mod monotone_convex;

pub use cubic_hermite::CubicHermite;
pub use flat_fwd::FlatFwd;
pub use linear::LinearDf;
pub use log_linear::LogLinearDf;
pub use monotone_convex::MonotoneConvex;

extern crate alloc;
use alloc::boxed::Box;

/// Extrapolation policy for interpolators when evaluation points fall outside the knot range.
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ExtrapolationPolicy {
    /// Use constant value extrapolation (flat-zero): extend the endpoint values.
    /// This is the traditional approach for discount curves.
    #[default]
    FlatZero,
    /// Use flat-forward extrapolation: extend the forward rate from the last/first segment.
    /// This maintains constant instantaneous forward rates beyond the curve endpoints.
    FlatForward,
}

/// Enum of supported interpolation styles. The default is `Linear`.
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum InterpStyle {
    /// Linear interpolation in discount factors.
    #[default]
    Linear,
    /// Log‐linear interpolation in discount factors.
    LogLinear,
    /// Monotone‐convex interpolation of discount factors.
    MonotoneConvex,
    /// Cubic Hermite spline interpolation of discount factors.
    CubicHermite,
    /// Piecewise‐flat forward‐rate interpolation.
    FlatFwd,
}

impl InterpStyle {
    /// Build a boxed interpolator implementing [`InterpFn`] for the given
    /// `knots` and `values` with the specified extrapolation policy.
    pub fn build(
        self,
        knots: Box<[crate::F]>,
        values: Box<[crate::F]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Box<dyn InterpFn>> {
        match self {
            InterpStyle::Linear => Ok(Box::new(super::interp::LinearDf::new(
                knots,
                values,
                extrapolation,
            )?)),
            InterpStyle::LogLinear => Ok(Box::new(super::interp::LogLinearDf::new(
                knots,
                values,
                extrapolation,
            )?)),
            InterpStyle::MonotoneConvex => Ok(Box::new(super::interp::MonotoneConvex::new(
                knots,
                values,
                extrapolation,
            )?)),
            InterpStyle::CubicHermite => Ok(Box::new(super::interp::CubicHermite::new(
                knots,
                values,
                extrapolation,
            )?)),
            InterpStyle::FlatFwd => Ok(Box::new(super::interp::FlatFwd::new(
                knots,
                values,
                extrapolation,
            )?)),
        }
    }
}
