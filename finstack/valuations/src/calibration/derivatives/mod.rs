//! Analytical derivatives for common calibration objectives.
//!
//! This module provides analytical gradient and Jacobian implementations
//! for frequently used calibration objectives, significantly improving
//! convergence speed and accuracy compared to finite differences.

pub mod sabr_derivatives;
pub mod sabr_model_params;

pub use sabr_derivatives::SABRCalibrationDerivatives;
pub use sabr_model_params::SABRModelParams;
