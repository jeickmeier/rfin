//! Two-dimensional *market-data surfaces* such as implied-volatility grids.
//!
//! Currently only [`vol_surface::VolSurface`] is provided but the module is
//! structured so additional surface types (e.g. dividend yield or correlation
//! surfaces) can be added without changing the public namespace.

pub mod vol_surface;

// Re-export for ergonomic access
pub use vol_surface::*;
