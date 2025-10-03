//! Credit scorecard analysis extension (placeholder).
//!
//! This extension provides credit rating assignment based on financial metrics
//! and configurable thresholds.
//!
//! **Status:** Not yet implemented. This is a placeholder for future development.
//!
//! # Planned Features
//!
//! - Credit rating assignment based on financial metrics
//! - Configurable rating scales and thresholds
//! - Weighted scoring across multiple metrics
//! - Support for multiple rating agencies (S&P, Moody's, Fitch)
//!
//! # Configuration Schema
//!
//! ```json
//! {
//!   "rating_scale": "S&P",
//!   "metrics": [
//!     {
//!       "name": "debt_to_ebitda",
//!       "formula": "total_debt / ttm(ebitda)",
//!       "weight": 0.3,
//!       "thresholds": {
//!         "AAA": [0.0, 1.0],
//!         "AA": [1.0, 2.0],
//!         "A": [2.0, 3.0],
//!         "BBB": [3.0, 4.0],
//!         "BB": [4.0, 5.0],
//!         "B": [5.0, 6.0],
//!         "CCC": [6.0, 999.0]
//!       }
//!     },
//!     {
//!       "name": "interest_coverage",
//!       "formula": "ebitda / interest_expense",
//!       "weight": 0.25,
//!       "thresholds": {
//!         "AAA": [8.0, 999.0],
//!         "AA": [6.0, 8.0],
//!         "A": [4.5, 6.0],
//!         "BBB": [3.0, 4.5],
//!         "BB": [2.0, 3.0],
//!         "B": [1.0, 2.0],
//!         "CCC": [0.0, 1.0]
//!       }
//!     }
//!   ]
//! }
//! ```
//!
//! # Example Usage (Future)
//!
//! ```rust,ignore
//! use finstack_statements::extensions::{CreditScorecardExtension, ExtensionRegistry};
//!
//! let mut registry = ExtensionRegistry::new();
//! registry.register(Box::new(CreditScorecardExtension::new()))?;
//!
//! let results = registry.execute_all(&context)?;
//! ```

use super::plugin::{Extension, ExtensionContext, ExtensionMetadata, ExtensionResult};
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Credit scorecard analysis extension for rating and stress testing.
///
/// **Note:** This is a placeholder implementation. The extension will return
/// `NotImplemented` status when executed.
pub struct CreditScorecardExtension {
    /// Extension configuration
    config: Option<ScorecardConfig>,
}

/// Configuration for credit scorecard analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorecardConfig {
    /// Rating scale to use (e.g., "S&P", "Moody's", "Fitch")
    #[serde(default = "default_rating_scale")]
    pub rating_scale: String,

    /// List of metrics to evaluate
    #[serde(default)]
    pub metrics: Vec<ScorecardMetric>,

    /// Minimum acceptable rating (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_rating: Option<String>,
}

/// Definition of a scorecard metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorecardMetric {
    /// Metric name
    pub name: String,

    /// Formula to calculate the metric (DSL syntax)
    pub formula: String,

    /// Weight in overall score (0.0 to 1.0)
    #[serde(default = "default_weight")]
    pub weight: f64,

    /// Rating thresholds: rating → [min, max]
    #[serde(default)]
    pub thresholds: indexmap::IndexMap<String, (f64, f64)>,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_rating_scale() -> String {
    "S&P".into()
}

fn default_weight() -> f64 {
    1.0
}

impl CreditScorecardExtension {
    /// Create a new credit scorecard extension with default configuration.
    pub fn new() -> Self {
        Self { config: None }
    }

