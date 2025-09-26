//! Market data utilities: curves, surfaces, and context aggregation.
//!
//! This module provides the core market data types used by pricing engines:
//! discount curves, forward curves, hazard curves, volatility surfaces, and
//! the MarketContext that aggregates them all.
//!
//! Key components:
//! - `term_structures`: yield curves, credit curves, inflation curves
//! - `surfaces`: volatility surfaces  
//! - `context`: MarketContext for aggregating market data
//! - `scalars`: market prices and time series
//! - `bumps`: scenario analysis and stress testing

/// Bump functionality for scenario analysis and stress testing.
pub mod bumps;
/// Market data context with enum-based storage (simplified from V2).
pub mod context;
/// Shared dividend schedules (cash/yield/stock) for equities/ETFs.
pub mod dividends;
/// Scalar market data types and time series (including primitives)
pub mod scalars;
/// Two-dimensional surfaces (e.g. volatility).
pub mod surfaces;
/// One-dimensional term structures (yield, credit, ...).
pub mod term_structures;
/// Public trait hierarchy used by pricing components.
pub mod traits;
// Re-export selected helpers for convenience at `market_data::*` level.
pub use crate::math::interp::utils::validate_knots;
// Re-export MarketContext at the top level for backward compatibility
pub use context::MarketContext;
// Re-export dividend schedule types for convenience
pub use dividends::*;

/// Numeric precision alias re-exported from the surrounding crate so that
/// downstream code can simply `use finstack_core::market_data::F`.
pub use crate::F;
