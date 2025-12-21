//! Market data tests.
//!
//! This module is organized by market data type:
//!
//! - [`curves`] - Term structure curves (discount, forward, hazard, inflation, base correlation)
//! - [`context`] - MarketContext container tests
//! - [`bumps`] - Bump infrastructure tests
//! - [`fx`] - FX provider tests
//! - [`scalars`] - Scalar time series and dividend tests
//! - [`credit_index`] - Credit index data tests
//! - [`serde`] - Cross-cutting serialization tests

// Test helpers shared across modules
mod test_helpers;

// Term structure curves
mod curves;

// Market context
mod context;

// Bump infrastructure
mod bumps;

// FX providers
mod fx;

// Scalar types (time series, dividends)
mod scalars;

// Credit index data
mod credit_index;

// Volatility surfaces
mod surfaces;

// Serialization tests
#[cfg(feature = "serde")]
mod serde;
