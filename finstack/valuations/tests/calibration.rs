//! Calibration test suite entry point.
//!
//! This module consolidates tests for:
//!
//! - **bootstrap**: Curve bootstrapping determinism and smoke tests
//! - **repricing**: Repricing accuracy for calibrated curves
//! - **config**: Configuration helpers and validation rules
//! - **finstack_config**: Finstack-specific config integration
//! - **serialization**: Serde roundtrip tests for calibration types
//! - **builder**: Simple calibration builder API tests
//! - **hazard_curve**: Hazard/credit curve calibration
//! - **inflation**: Inflation curve calibration and conventions
//! - **swaption_vol**: Swaption volatility surface calibration
//! - **base_correlation**: Base correlation surface calibration
//! - **failure_modes**: Engine error handling and failure scenarios
//! - **explainability**: Explanation trace generation
//! - **validation**: Curve and surface validation tests
//! - **parity_comprehensive**: All-types calibration parity verification
//! - **bloomberg_accuracy**: Bloomberg benchmark accuracy tests
//! - **v2_parity**: V2 API parity tests
//! - **term_structures/**: Independent term structure property tests
//!
//! Run all calibration tests:
//! ```bash
//! cargo test --test calibration
//! ```

// ============================================================================
// Shared Test Utilities
// ============================================================================

/// Common test utilities: fixtures, tolerances, assertions, builders
#[path = "common/mod.rs"]
mod common;

// ============================================================================
// Calibration Tests
// ============================================================================

#[path = "calibration/mod.rs"]
mod calibration;
