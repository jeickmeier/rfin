//! Metrics integration tests.
//!
//! Tests for risk metrics, Greeks, sensitivities, and their mathematical relationships.
//!
//! Test modules:
//! - `sign_conventions`: Market-standard sign validation for all Greeks
//! - `greek_relationships`: Mathematical relationships (Charm, Color, Speed)
//! - `edge_cases`: Extreme parameter handling
//! - `convergence`: Analytical vs FD convergence, bucket summation
//! - `determinism`: MC reproducibility (feature-gated)
//! - `graceful_metrics_test`: Error handling
//! - `invariants`: Property-based invariant tests (DV01 sum-to-parallel, MC determinism)

#[path = "metrics/graceful_metrics_test.rs"]
mod graceful_metrics_test;

#[path = "metrics/greek_relationships.rs"]
mod greek_relationships;

#[path = "metrics/sign_conventions.rs"]
mod sign_conventions;

#[path = "metrics/determinism.rs"]
mod determinism;

#[path = "metrics/edge_cases.rs"]
mod edge_cases;

#[path = "metrics/convergence.rs"]
mod convergence;

#[path = "metrics/invariants.rs"]
mod invariants;

// Note: tests/metrics/tests/ subdirectory contains internal unit tests that use
// crate-internal APIs (finite_difference, bump helpers). These run as part of
// `cargo test` within the library, not as integration tests.
