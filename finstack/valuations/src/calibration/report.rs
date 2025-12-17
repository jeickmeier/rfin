//! Calibration reporting and diagnostics.

use crate::calibration::solver::SolverConfig;
use finstack_core::config::ResultsMeta;
use finstack_core::explain::ExplanationTrace;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn default_true() -> bool {
    true
}

/// Diagnostics computed from residual values.
struct ResidualDiagnostics {
    max_residual: f64,
    rmse: f64,
    has_penalty: bool,
}

/// Filter out penalty sentinel values and compute common diagnostics.
///
/// Penalties (INFINITY or values >= PENALTY * 0.5) are excluded from max/RMSE
/// unless ALL values are penalties, in which case the raw stats are used.
fn compute_residual_diagnostics(residuals: &BTreeMap<String, f64>) -> ResidualDiagnostics {
    let penalty_abs_min = crate::calibration::RESIDUAL_PENALTY_ABS_MIN;

    // Filter to finite non-penalty values
    let finite_vals: Vec<f64> = residuals
        .values()
        .copied()
        .filter(|r| r.is_finite() && r.abs() < penalty_abs_min)
        .collect();

    let has_penalty = residuals
        .values()
        .any(|r| !r.is_finite() || r.abs() >= penalty_abs_min);

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

    ResidualDiagnostics {
        max_residual,
        rmse,
        has_penalty,
    }
}

/// Calibration diagnostic report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationReport {
    /// Calibration success flag
    pub success: bool,
    /// Final residuals by instrument
    pub residuals: BTreeMap<String, f64>,
    /// Number of solver iterations (e.g., accepted LM steps)
    pub iterations: usize,
    /// Final objective function value
    pub objective_value: f64,
    /// Maximum absolute residual
    pub max_residual: f64,
    /// Root mean square error
    pub rmse: f64,
    /// Whether the calibrated market object passed validation checks (no-arbitrage, bounds, etc).
    ///
    /// This flag is independent from the solver residual tolerance checks.
    /// Market-standard workflows typically require both:
    /// - fit criteria met, and
    /// - validation/no-arbitrage checks passed.
    ///
    /// When `validation_passed == false`, `success` should generally be `false` even if residuals
    /// are within tolerance.
    #[serde(default = "default_true")]
    pub validation_passed: bool,
    /// Optional validation failure details (set when `validation_passed == false`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_error: Option<String>,
    /// Convergence reason
    pub convergence_reason: String,
    /// Calibration metadata (key-value pairs for domain-specific info)
    pub metadata: BTreeMap<String, String>,
    /// Solver configuration used for calibration.
    ///
    /// Captures the complete solver state for reproducibility. Defaults to
    /// Hybrid solver if not explicitly set.
    #[serde(default)]
    pub solver_config: SolverConfig,
    /// Result metadata (timestamp, version, rounding context, etc.)
    #[serde(default)]
    pub results_meta: ResultsMeta,
    /// Optional explanation trace (enabled via CalibrationConfig.explain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<ExplanationTrace>,
}

impl CalibrationReport {
    /// Convenience constructor covering the common case of a completed calibration.
    pub fn new(
        residuals: BTreeMap<String, f64>,
        iterations: usize,
        success: bool,
        convergence_reason: impl Into<String>,
    ) -> Self {
        let diag = compute_residual_diagnostics(&residuals);

        // Create default results metadata with stamping
        let results_meta =
            finstack_core::config::results_meta(&finstack_core::config::FinstackConfig::default());

        Self {
            success,
            residuals,
            iterations,
            // Default objective value is RMSE of residuals (penalty residuals excluded).
            // This is a generic, comparable scalar objective across calibrators. Individual
            // calibrators may overwrite this with a domain-specific objective via
            // `with_metadata(...)` or a future explicit objective setter.
            objective_value: diag.rmse,
            max_residual: diag.max_residual,
            rmse: diag.rmse,
            validation_passed: true,
            validation_error: None,
            convergence_reason: convergence_reason.into(),
            metadata: BTreeMap::new(),
            solver_config: SolverConfig::default(),
            results_meta,
            explanation: None,
        }
    }

    /// Attach an explanation trace to this report.
    pub fn with_explanation(mut self, trace: ExplanationTrace) -> Self {
        self.explanation = Some(trace);
        self
    }

    /// Attach custom results metadata to this report.
    pub fn with_results_meta(mut self, meta: ResultsMeta) -> Self {
        self.results_meta = meta;
        self
    }

    /// Create a successful calibration report with no residuals.
    pub fn success_empty(reason: impl Into<String>) -> Self {
        Self::new(BTreeMap::new(), 0, true, reason)
    }

