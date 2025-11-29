//! Calibration integration tests
//!
//! Test modules:
//! - test_calibration_serialization: JSON roundtrip tests for all calibration types,
//!   including CalibrationEnvelope and MarketContextState
//! - test_hazard_curve_calibration: Hazard curve calibration with positivity checks
//! - test_simple_calibration_builder: CalibrationSpec pipeline construction and execution
//! - test_swaption_vol_calibration: SABR swaption volatility surface calibration
//! - test_calibration_repricing: Repricing tolerance tests for calibrated curves
//! - test_explainability: Jacobian computation and explanation tracing
//! - test_curve_monotonicity: Property-based tests for discount curve properties
//! - test_forward_parity: Property-based tests for forward rate parity

mod test_calibration;
mod test_calibration_repricing;
mod test_calibration_serialization;
mod test_curve_monotonicity;
mod test_explainability;
mod test_forward_parity;
mod test_hazard_curve_calibration;
mod test_simple_calibration_builder;
mod test_swaption_vol_calibration;
