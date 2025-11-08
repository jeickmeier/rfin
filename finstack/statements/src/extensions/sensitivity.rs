//! Sensitivity analysis extension.

use crate::analysis::{SensitivityAnalyzer, SensitivityConfig};
use crate::error::Result;
use crate::extensions::{Extension, ExtensionContext, ExtensionMetadata, ExtensionResult};

/// Extension for running sensitivity analysis.
pub struct SensitivityExtension {
    config: SensitivityConfig,
}

impl SensitivityExtension {
    /// Create a new sensitivity extension.
    pub fn new(config: SensitivityConfig) -> Self {
        Self { config }
    }
}

impl Extension for SensitivityExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "sensitivity".into(),
            version: "0.1.0".into(),
            description: Some("Sensitivity analysis for financial models".into()),
            author: None,
        }
    }

    fn execute(&mut self, context: &ExtensionContext) -> Result<ExtensionResult> {
        let analyzer = SensitivityAnalyzer::new(context.model);
        let result = analyzer.run(&self.config)?;

        Ok(ExtensionResult::success("Sensitivity analysis complete")
            .with_data("scenario_count", serde_json::json!(result.scenarios.len())))
    }
}