    /// Add metadata key-value pair to the report (builder pattern).
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Update metadata key-value pair on an existing report.
    pub fn update_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Set solver configuration (builder pattern).
    pub fn with_solver_config(mut self, config: SolverConfig) -> Self {
        self.solver_config = config;
        self
    }

    /// Attach validation outcome. If validation fails, the report is marked unsuccessful.
    pub fn with_validation_result(mut self, passed: bool, error: Option<String>) -> Self {
        self.validation_passed = passed;
        self.validation_error = error;

        if !self.validation_passed {
            self.success = false;
            if let Some(err) = &self.validation_error {
                if self.convergence_reason.contains("converged") {
                    self.convergence_reason =
                        format!("Converged to tolerance but failed validation: {err}");
                } else if !self.convergence_reason.contains("validation failed") {
                    self.convergence_reason =
                        format!("{}; validation failed: {err}", self.convergence_reason);
                }
            } else if self.convergence_reason.contains("converged") {
                self.convergence_reason =
                    "Converged to tolerance but failed validation".to_string();
            } else if !self.convergence_reason.contains("validation failed") {
                self.convergence_reason = format!("{}; validation failed", self.convergence_reason);
            }
        }

        self
    }

    /// Update solver configuration on an existing report.
    pub fn update_solver_config(&mut self, config: SolverConfig) {
        self.solver_config = config;
    }

    /// Create a calibration report for a specific calibration type with tolerance checking.
    ///
    /// This method properly determines success/failure based on:
    /// - Whether any residuals contain PENALTY values (indicating hard failures)
    /// - Whether max_residual exceeds the configured tolerance
    ///
    /// # Arguments
    /// * `calibration_type` - Type identifier for the calibration (e.g., "yield_curve")
    /// * `residuals` - Map of instrument labels to their calibration residuals
    /// * `iterations` - Number of solver iterations performed
    /// * `tolerance` - Configured tolerance threshold for success determination
    ///
    /// # Example
    /// ```ignore
    /// let report = CalibrationReport::for_type_with_tolerance(
    ///     "yield_curve",
    ///     residuals,
    ///     iterations,
    ///     config.tolerance,
    /// );
    /// if !report.success {
    ///     return Err(Error::Calibration { ... });
    /// }
    /// ```
    pub fn for_type_with_tolerance(
        calibration_type: impl Into<String>,
        residuals: BTreeMap<String, f64>,
        iterations: usize,
        tolerance: f64,
    ) -> Self {
        let type_str = calibration_type.into();

        if residuals.is_empty() {
            return Self::new(
                residuals,
                iterations,
                false,
                format!(
                    "{} calibration failed: no residuals were produced",
                    type_str.replace('_', " ")
                ),
            )
            .with_metadata("type", type_str)
            .with_metadata("tolerance", format!("{:.2e}", tolerance));
        }

        let diag = compute_residual_diagnostics(&residuals);

        // Determine success and convergence reason
        let (success, convergence_reason) = if diag.has_penalty {
            let penalty_abs_min = crate::calibration::RESIDUAL_PENALTY_ABS_MIN;
            let penalty_instruments: Vec<&String> = residuals
                .iter()
                .filter(|(_, r)| !r.is_finite() || r.abs() >= penalty_abs_min)
                .map(|(k, _)| k)
                .collect();
            (
                false,
                format!(
                    "{} calibration failed: penalty values detected for instruments: {:?}",
                    type_str.replace('_', " "),
                    penalty_instruments
                ),
            )
        } else if diag.max_residual > tolerance {
            (
                false,
                format!(
                    "{} calibration failed: max_residual ({:.2e}) exceeds tolerance ({:.2e})",
                    type_str.replace('_', " "),
                    diag.max_residual,
                    tolerance
                ),
            )
        } else {
            (
                true,
                format!(
                    "{} calibration converged: max_residual ({:.2e}) within tolerance ({:.2e})",
                    type_str.replace('_', " "),
                    diag.max_residual,
                    tolerance
                ),
            )
        };

        Self::new(residuals, iterations, success, convergence_reason)
            .with_metadata("type", type_str)
            .with_metadata("tolerance", format!("{:.2e}", tolerance))
    }
}

