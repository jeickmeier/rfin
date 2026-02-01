//! Cashflows test suite entry point.
//!
//! This module consolidates tests for:
//!
//! - **builder/**: Cashflow schedule generation, amortization, credit models (PSA/SDA)
//! - **covenants/**: Covenant violation detection and enforcement
//! - **day_count**: ISDA 2006 day count convention golden values
//! - **provider_contract**: CashflowProvider trait contract compliance
//! - **schema_roundtrip**: JSON schema serialization roundtrip tests
//!
//! Run all cashflow tests:
//! ```bash
//! cargo test --test cashflows
//! ```

// ============================================================================
// Shared Test Utilities
// ============================================================================

/// Common test helpers: tolerance constants, date constructors, test curves
#[path = "cashflows/helpers.rs"]
mod helpers;

#[allow(clippy::expect_used, clippy::unwrap_used, dead_code, unused_imports)]
mod finstack_test_utils {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/test_utils.rs"
    ));
}

// ============================================================================
// Cashflow Builder Tests
// ============================================================================

/// Cashflow builder, schedule generation, amortization, and credit model tests
#[path = "cashflows/builder/mod.rs"]
mod builder;

// ============================================================================
// Day Count Convention Tests
// ============================================================================

/// ISDA 2006 day count convention golden values
#[path = "cashflows/day_count.rs"]
mod day_count;

// ============================================================================
// CashflowProvider Contract Tests
// ============================================================================

/// CashflowProvider trait contract compliance tests
#[path = "cashflows/provider_contract.rs"]
mod provider_contract;

// ============================================================================
// Schema Roundtrip Tests
// ============================================================================

/// JSON schema serialization roundtrip tests
#[path = "cashflows/schema_roundtrip.rs"]
mod schema_roundtrip;

// ============================================================================
// Covenant Tests
// ============================================================================

/// Covenant integration and enforcement tests
#[path = "cashflows/covenants/mod.rs"]
mod covenants;
