//! Scalar market data types and time series.
//!
//! This module contains market data that represents scalar values over time,
//! including basic scalar primitives and specialized implementations like
//! inflation indices.

/// Generic market primitives: scalars and ad-hoc time series.
pub mod primitives;

/// Inflation index data (CPI/RPI) using Polars DataFrames.
pub mod inflation_index;

// Re-export for ergonomic access
pub use primitives::*;
pub use inflation_index::*;