    /// Create a new credit scorecard extension with the given configuration.
    pub fn with_config(config: ScorecardConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> Option<&ScorecardConfig> {
        self.config.as_ref()
    }

    /// Set the configuration.
    pub fn set_config(&mut self, config: ScorecardConfig) {
        self.config = Some(config);
    }
}

impl Default for CreditScorecardExtension {
    fn default() -> Self {
        Self::new()
    }
}

impl Extension for CreditScorecardExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "credit_scorecard".into(),
            version: "0.1.0".into(),
            description: Some(
                "Credit rating and stress testing based on financial metrics".into(),
            ),
            author: Some("Finstack Team".into()),
        }
    }

    fn execute(&mut self, _context: &ExtensionContext) -> Result<ExtensionResult> {
        // Placeholder implementation
        Ok(ExtensionResult::not_implemented(
            "Credit scorecard analysis is not yet implemented. \
             This extension will provide credit rating assignment based on \
             financial metrics in a future release. See documentation for planned features."
        )
        .with_data("planned_features", serde_json::json!([
            "Credit rating assignment based on financial metrics",
            "Weighted scoring across multiple metrics",
            "Configurable rating scales and thresholds",
            "Support for multiple rating agencies (S&P, Moody's, Fitch)",
            "Rating history tracking across periods"
        ])))
    }

    fn is_enabled(&self) -> bool {
        // Extension is always available but returns NotImplemented
        true
    }

    fn config_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "ScorecardConfig",
            "type": "object",
            "properties": {
                "rating_scale": {
                    "type": "string",
                    "default": "S&P",
                    "description": "Rating scale to use (e.g., 'S&P', 'Moody's', 'Fitch')"
                },
                "metrics": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["name", "formula"],
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Metric name"
                            },
                            "formula": {
                                "type": "string",
                                "description": "Formula to calculate the metric (DSL syntax)"
                            },
                            "weight": {
                                "type": "number",
                                "default": 1.0,
                                "description": "Weight in overall score (0.0 to 1.0)"
                            },
                            "thresholds": {
                                "type": "object",
                                "description": "Rating thresholds: rating → [min, max]"
                            },
                            "description": {
                                "type": "string",
                                "description": "Metric description"
                            }
                        }
                    }
                },
                "min_rating": {
                    "type": "string",
                    "description": "Minimum acceptable rating (optional)"
                }
            }
        }))
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        // Validate configuration structure
        let scorecard_config: ScorecardConfig =
            serde_json::from_value(config.clone()).map_err(|e| {
                crate::error::Error::invalid_input(format!(
                    "Invalid scorecard configuration: {}",
                    e
                ))
            })?;

        // Validate metric weights sum to reasonable values
        let total_weight: f64 = scorecard_config.metrics.iter().map(|m| m.weight).sum();
        if total_weight > 0.0 && !(0.01..=100.0).contains(&total_weight) {
            return Err(crate::error::Error::invalid_input(format!(
                "Total metric weights ({}) should be between 0.01 and 100.0",
                total_weight
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scorecard_extension_creation() {
        let extension = CreditScorecardExtension::new();
        let metadata = extension.metadata();

        assert_eq!(metadata.name, "credit_scorecard");
        assert_eq!(metadata.version, "0.1.0");
        assert!(extension.is_enabled());
    }

    #[test]
    fn test_scorecard_extension_with_config() {
        let config = ScorecardConfig {
            rating_scale: "S&P".into(),
            metrics: vec![ScorecardMetric {
                name: "leverage".into(),
                formula: "debt / ebitda".into(),
                weight: 0.3,
                thresholds: indexmap::IndexMap::new(),
                description: Some("Leverage ratio".into()),
            }],
            min_rating: None,
        };

        let extension = CreditScorecardExtension::with_config(config);
        assert!(extension.config().is_some());
        assert_eq!(extension.config().unwrap().metrics.len(), 1);
    }

    #[test]
    fn test_scorecard_execute_not_implemented() {
        use crate::evaluator::core::Results;
        use crate::types::FinancialModelSpec;

        let model = FinancialModelSpec::new("test", Vec::new());
        let results = Results::new();
        let context = ExtensionContext::new(&model, &results);

        let mut extension = CreditScorecardExtension::new();
        let result = extension.execute(&context).unwrap();

        assert_eq!(result.status, super::super::plugin::ExtensionStatus::NotImplemented);
        assert!(result.message.contains("not yet implemented"));
    }

    #[test]
    fn test_scorecard_config_schema() {
        let extension = CreditScorecardExtension::new();
        let schema = extension.config_schema();

        assert!(schema.is_some());
        let schema_obj = schema.unwrap();
        assert!(schema_obj.get("properties").is_some());
    }

    #[test]
    fn test_scorecard_config_validation() {
        let extension = CreditScorecardExtension::new();

        let valid_config = serde_json::json!({
            "rating_scale": "S&P",
            "metrics": [
                {
                    "name": "leverage",
                    "formula": "debt / ebitda",
                    "weight": 0.3,
                    "thresholds": {
                        "AAA": [0.0, 1.0],
                        "AA": [1.0, 2.0],
                        "A": [2.0, 3.0]
                    }
                }
            ]
        });

        assert!(extension.validate_config(&valid_config).is_ok());
    }

    #[test]
    fn test_scorecard_config_validation_invalid_weights() {
        let extension = CreditScorecardExtension::new();

        let invalid_config = serde_json::json!({
            "rating_scale": "S&P",
            "metrics": [
                {
                    "name": "leverage",
                    "formula": "debt / ebitda",
                    "weight": 150.0
                }
            ]
        });

        assert!(extension.validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_scorecard_metric() {
        let metric = ScorecardMetric {
            name: "debt_to_ebitda".into(),
            formula: "total_debt / ttm(ebitda)".into(),
            weight: 0.3,
            thresholds: indexmap::IndexMap::new(),
            description: Some("Leverage ratio".into()),
        };

        assert_eq!(metric.name, "debt_to_ebitda");
        assert_eq!(metric.weight, 0.3);
    }

    #[test]
    fn test_scorecard_config_with_thresholds() {
        let mut thresholds = indexmap::IndexMap::new();
        thresholds.insert("AAA".into(), (0.0, 1.0));
        thresholds.insert("AA".into(), (1.0, 2.0));
        thresholds.insert("A".into(), (2.0, 3.0));

        let metric = ScorecardMetric {
            name: "debt_to_ebitda".into(),
            formula: "total_debt / ttm(ebitda)".into(),
            weight: 0.3,
            thresholds,
            description: Some("Leverage ratio".into()),
        };

        assert_eq!(metric.thresholds.len(), 3);
        assert_eq!(metric.thresholds.get("AAA"), Some(&(0.0, 1.0)));
    }
}
