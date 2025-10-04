//! Core extension trait and types.

use crate::error::Result;
use crate::evaluator::Results;
use crate::types::FinancialModelSpec;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Extension trait for custom analysis and validation.
///
/// Extensions can process financial models and evaluation results to provide
/// additional insights, perform validations, or transform data.
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_statements::extensions::{Extension, ExtensionContext, ExtensionResult};
///
/// struct ValidationExtension;
///
/// impl Extension for ValidationExtension {
///     fn metadata(&self) -> ExtensionMetadata {
///         ExtensionMetadata {
///             name: "validator".into(),
///             version: "1.0.0".into(),
///             description: Some("Validates model consistency".into()),
///             author: Some("Financial Team".into()),
///         }
///     }
///
///     fn execute(&mut self, context: &ExtensionContext) -> Result<ExtensionResult> {
///         // Perform validation logic
///         Ok(ExtensionResult::success("All validations passed"))
///     }
/// }
/// ```
pub trait Extension: Send + Sync {
    /// Get extension metadata.
    fn metadata(&self) -> ExtensionMetadata;

    /// Execute the extension.
    ///
    /// # Arguments
    ///
    /// * `context` - Extension context containing model and results
    ///
    /// # Returns
    ///
    /// Returns an `ExtensionResult` containing the execution status and any output data.
    fn execute(&mut self, context: &ExtensionContext) -> Result<ExtensionResult>;

    /// Check if the extension is enabled.
    ///
    /// Default implementation returns `true`. Override to implement conditional execution.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Get extension configuration schema (optional).
    ///
    /// Returns a JSON schema describing the expected configuration format.
    fn config_schema(&self) -> Option<serde_json::Value> {
        None
    }

    /// Validate extension configuration (optional).
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if configuration is valid, or an error otherwise.
    fn validate_config(&self, _config: &serde_json::Value) -> Result<()> {
        Ok(())
    }
}

/// Metadata about an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    /// Unique extension name
    pub name: String,

    /// Semantic version
    pub version: String,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Extension author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// Context passed to extensions during execution.
#[derive(Debug)]
pub struct ExtensionContext<'a> {
    /// The financial model being analyzed
    pub model: &'a FinancialModelSpec,

    /// Evaluation results
    pub results: &'a Results,

    /// Extension-specific configuration
    pub config: Option<&'a serde_json::Value>,

    /// Additional runtime context
    pub runtime_context: IndexMap<String, serde_json::Value>,
}

impl<'a> ExtensionContext<'a> {
    /// Create a new extension context.
    pub fn new(model: &'a FinancialModelSpec, results: &'a Results) -> Self {
        Self {
            model,
            results,
            config: None,
            runtime_context: IndexMap::new(),
        }
    }

    /// Add a configuration.
    #[must_use = "builder methods take self by value and return the modified value"]
    pub fn with_config(mut self, config: &'a serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }

    /// Add runtime context data.
    #[must_use = "builder methods take self by value and return the modified value"]
    pub fn add_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.runtime_context.insert(key.into(), value);
        self
    }
}

/// Result of extension execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionResult {
    /// Execution status
    pub status: ExtensionStatus,

    /// Human-readable message
    pub message: String,

    /// Structured output data
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub data: IndexMap<String, serde_json::Value>,

    /// Warnings generated during execution
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,

    /// Errors encountered (non-fatal)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

impl ExtensionResult {
    /// Create a successful result.
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            status: ExtensionStatus::Success,
            message: message.into(),
            data: IndexMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Create a failed result.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            status: ExtensionStatus::Failed,
            message: message.into(),
            data: IndexMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Create a not implemented result.
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self {
            status: ExtensionStatus::NotImplemented,
            message: message.into(),
            data: IndexMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Create a skipped result.
    pub fn skipped(message: impl Into<String>) -> Self {
        Self {
            status: ExtensionStatus::Skipped,
            message: message.into(),
            data: IndexMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add data to the result.
    #[must_use = "builder methods take self by value and return the modified value"]
    pub fn with_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Add a warning.
    #[must_use = "builder methods take self by value and return the modified value"]
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add an error.
    #[must_use = "builder methods take self by value and return the modified value"]
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self
    }
}

/// Status of extension execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionStatus {
    /// Extension executed successfully
    Success,

    /// Extension execution failed
    Failed,

    /// Extension is not yet implemented
    NotImplemented,

    /// Extension was skipped (e.g., disabled or conditions not met)
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_metadata() {
        let metadata = ExtensionMetadata {
            name: "test_extension".into(),
            version: "1.0.0".into(),
            description: Some("Test extension".into()),
            author: Some("Test Author".into()),
        };

        assert_eq!(metadata.name, "test_extension");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_extension_result_success() {
        let result = ExtensionResult::success("Operation completed");

        assert_eq!(result.status, ExtensionStatus::Success);
        assert_eq!(result.message, "Operation completed");
        assert!(result.data.is_empty());
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_extension_result_with_data() {
        let result = ExtensionResult::success("Analysis complete")
            .with_data("total_revenue", serde_json::json!(1_000_000.0))
            .with_warning("Minor inconsistency detected");

        assert_eq!(result.status, ExtensionStatus::Success);
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_extension_result_not_implemented() {
        let result = ExtensionResult::not_implemented("This feature is coming soon");

        assert_eq!(result.status, ExtensionStatus::NotImplemented);
    }

    #[test]
    fn test_extension_status_serialization() {
        let status = ExtensionStatus::Success;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""success""#);

        let deserialized: ExtensionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ExtensionStatus::Success);
    }
}
