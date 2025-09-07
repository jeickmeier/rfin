//! Domain-agnostic interpolation framework (moved from market_data::interp).
//!
//! Provides common interpolation traits, policies, and implementations without
//! depending on market-data specific modules. Suitable for use across math and
//! pricing components.

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
/// Shared helpers (validation and search).
pub mod utils;
/// Traits for interpolation.
pub mod traits;
/// Types and factory for interpolation.
pub mod types;

// Re-exports for ergonomic access
pub use cubic_hermite::CubicHermite;
pub use flat_fwd::FlatFwd;
pub use linear::LinearDf;
pub use log_linear::LogLinearDf;
pub use monotone_convex::MonotoneConvex;
pub use traits::InterpFn;
pub use types::{ExtrapolationPolicy, InterpStyle, DERIVATIVE_EPSILON};
