//! Calibration reporting and diagnostics.

use crate::calibration::constants::RESIDUAL_PENALTY_ABS_MIN;
use crate::calibration::solver::SolverConfig;
use finstack_core::config::ResultsMeta;
use finstack_core::explain::ExplanationTrace;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn default_true() -> bool {
    true
}

/// Diagnostics computed from residual values.
///
/// Provides a statistical summary of the instrument fitting errors.
#[derive(Debug)]
struct ResidualDiagnostics {
    /// Maximum absolute residual across all instruments.
    max_residual: f64,
    /// Root mean square error of all residuals.
    rmse: f64,
    /// Whether any residual was a penalty value (solver failure).
    has_penalty: bool,
}

/// Filter out penalty sentinel values and compute common diagnostics.
///
/// Penalties (INFINITY or values >= [`PENALTY`](crate::calibration::PENALTY) * 0.5) are
/// excluded from max/RMSE unless ALL values are penalties.
///
/// # Arguments
/// * `residuals` - Map of instrument identifiers to their final residual values.
///
/// # Returns
/// A [`ResidualDiagnostics`] struct containing max, RMSE, and penalty status.
fn compute_residual_diagnostics(residuals: &BTreeMap<String, f64>) -> ResidualDiagnostics {
    let penalty_abs_min = RESIDUAL_PENALTY_ABS_MIN;

    // PERF: single pass, no allocation. Track both:
    // - "valid" residuals: finite and non-penalty
    // - fallbacks: all residuals (including penalties) if no valid exist
    let mut has_penalty = false;

    let mut max_abs_all = 0.0_f64;
    let mut sum_sq_all = 0.0_f64;
    let mut n_all = 0usize;

    let mut max_abs_valid = 0.0_f64;
    let mut sum_sq_valid = 0.0_f64;
    let mut n_valid = 0usize;

    for &r in residuals.values() {
        n_all += 1;
        let abs = r.abs();
        if abs > max_abs_all {
            max_abs_all = abs;
        }
        sum_sq_all += r * r;

        let is_penalty = !r.is_finite() || abs >= penalty_abs_min;
        has_penalty |= is_penalty;

        if !is_penalty {
            n_valid += 1;
            if abs > max_abs_valid {
                max_abs_valid = abs;
            }
            sum_sq_valid += r * r;
        }
    }

    let max_residual = if n_valid > 0 {
        max_abs_valid
    } else {
        max_abs_all
    };

    let rmse = if n_all == 0 {
        0.0
    } else if n_valid > 0 {
        (sum_sq_valid / n_valid as f64).sqrt()
    } else {
        (sum_sq_all / n_all as f64).sqrt()
    };

    ResidualDiagnostics {
        max_residual,
        rmse,
        has_penalty,
    }
}

