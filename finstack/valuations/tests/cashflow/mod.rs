//! Cashflow integration tests.
//!
//! This module contains tests for:
//! - Cashflow builder and schedule generation
//! - Period-based aggregation and PV calculations
//! - Amortization specification validation
//! - JSON schema roundtrip serialization
//! - Day count convention golden values
//! - CashflowProvider trait contract compliance

mod amortization_spec;
mod builder_tests;
mod cashflow_schemas_examples;
mod day_count_golden_tests;
mod provider_contract;
pub mod test_helpers;
