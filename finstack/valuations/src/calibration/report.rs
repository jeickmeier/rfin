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
    /// Internal constructor.
    fn new() -> Self {
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

    /// Internal: Mark calibration as successful.
    fn success(mut self) -> Self {
        self.success = true;
        self
    }

    /// Internal: Set residuals and calculate metrics.
    fn with_residuals(mut self, residuals: BTreeMap<String, F>) -> Self {
        self.max_residual = residuals.values().map(|r| r.abs()).fold(0.0, f64::max);
        let sum_sq: F = residuals.values().map(|r| r * r).sum();
        self.rmse = if residuals.is_empty() {
            0.0
        } else {
            (sum_sq / residuals.len() as F).sqrt()
        };
        self.residuals = residuals;
        self
    }

    /// Internal: Set iteration count.
    fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Internal: Set convergence reason.
    fn with_convergence_reason(mut self, reason: impl Into<String>) -> Self {
        self.convergence_reason = reason.into();
        self
    }

    /// Add metadata to the report.
    ///
    /// # Example
    /// ```
    /// use finstack_valuations::calibration::CalibrationReport;
    /// use std::collections::BTreeMap;
    ///
    /// let residuals = BTreeMap::from([("quote1".to_string(), 0.001)]);
    /// let iterations = 10;
    /// let report = CalibrationReport::success_simple(residuals, iterations)
    ///     .with_metadata("currency", "USD");
    /// ```
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Create a simple successful calibration report (most common case).
    ///
    /// # Example
    /// ```
    /// use finstack_valuations::calibration::CalibrationReport;
    /// use std::collections::BTreeMap;
    ///
    /// let residuals = BTreeMap::from([("quote1".to_string(), 0.001)]);
    /// let iterations = 10;
    /// let report = CalibrationReport::success_simple(residuals, iterations);
    /// ```
    pub fn success_simple(residuals: BTreeMap<String, F>, iterations: usize) -> Self {
        Self::new()
            .success()
            .with_residuals(residuals)
            .with_iterations(iterations)
            .with_convergence_reason("Calibration completed successfully")
    }

    /// Create an empty successful report (no quotes to calibrate).
    ///
    /// # Example
    /// ```
    /// use finstack_valuations::calibration::CalibrationReport;
    ///
    /// let report = CalibrationReport::empty_success("No quotes provided");
    /// ```
    pub fn empty_success(reason: impl Into<String>) -> Self {
        Self::new()
            .success()
            .with_iterations(0)
            .with_convergence_reason(reason)
    }

    /// Create a simple failure report.
    ///
    /// # Example
    /// ```
    /// use finstack_valuations::calibration::CalibrationReport;
    ///
    /// let iterations = 100;
    /// let report = CalibrationReport::failure_simple("Convergence not achieved", iterations);
    /// ```
    pub fn failure_simple(reason: impl Into<String>, iterations: usize) -> Self {
        Self::new()
            .with_iterations(iterations)
            .with_convergence_reason(reason)
    }

    /// Builder method to set objective value.
    pub fn with_objective_value(mut self, value: F) -> Self {
        self.objective_value = value;
        self
    }

    /// Builder method to add multiple metadata entries at once.
    pub fn with_metadata_batch(
        mut self,
        entries: Vec<(impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (key, value) in entries {
            self.metadata.insert(key.into(), value.into());
        }
        self
    }

    /// Add a single residual to the report.
    pub fn push_residual(&mut self, key: impl Into<String>, value: F) {
        self.residuals.insert(key.into(), value);
        // Update derived metrics
        self.max_residual = self.residuals.values().map(|r| r.abs()).fold(0.0, f64::max);
        let sum_sq: F = self.residuals.values().map(|r| r * r).sum();
        self.rmse = if self.residuals.is_empty() {
            0.0
        } else {
            (sum_sq / self.residuals.len() as F).sqrt()
        };
    }

    /// Create a success report for a specific calibration type.
    ///
    /// # Example
    /// ```
    /// use finstack_valuations::calibration::CalibrationReport;
    /// use std::collections::BTreeMap;
    ///
    /// let residuals = BTreeMap::from([("quote1".to_string(), 0.001)]);
    /// let iterations = 10;
    /// let report = CalibrationReport::for_type("yield_curve", residuals, iterations);
    /// ```
    pub fn for_type(
        calibration_type: impl Into<String>,
        residuals: BTreeMap<String, F>,
        iterations: usize,
    ) -> Self {
        let type_str = calibration_type.into();
        let reason = format!("{} calibration completed", type_str.replace('_', " "));
        Self::new()
            .success()
            .with_residuals(residuals)
            .with_iterations(iterations)
            .with_convergence_reason(reason)
            .with_metadata("type", type_str)
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
