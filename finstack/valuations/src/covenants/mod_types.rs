use serde::{Deserialize, Serialize};

/// Covenant check result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantReport {
    /// Type of covenant being checked
    pub covenant_type: String,

    /// Whether the covenant passed
    pub passed: bool,

    /// Actual value of the metric
    pub actual_value: Option<f64>,

    /// Required threshold
    pub threshold: Option<f64>,

    /// Details or explanation
    pub details: Option<String>,
}

impl CovenantReport {
    /// Create a passing covenant report.
    pub fn passed(covenant_type: &str) -> Self {
        Self {
            covenant_type: covenant_type.to_string(),
            passed: true,
            actual_value: None,
            threshold: None,
            details: None,
        }
    }

    /// Create a failing covenant report.
    pub fn failed(covenant_type: &str) -> Self {
        Self {
            covenant_type: covenant_type.to_string(),
            passed: false,
            actual_value: None,
            threshold: None,
            details: None,
        }
    }

    /// Add actual value to the report.
    pub fn with_actual(mut self, value: f64) -> Self {
        self.actual_value = Some(value);
        self
    }

    /// Add threshold to the report.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self
    }

    /// Add details to the report.
    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}