/// Detailed report of a calibration exercise.
///
/// Consolidates success status, residuals, convergence diagnostics, and optional
/// tracing information. Used by the calibration engine to return results and
/// by risk systems to audit calibration quality.
///
/// # Examples
/// ```rust
/// use finstack_valuations::calibration::CalibrationReport;
/// use std::collections::BTreeMap;
///
/// let mut residuals = BTreeMap::new();
/// residuals.insert("1Y".to_string(), 1e-12);
///
/// let report = CalibrationReport::new(residuals, 10, true, "Converged");
/// assert!(report.success);
/// assert!(report.max_residual <= 1e-12);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationReport {
    /// User-facing success flag. True only if both fitting and validation passed.
    pub success: bool,
    /// Final residuals (fitting errors) by instrument identifier.
    pub residuals: BTreeMap<String, f64>,
    /// Number of solver iterations or function evaluations.
    pub iterations: usize,
    /// Final objective function value (usually RMSE).
    pub objective_value: f64,
    /// Maximum absolute residual across all instruments.
    pub max_residual: f64,
    /// Root mean square error of all residuals.
    pub rmse: f64,
    /// Whether the calibrated market object passed all validation checks.
    #[serde(default = "default_true")]
    pub validation_passed: bool,
    /// Optional details on validation failures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_error: Option<String>,
    /// Human-readable reason for convergence or failure.
    pub convergence_reason: String,
    /// Domain-specific metadata (e.g., "type": "yield_curve").
    pub metadata: BTreeMap<String, String>,
    /// Solver configuration used during this calibration run.
    #[serde(default)]
    pub solver_config: SolverConfig,
    /// Results metadata (timestamp, software version, etc.).
    #[serde(default)]
    pub results_meta: ResultsMeta,
    /// Optional detailed trace of the calibration steps (enabled via config).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<ExplanationTrace>,
    /// Optional model/methodology version used for this calibration.
    ///
    /// Used for audit trails and regulatory compliance. Examples:
    /// - "ISDA Standard Model v1.8.2" for CDS hazard curve calibration
    /// - "Multi-curve OIS discounting" for discount curve calibration
    /// - "SABR v1.0" for volatility surface calibration
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub model_version: Option<String>,
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
            model_version: None,
        }
    }

    /// Attach an explanation trace to this report.
    #[must_use]
    pub fn with_explanation(mut self, trace: ExplanationTrace) -> Self {
        self.explanation = Some(trace);
        self
    }

    /// Attach custom results metadata to this report.
    #[must_use]
    pub fn with_results_meta(mut self, meta: ResultsMeta) -> Self {
        self.results_meta = meta;
        self
    }

    /// Attach model/methodology version to this report.
    ///
    /// Used for audit trails and regulatory compliance. Examples:
    /// - "ISDA Standard Model v1.8.2" for CDS hazard curve calibration
    /// - "Multi-curve OIS discounting" for discount curve calibration
    #[must_use]
    pub fn with_model_version(mut self, version: impl Into<String>) -> Self {
        self.model_version = Some(version.into());
        self
    }

    /// Create a successful calibration report with no residuals.
    pub fn success_empty(reason: impl Into<String>) -> Self {
        Self::new(BTreeMap::new(), 0, true, reason)
    }

    /// Add metadata key-value pair to the report (builder pattern).
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Update metadata key-value pair on an existing report.
    pub fn update_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Set solver configuration (builder pattern).
    #[must_use]
    pub fn with_solver_config(mut self, config: SolverConfig) -> Self {
        self.solver_config = config;
        self
    }

    /// Attach validation outcome. If validation fails, the report is marked unsuccessful.
    #[must_use]
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
        self.metadata.insert(
            "solver_tolerance".to_string(),
            format!("{:.2e}", self.solver_config.tolerance()),
        );
        self.metadata.insert(
            "solver_max_iterations".to_string(),
            self.solver_config.max_iterations().to_string(),
        );
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
    /// ```rust,no_run
    /// use finstack_valuations::calibration::CalibrationReport;
    /// use std::collections::BTreeMap;
    ///
    /// # fn main() -> finstack_core::Result<()> {
    /// let residuals = BTreeMap::from([("DEP-1D".to_string(), 1e-6)]);
    /// let iterations = 5;
    /// let tolerance = 1e-4;
    ///
    /// let report = CalibrationReport::for_type_with_tolerance(
    ///     "yield_curve",
    ///     residuals,
    ///     iterations,
    ///     tolerance,
    /// );
    /// if !report.success {
    ///     return Err(finstack_core::Error::Calibration {
    ///         message: report.convergence_reason.clone(),
    ///         category: "calibration".to_string(),
    ///     });
    /// }
    /// # Ok(())
    /// # }
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
            .with_metadata("tolerance", format!("{:.2e}", tolerance))
            .with_metadata("success_tolerance", format!("{:.2e}", tolerance));
        }

        let diag = compute_residual_diagnostics(&residuals);

        // Determine success and convergence reason
        let (success, convergence_reason) = if diag.has_penalty {
            let penalty_abs_min = RESIDUAL_PENALTY_ABS_MIN;
            let penalty_instruments: Vec<&str> = residuals
                .iter()
                .filter(|(_, r)| !r.is_finite() || r.abs() >= penalty_abs_min)
                .map(|(k, _)| k.as_str())
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

        let tolerance_str = format!("{:.2e}", tolerance);

        Self::new(residuals, iterations, success, convergence_reason)
            .with_metadata("type", type_str)
            .with_metadata("tolerance", tolerance_str.clone())
            .with_metadata("success_tolerance", tolerance_str)
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
            model_version: None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::calibration::constants::PENALTY;

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
        let penalty = PENALTY;
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
        assert_eq!(
            report.metadata.get("success_tolerance"),
            deserialized.metadata.get("success_tolerance")
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
        assert_eq!(
            report.metadata.get("success_tolerance"),
            report.metadata.get("tolerance"),
            "success_tolerance should mirror tolerance metadata"
        );
    }
}
