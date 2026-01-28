//! Market data model test suite entry point.
//!
//! This module consolidates tests for:
//!
//! - **build/**: Instrument building from market quotes
//!   - `credit`: Credit instrument building from CDS quotes
//!   - `rates`: Rate instrument building from deposit/swap/FRA quotes
//! - **quote_bumps**: Tests for rate, spread, and vol bump operations
//! - **quote_schemas**: Schema validation and serialization tests for quote types
//!
//! Run all market tests:
//! ```bash
//! cargo test --test market
//! ```

// ============================================================================
// Shared Test Utilities
// ============================================================================

/// Common test utilities: fixtures, tolerances, assertions, builders
#[path = "common/mod.rs"]
mod common;

// ============================================================================
// Market Tests
// ============================================================================

#[path = "market/mod.rs"]
mod market;
