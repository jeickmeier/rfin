//! Attribution test suite entry point.
//!
//! These tests verify P&L attribution functionality including:
//! - Parallel attribution (independent factor isolation)
//! - Waterfall attribution (sequential factor application)
//! - Metrics-based attribution (linear approximation)
//! - Model parameters attribution (prepayment, default, recovery)
//! - Serialization roundtrips for all attribution types

#[path = "attribution/mod.rs"]
mod attribution_tests;