impl Default for CalibrationReport {
    fn default() -> Self {
        let results_meta =
            finstack_core::config::results_meta(&finstack_core::config::FinstackConfig::default());

        Self {
            success: false,
            residuals: BTreeMap::new(),
            iterations: 0,
            objective_value: f64::INFINITY,
            max_residual: f64::INFINITY,
            rmse: f64::INFINITY,
            validation_passed: false,
            validation_error: None,
            convergence_reason: "Not started".to_string(),
            metadata: BTreeMap::new(),
            solver_config: SolverConfig::default(),
            results_meta,
            explanation: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_type_with_tolerance_success() {
        // All residuals within tolerance
        let mut residuals = BTreeMap::new();
        residuals.insert("quote_1Y".to_string(), 1e-10);
        residuals.insert("quote_2Y".to_string(), 5e-11);
        residuals.insert("quote_5Y".to_string(), 2e-10);

        let report = CalibrationReport::for_type_with_tolerance("yield_curve", residuals, 10, 1e-8);

        assert!(
            report.success,
            "Should succeed when all residuals within tolerance"
        );
        assert!(
            report.convergence_reason.contains("converged"),
            "Reason should indicate convergence: {}",
            report.convergence_reason
        );
        assert!(
            report.max_residual < 1e-8,
            "Max residual should be computed correctly"
        );
    }

    #[test]
    fn test_for_type_with_tolerance_fails_exceeds_tolerance() {
        // One residual exceeds tolerance
        let mut residuals = BTreeMap::new();
        residuals.insert("quote_1Y".to_string(), 1e-10);
        residuals.insert("quote_2Y".to_string(), 1e-6); // Exceeds 1e-8 tolerance
        residuals.insert("quote_5Y".to_string(), 2e-10);

        let report = CalibrationReport::for_type_with_tolerance("yield_curve", residuals, 10, 1e-8);

        assert!(
            !report.success,
            "Should fail when residual exceeds tolerance"
        );
        assert!(
            report.convergence_reason.contains("failed"),
            "Reason should indicate failure: {}",
            report.convergence_reason
        );
        assert!(
            report.convergence_reason.contains("exceeds tolerance"),
            "Reason should explain the tolerance breach: {}",
            report.convergence_reason
        );
    }

    #[test]
    fn test_for_type_with_tolerance_fails_penalty_values() {
        // One residual contains PENALTY value indicating solver failure
        let penalty = crate::calibration::PENALTY;
        let mut residuals = BTreeMap::new();
        residuals.insert("quote_1Y".to_string(), 1e-10);
        residuals.insert("quote_2Y_failed".to_string(), penalty); // PENALTY value
        residuals.insert("quote_5Y".to_string(), 2e-10);

        let report = CalibrationReport::for_type_with_tolerance("yield_curve", residuals, 10, 1e-8);

        assert!(!report.success, "Should fail when PENALTY value present");
        assert!(
            report.convergence_reason.contains("failed"),
            "Reason should indicate failure: {}",
            report.convergence_reason
        );
        assert!(
            report.convergence_reason.contains("penalty"),
            "Reason should mention penalty values: {}",
            report.convergence_reason
        );
        assert!(
            report.convergence_reason.contains("quote_2Y_failed"),
            "Reason should identify the failing instrument: {}",
            report.convergence_reason
        );
    }

    #[test]
    fn test_for_type_with_tolerance_fails_non_finite() {
        // Non-finite residual (infinity)
        let mut residuals = BTreeMap::new();
        residuals.insert("quote_1Y".to_string(), 1e-10);
        residuals.insert("quote_2Y_inf".to_string(), f64::INFINITY);
        residuals.insert("quote_5Y".to_string(), 2e-10);

        let report = CalibrationReport::for_type_with_tolerance("yield_curve", residuals, 10, 1e-8);

        assert!(!report.success, "Should fail when infinity present");
        assert!(
            report.convergence_reason.contains("penalty"),
            "Non-finite values should be treated as penalty failures: {}",
            report.convergence_reason
        );
    }

    #[test]
    fn test_for_type_with_tolerance_serialization() {
        let mut residuals = BTreeMap::new();
        residuals.insert("quote_1Y".to_string(), 1e-10);

        let report = CalibrationReport::for_type_with_tolerance("yield_curve", residuals, 10, 1e-8);

        // Test JSON round-trip
        let json = serde_json::to_string(&report).expect("Serialization should succeed");
        let deserialized: CalibrationReport =
            serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(report.success, deserialized.success);
        assert_eq!(report.convergence_reason, deserialized.convergence_reason);
        assert_eq!(
            report.metadata.get("tolerance"),
            deserialized.metadata.get("tolerance")
        );
    }

    #[test]
    fn test_for_type_with_tolerance_metadata_includes_tolerance() {
        let residuals = BTreeMap::new();
        let tolerance = 1e-8;

        let report =
            CalibrationReport::for_type_with_tolerance("yield_curve", residuals, 0, tolerance);

        assert!(
            report.metadata.contains_key("tolerance"),
            "Metadata should include tolerance"
        );
        assert!(
            report.metadata.contains_key("type"),
            "Metadata should include type"
        );
        assert_eq!(
            report.metadata.get("type"),
            Some(&"yield_curve".to_string())
        );
    }
}
