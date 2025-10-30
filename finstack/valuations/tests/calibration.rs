//! Calibration integration tests

#[path = "calibration/test_calibration_serialization.rs"]
mod test_calibration_serialization;

#[path = "calibration/test_hazard_curve_calibration.rs"]
mod test_hazard_curve_calibration;

#[path = "calibration/test_simple_calibration_builder.rs"]
mod test_simple_calibration_builder;

#[path = "calibration/test_swaption_vol_calibration.rs"]
mod test_swaption_vol_calibration;

#[path = "calibration/test_calibration_repricing.rs"]
mod test_calibration_repricing;
