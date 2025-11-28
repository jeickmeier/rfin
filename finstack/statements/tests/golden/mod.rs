//! Golden tests for serialization stability and external parity.
//!
//! This module contains tests that verify:
//! - Wire format stability (serialization doesn't change)
//! - End-to-end correctness (evaluation produces consistent results)
//! - Parity with external tools (Excel, pandas, QuantLib)

mod golden_tests;
mod golden_parity;

