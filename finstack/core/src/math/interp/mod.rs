//! Interpolation algorithms for yield curves and term structures.
//!
//! Provides multiple interpolation schemes for constructing smooth, arbitrage-free
//! curves from discrete market quotes. Implementations are optimized for financial
//! use cases with focus on monotonicity and positive forward rates.
//!
//! # Interpolation Methods
//!
//! - [`LinearDf`]: Linear in discount factors (simple but may create arbitrage)
//! - [`LogLinearDf`]: Linear in log(DF), constant zero rates
//! - [`FlatFwd`]: Piecewise-constant forward rates (no-arbitrage)
//! - [`MonotoneConvex`]: Hagan-West smooth, monotone, no-arbitrage
//! - [`CubicHermite`]: PCHIP shape-preserving cubic
//!
//! # Arbitrage Considerations
//!
//! Not all methods guarantee positive forward rates:
//! - **Linear**: May produce negative forwards ❌
//! - **LogLinear**: Guarantees positive forwards ✅
//! - **FlatFwd**: Guarantees positive forwards ✅
//! - **MonotoneConvex**: Guarantees positive forwards ✅
//! - **CubicHermite**: Shape-preserving but requires monotone input ⚠️
//!
//! # References
//!
//! - Hagan, P. S., & West, G. (2006). "Interpolation Methods for Curve Construction."
//!   *Applied Mathematical Finance*, 13(2), 89-129.
//! - Hagan, P. S., & West, G. (2008). "Methods for Constructing a Yield Curve."
//!   *Wilmott Magazine*, May 2008.

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
/// Traits for interpolation.
pub mod traits;
/// Types and factory for interpolation.
pub mod types;
/// Shared helpers (validation and search).
pub mod utils;

// Re-exports for ergonomic access
pub use cubic_hermite::CubicHermite;
pub use flat_fwd::FlatFwd;
pub use linear::LinearDf;
pub use log_linear::LogLinearDf;
pub use monotone_convex::MonotoneConvex;
pub use traits::InterpFn;
pub use types::{ExtrapolationPolicy, InterpStyle, DERIVATIVE_EPSILON};
