//! Determinism test suite for market standards compliance.
//!
//! These tests verify that the same inputs produce bitwise-identical outputs
//! across multiple runs. This is critical for reproducibility, regression testing,
//! and cross-platform consistency.
//!
//! All tests in this module should:
//! - Use fixed seeds for any randomness
//! - Verify bitwise identity (not just approximate equality)
//! - Run the same calculation multiple times (typically 10-100 iterations)
//! - Cover major instrument types and pricing paths

mod test_bond_pricing;
mod test_calibration;
mod test_cds_pricing;
mod test_option_pricing;
mod test_swap_pricing;
