//! Market data module integration tests.
//!
//! This test suite verifies market data types and functionality:
//! - Term structure curves (discount, forward, hazard, inflation, base correlation)
//! - MarketContext container
//! - Bump infrastructure
//! - FX providers
//! - Scalar time series and dividends
//! - Credit index data
//!
//! # Test Organization
//!
//! - `test_helpers`: Shared test utilities
//! - `curves/`: Term structure curve tests
//! - `context`: MarketContext container tests
//! - `bumps`: Bump infrastructure tests
//! - `fx`: FX provider tests
//! - `scalars`: Scalar time series and dividend tests
//! - `credit_index`: Credit index data tests
//! - `serde`: Cross-cutting serialization tests

mod market_data;
