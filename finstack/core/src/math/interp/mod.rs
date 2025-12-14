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
//! - [`MonotoneConvex`]: Hagan-West smooth, monotone, no-arbitrage
//! - [`CubicHermite`]: PCHIP shape-preserving cubic
//! - [`PiecewiseQuadraticForward`]: Natural cubic in log DF (C² forwards)
//!
//! # Arbitrage Considerations
//!
//! Not all methods guarantee positive forward rates:
//! - **Linear**: May produce negative forwards ❌
//! - **LogLinear**: Guarantees positive forwards ✅
//! - **MonotoneConvex**: Guarantees positive forwards ✅
//! - **CubicHermite**: Shape-preserving but requires monotone input ⚠️
//!
//! # References
//!
//! - Hagan, P. S., & West, G. (2006). "Interpolation Methods for Curve Construction."
//!   *Applied Mathematical Finance*, 13(2), 89-129.
//! - Hagan, P. S., & West, G. (2008). "Methods for Constructing a Yield Curve."
//!   *Wilmott Magazine*, May 2008.

/// Generic interpolator container with strategy pattern.
pub mod generic;
/// Concrete strategy implementations.
pub mod strategies;
/// Traits for interpolation.
pub mod traits;
/// Types and factory for interpolation.
pub mod types;
/// Shared helpers (validation and search).
pub mod utils;
/// Public wrapper types for interpolators.
pub mod wrappers;

// Re-exports for ergonomic access
pub use generic::Interpolator;
pub use strategies::DEFAULT_MONOTONE_CONVEX_EPSILON;
pub use traits::{InterpFn, InterpolationStrategy};
pub use types::{ExtrapolationPolicy, InterpStyle, DERIVATIVE_EPSILON};
pub use wrappers::{
    CubicHermite, LinearDf, LogLinearDf, MonotoneConvex, PiecewiseQuadraticForward,
};
