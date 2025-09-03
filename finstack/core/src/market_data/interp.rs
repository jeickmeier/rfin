//! Generic interpolation framework used by term structures.
//!
//! At runtime *any* interpolator is referenced via the object-safe [`InterpFn`]
//! trait.  This makes it possible to store heterogeneous interpolation styles
//! behind a single `dyn` trait object (e.g. inside a [`Box`]).
//!
//! For ergonomics an enum [`InterpStyle`] acts as a factory so that builder
//! code can simply choose a style and obtain a ready-to-use interpolator via
//! [`InterpStyle::make_interp`].  The enum also documents all algorithms that
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
//! use finstack_core::market_data::interp::{InterpStyle, InterpFn};
//! let times = vec![0.0, 1.0, 2.0].into_boxed_slice();
//! let dfs   = vec![1.0, 0.97, 0.94].into_boxed_slice();
//! let interp = InterpStyle::MonotoneConvex
//!     .make_interp(times, dfs)
//!     .expect("valid input");
//! assert!(interp.interp(1.5) < 1.0);
//! ```

/// Internal helper trait implemented by each concrete interpolation struct.
/// Renamed from `Interpolator` to avoid a naming clash with the new enum of
/// the same name.
pub trait InterpFn: Send + Sync + core::fmt::Debug {
    /// Interpolate at coordinate `x`.
    fn interp(&self, x: crate::F) -> crate::F;
    
    /// Compute the first derivative at coordinate `x`.
    /// Essential for calculating hedge sensitivities (Delta, Rho, etc.).
    fn interp_prime(&self, x: crate::F) -> crate::F;
    
    /// Set the extrapolation policy for out-of-bounds evaluation.
    /// Default implementations should handle this appropriately.
    fn set_extrapolation_policy(&mut self, policy: ExtrapolationPolicy);
    
    /// Get the current extrapolation policy.
    fn extrapolation_policy(&self) -> ExtrapolationPolicy;
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

/// Shared trait implemented by builder types that can configure an
/// interpolation style. Provides zero-cost default helpers to select
/// styles while centralising the API surface.
pub trait InterpConfigurableBuilder: Sized {
    /// Set the interpolation style to use when building.
    fn set_interp(self, style: InterpStyle) -> Self;

    /// Use linear DF interpolation.
    fn linear_df(self) -> Self {
        self.set_interp(InterpStyle::Linear)
    }
    /// Use log-linear DF interpolation (constant zero rate between knots).
    fn log_df(self) -> Self {
        self.set_interp(InterpStyle::LogLinear)
    }
    /// Use Hagan–West monotone-convex interpolation.
    fn monotone_convex(self) -> Self {
        self.set_interp(InterpStyle::MonotoneConvex)
    }
    /// Use monotone cubic-Hermite interpolation (PCHIP).
    fn cubic_hermite(self) -> Self {
        self.set_interp(InterpStyle::CubicHermite)
    }
    /// Use piecewise flat-forward interpolation.
    fn flat_fwd(self) -> Self {
        self.set_interp(InterpStyle::FlatFwd)
    }
}

impl InterpStyle {
    /// Build a boxed interpolator implementing [`InterpFn`] for the given
    /// `knots` and `values`.
    pub fn make_interp(
        self,
        knots: Box<[crate::F]>,
        values: Box<[crate::F]>,
    ) -> crate::Result<Box<dyn InterpFn>> {
        self.make_interp_with_extrapolation(knots, values, ExtrapolationPolicy::default())
    }

    /// Build a boxed interpolator with specified extrapolation policy.
    pub fn make_interp_with_extrapolation(
        self,
        knots: Box<[crate::F]>,
        values: Box<[crate::F]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Box<dyn InterpFn>> {
        let mut interp: Box<dyn InterpFn> = match self {
            InterpStyle::Linear => Box::new(super::interp::LinearDf::new(knots, values)?),
            InterpStyle::LogLinear => Box::new(super::interp::LogLinearDf::new(knots, values)?),
            InterpStyle::MonotoneConvex => {
                Box::new(super::interp::MonotoneConvex::new(knots, values)?)
            }
            InterpStyle::CubicHermite => {
                Box::new(super::interp::CubicHermite::new(knots, values)?)
            }
            InterpStyle::FlatFwd => Box::new(super::interp::FlatFwd::new(knots, values)?),
        };
        interp.set_extrapolation_policy(extrapolation);
        Ok(interp)
    }
}
