//! Models test suite entry point.
//!
//! This module consolidates tests for:
//! - Calibration: curve/surface calibration, repricing, serialization
//! - Term structures: curve properties, monotonicity, forward parity
//! - Market data: quote schemas, market building, quote bumps
//! - Pricer registry: instrument type parsing, model keys, batch pricing
//!
//! Run all models tests:
//! ```bash
//! cargo test --test models
//! ```

// ============================================================================
// Common Test Utilities
// ============================================================================

/// Shared fixtures and helpers used across the test suite
#[path = "models/common/mod.rs"]
mod common;

// ============================================================================
// Calibration Tests
// ============================================================================

/// Calibration tests - curve fitting, calibration specs, repricing
#[path = "models/calibration/mod.rs"]
mod calibration;

// ============================================================================
// Term Structure Tests
// ============================================================================

/// Term structure property tests - monotonicity, forward parity
#[path = "models/term_structures/mod.rs"]
mod term_structures;

// ============================================================================
// Market Data Tests
// ============================================================================

/// Market data tests - quote bumps and schema validation
#[path = "models/market/mod.rs"]
mod market;

// ============================================================================
// Pricer Registry Tests
// ============================================================================

/// Pricer registry tests - instrument types, model keys, batch pricing
#[path = "models/pricer/mod.rs"]
mod pricer;
