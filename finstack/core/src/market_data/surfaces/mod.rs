//! Two-dimensional market-data surfaces (e.g., implied volatility grids).
//!
//! Currently only [`vol_surface::VolSurface`] is exposed, but the structure is
//! intentionally modular so additional surface types (dividend yield, base
//! correlation) can be added without reshuffling the public namespace.
//!
//! # Examples
//! ```rust
//! use finstack_core::market_data::surfaces::VolSurface;
//! use finstack_core::types::CurveId;
//!
//! let surface = VolSurface::builder("EQ-FLAT")
//!     .expiries(&[1.0, 2.0])
//!     .strikes(&[90.0, 100.0])
//!     .row(&[0.2, 0.2])
//!     .row(&[0.2, 0.2])
//!     .build()
//!     .unwrap();
//! assert_eq!(surface.id(), &CurveId::from("EQ-FLAT"));
//! ```

pub mod vol_surface;

// Re-export for ergonomic access
pub use vol_surface::*;
