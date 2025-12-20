//! Integration test suite for Phase 1 market convention refactors.
//!
//! These tests verify end-to-end workflows including:
//! - Metrics strict mode (Phase 1.2)
//! - Strict metric parsing (Phase 1.3)
//! - Calibration residual normalization (Phase 1.4)
//!
//! Run with:
//! ```bash
//! cargo test --test integration_tests -- --nocapture
//! ```

#[path = "integration/mod.rs"]
mod integration;
