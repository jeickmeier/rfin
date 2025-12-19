//! Calibration test suite.
//!
//! All tests in this module target the plan-driven calibration API
//! (`finstack_valuations::calibration`).

mod test_base_correlation_calibration;
mod test_bloomberg_calibration_accuracy;
mod test_calibration;
mod test_calibration_from_finstack_config;
mod test_calibration_repricing;
mod test_calibration_serialization;
mod test_explainability;
mod test_hazard_curve_calibration;
mod test_inflation_conventions;
mod test_parity_comprehensive;
mod test_simple_calibration_builder;
mod test_swaption_vol_calibration;
mod v2_parity_test;

mod tolerances;
