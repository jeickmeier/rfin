//! Risk test suite entry point.
//!
//! This module consolidates tests for:
//! - Metrics: Greeks, sensitivities, DV01, sign conventions
//! - Attribution: P&L attribution, parallel/waterfall decomposition
//!
//! Run all risk tests:
//! ```bash
//! cargo test --test risk
//! ```

// ============================================================================
// Common Test Utilities
// ============================================================================

/// Shared test utilities: assertions, builders, tolerances
#[path = "risk/common/mod.rs"]
pub mod common;

// ============================================================================
// Metrics Tests
// ============================================================================

/// Graceful metrics error handling tests
#[path = "risk/metrics/graceful_metrics_test.rs"]
mod graceful_metrics_test;

/// Greek mathematical relationships tests
#[path = "risk/metrics/greek_relationships.rs"]
mod greek_relationships;

/// Sign conventions tests for all Greeks
#[path = "risk/metrics/sign_conventions.rs"]
mod sign_conventions;

/// Determinism tests (MC reproducibility)
#[path = "risk/metrics/determinism.rs"]
mod determinism;

/// Edge cases for metrics computation
#[path = "risk/metrics/edge_cases.rs"]
mod edge_cases;

/// Convergence tests (analytical vs FD, bucket summation)
#[path = "risk/metrics/convergence.rs"]
mod convergence;

/// Property-based invariant tests (DV01 sum-to-parallel, MC determinism)
#[path = "risk/metrics/invariants.rs"]
mod invariants;

// Note: vanna_volga_pockets.rs uses pub(crate) internal APIs and is tested
// within the library via `cargo test` (not as integration test)

// ============================================================================
// Attribution Tests
// ============================================================================

/// P&L attribution tests - parallel, waterfall, metrics-based
#[path = "risk/attribution/mod.rs"]
mod attribution;
