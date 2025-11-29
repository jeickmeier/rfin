//! Calibration test suite entry point.
//!
//! These tests verify calibration functionality including:
//! - JSON serialization roundtrips for all calibration types
//! - Hazard curve calibration with positivity validation
//! - CalibrationSpec pipeline construction and execution
//! - SABR swaption volatility surface calibration
//! - Repricing tolerance tests for calibrated curves
//! - Jacobian computation and explanation tracing

#[path = "calibration/mod.rs"]
mod calibration_tests;
