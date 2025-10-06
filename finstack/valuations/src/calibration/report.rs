//! Calibration reporting and diagnostics.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Calibration diagnostic report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationReport {
    /// Calibration success flag
    pub success: bool,
    /// Final residuals by instrument
    pub residuals: BTreeMap<String, f64>,
    /// Number of iterations taken
    pub iterations: usize,
    /// Final objective function value
    pub objective_value: f64,
    /// Maximum absolute residual
    pub max_residual: f64,
    /// Root mean square error
    pub rmse: f64,
    /// Convergence reason
    pub convergence_reason: String,
    /// Calibration metadata
    pub metadata: BTreeMap<String, String>,
}

impl CalibrationReport {
    /// Convenience constructor covering the common case of a completed calibration.
    pub fn new(
        residuals: BTreeMap<String, f64>,
        iterations: usize,
        success: bool,
        convergence_reason: impl Into<String>,
    ) -> Self {
        // Ignore sentinel penalty values when computing diagnostics, so a single
        // hard failure doesn't drown out useful residual magnitudes. If all
        // residuals are penalties, we fall back to the raw max.
        let penalty = crate::calibration::PENALTY;
        let finite_vals: Vec<f64> = residuals
            .values()
            .copied()
            .filter(|r| r.is_finite() && r.abs() < penalty * 0.5)
            .collect();
        let max_residual = if finite_vals.is_empty() {
            residuals.values().map(|r| r.abs()).fold(0.0, f64::max)
        } else {
            finite_vals.iter().map(|r| r.abs()).fold(0.0, f64::max)
        };
        let rmse = if residuals.is_empty() {
            0.0
        } else if finite_vals.is_empty() {
            let sum_sq: f64 = residuals.values().map(|r| r * r).sum();
            (sum_sq / residuals.len() as f64).sqrt()
        } else {
            let sum_sq: f64 = finite_vals.iter().map(|r| r * r).sum();
            (sum_sq / finite_vals.len() as f64).sqrt()
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
        residuals: BTreeMap<String, f64>,
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
            objective_value: f64::INFINITY,
            max_residual: f64::INFINITY,
            rmse: f64::INFINITY,
            convergence_reason: "Not started".to_string(),
            metadata: BTreeMap::new(),
        }
    }
}
