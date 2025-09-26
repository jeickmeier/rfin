//! Calibration reporting and diagnostics.

use finstack_core::F;
use std::collections::BTreeMap;

/// Calibration diagnostic report.
#[derive(Clone, Debug)]
pub struct CalibrationReport {
    /// Calibration success flag
    pub success: bool,
    /// Final residuals by instrument
    pub residuals: BTreeMap<String, F>,
    /// Number of iterations taken
    pub iterations: usize,
    /// Final objective function value
    pub objective_value: F,
    /// Maximum absolute residual
    pub max_residual: F,
    /// Root mean square error
    pub rmse: F,
    /// Convergence reason
    pub convergence_reason: String,
    /// Calibration metadata
    pub metadata: BTreeMap<String, String>,
}

impl CalibrationReport {
    /// Convenience constructor covering the common case of a completed calibration.
    pub fn new(
        residuals: BTreeMap<String, F>,
        iterations: usize,
        success: bool,
        convergence_reason: impl Into<String>,
    ) -> Self {
        let max_residual = residuals.values().map(|r| r.abs()).fold(0.0, f64::max);
        let rmse = if residuals.is_empty() {
            0.0
        } else {
            let sum_sq: F = residuals.values().map(|r| r * r).sum();
            (sum_sq / residuals.len() as F).sqrt()
        };

        Self {
            success,
            residuals,
            iterations,
            objective_value: max_residual,
            max_residual,
            rmse,
            convergence_reason: convergence_reason.into(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn success_empty(reason: impl Into<String>) -> Self {
        Self::new(BTreeMap::new(), 0, true, reason)
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn update_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    pub fn for_type(
        calibration_type: impl Into<String>,
        residuals: BTreeMap<String, F>,
        iterations: usize,
    ) -> Self {
        let type_str = calibration_type.into();
        let reason = format!("{} calibration completed", type_str.replace('_', " "));
        Self::new(residuals, iterations, true, reason).with_metadata("type", type_str)
    }
}

impl Default for CalibrationReport {
    fn default() -> Self {
        Self {
            success: false,
            residuals: BTreeMap::new(),
            iterations: 0,
            objective_value: F::INFINITY,
            max_residual: F::INFINITY,
            rmse: F::INFINITY,
            convergence_reason: "Not started".to_string(),
            metadata: BTreeMap::new(),
        }
    }
}
