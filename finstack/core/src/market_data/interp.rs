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

/// Enum of supported interpolation styles. The default is `Linear`.
#[derive(Copy, Clone, Debug, Default)]
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
    /// `knots` and `values`.
    pub fn make_interp(
        self,
        knots: Box<[crate::F]>,
        values: Box<[crate::F]>,
    ) -> crate::Result<Box<dyn InterpFn>> {
        match self {
            InterpStyle::Linear => Ok(Box::new(super::interp::LinearDf::new(knots, values)?)),
            InterpStyle::LogLinear => Ok(Box::new(super::interp::LogLinearDf::new(knots, values)?)),
            InterpStyle::MonotoneConvex => {
                Ok(Box::new(super::interp::MonotoneConvex::new(knots, values)?))
            }
            InterpStyle::CubicHermite => {
                Ok(Box::new(super::interp::CubicHermite::new(knots, values)?))
            }
            InterpStyle::FlatFwd => Ok(Box::new(super::interp::FlatFwd::new(knots, values)?)),
        }
    }
}
