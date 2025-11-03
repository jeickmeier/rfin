//! Node explanation extension.

use crate::error::Result;
use crate::evaluator::DependencyGraph;
use crate::explain::DependencyTracer;
use crate::extensions::{Extension, ExtensionContext, ExtensionMetadata, ExtensionResult};

/// Extension for explaining node dependencies.
///
/// # Examples
///
/// ```rust
/// use finstack_statements::prelude::*;
/// use finstack_statements::extensions::ExplainerExtension;
///
/// # fn main() -> Result<()> {
/// let model = ModelBuilder::new("demo")
///     .periods("2025Q1..Q1", None)?
///     .value("revenue", &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0))])
///     .compute("gross_profit", "revenue * 0.4")?
///     .build()?;
///
/// let mut evaluator = Evaluator::new();
/// let results = evaluator.evaluate(&model)?;
///
/// // Create explainer extension
/// let mut extension = ExplainerExtension::new("gross_profit");
///
/// // Execute the extension
/// let context = ExtensionContext::new(&model, &results);
/// let result = extension.execute(&context)?;
///
/// assert_eq!(result.status, finstack_statements::extensions::ExtensionStatus::Success);
/// # Ok(())
/// # }
/// ```
pub struct ExplainerExtension {
    node_id: String,
}

impl ExplainerExtension {
    /// Create a new explainer extension for a specific node.
    ///
    /// # Arguments
    /// * `node_id` - The node identifier to explain
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
        }
    }
}

impl Extension for ExplainerExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "explainer".into(),
            version: "0.1.0".into(),
            description: Some("Dependency tracing and formula explanation".into()),
            author: None,
        }
    }

    fn execute(&mut self, context: &ExtensionContext) -> Result<ExtensionResult> {
        let graph = DependencyGraph::from_model(context.model)?;
        let tracer = DependencyTracer::new(context.model, &graph);
        let tree = tracer.dependency_tree(&self.node_id)?;

        Ok(ExtensionResult::success("Node explanation generated")
            .with_data("node_id", serde_json::json!(self.node_id))
            .with_data("depth", serde_json::json!(tree.depth()))
            .with_data("node_count", serde_json::json!(tree.node_count())))
    }
}

