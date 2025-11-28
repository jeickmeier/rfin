//! Cashflow test suite entry point.
//!
//! These tests verify cashflow functionality including:
//! - Cashflow builder and schedule generation
//! - Period-based aggregation and PV calculations
//! - Amortization specification validation
//! - JSON schema roundtrip serialization

#[path = "cashflow/mod.rs"]
mod cashflow_tests;
