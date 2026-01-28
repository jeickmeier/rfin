//! P&L attribution test suite entry point.
//!
//! This module consolidates tests for:
//!
//! - **bond_attribution**: Bond P&L attribution (carry, roll, spread, rate)
//! - **fx_attribution**: FX P&L attribution (spot, forward, basis)
//! - **metrics_based_convexity**: Convexity P&L attribution tests
//! - **model_params_attribution**: Model parameter change attribution
//! - **scalars_attribution**: Scalar market data attribution tests
//! - **serialization_roundtrip**: JSON roundtrip tests for attribution types
//!
//! Run all attribution tests:
//! ```bash
//! cargo test --test attribution
//! ```

#[path = "common/mod.rs"]
mod common;

#[path = "attribution/mod.rs"]
mod attribution;
