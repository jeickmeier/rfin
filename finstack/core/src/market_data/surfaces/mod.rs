//! Two-dimensional market data surfaces.
//!
//! Provides 2D interpolation structures for market observables that vary by
//! two parameters (e.g., volatility by strike and maturity). Currently supports
//! volatility surfaces with planned expansion for correlation and dividend surfaces.
//!
//! # Surface Types
//!
//! - `VolSurface`: Implied volatility by strike and maturity (bilinear interpolation)
//!
//! # Examples
//! ```rust
//! use finstack_core::market_data::surfaces::VolSurface;
//! use finstack_core::types::CurveId;
//! # fn main() -> finstack_core::Result<()> {
//!
//! let surface = VolSurface::builder("EQ-FLAT")
//!     .expiries(&[1.0, 2.0])
//!     .strikes(&[90.0, 100.0])
//!     .row(&[0.2, 0.2])
//!     .row(&[0.2, 0.2])
//!     .build()
//!     ?;
//! assert_eq!(surface.id(), &CurveId::from("EQ-FLAT"));
//! # Ok(())
//! # }
//! ```

mod delta_vol_surface;
pub mod fx_delta_vol_surface;
mod vol_surface;

// Re-export for ergonomic access (curated list)
pub use delta_vol_surface::FxDeltaVolSurfaceBuilder;
pub use fx_delta_vol_surface::FxDeltaVolSurface;
pub use vol_surface::{VolSurface, VolSurfaceAxis, VolSurfaceBuilder};
