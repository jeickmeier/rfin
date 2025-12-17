//! Calibration v2 test suite.
//!
//! All tests in this module target the plan-driven v2 calibration API
//! (`finstack_valuations::calibration::v2`).

mod test_base_correlation_calibration;
mod test_bloomberg_calibration_accuracy;
mod test_calibration;
mod test_calibration_from_finstack_config;
mod test_calibration_repricing;
mod test_calibration_serialization;
mod test_discount_curve_calibration;
mod test_discount_curve_time_axis;
mod test_explainability;
mod test_hazard_curve_calibration;
mod test_inflation_conventions;
mod test_rates_quote_schema;
mod test_simple_calibration_builder;
mod test_strict_pricing;
mod test_swaption_vol_calibration;
mod v2_parity_test;
