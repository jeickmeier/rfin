//! Integration tests for the extension system.

use finstack_statements::extensions::{
    CorkscrewExtension, CreditScorecardExtension, Extension, ExtensionContext, ExtensionMetadata,
    ExtensionRegistry, ExtensionResult, ExtensionStatus,
};
use finstack_statements::prelude::*;
use indexmap::indexmap;

// ============================================================================
// Test Extension Implementation
// ============================================================================

struct SimpleValidationExtension {
    enabled: bool,
}

impl SimpleValidationExtension {
    fn new() -> Self {
        Self { enabled: true }
    }
}

impl Extension for SimpleValidationExtension {
    fn metadata(&self) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "simple_validator".into(),
            version: "1.0.0".into(),
            description: Some("Simple validation extension for testing".into()),
            author: Some("Test Suite".into()),
        }
    }

    fn execute(&mut self, context: &ExtensionContext) -> Result<ExtensionResult> {
        // Count nodes in the model
        let node_count = context.model.nodes.len();
        let period_count = context.model.periods.len();

        Ok(ExtensionResult::success("Validation passed")
            .with_data("node_count", serde_json::json!(node_count))
            .with_data("period_count", serde_json::json!(period_count)))
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// ============================================================================
// Extension Registry Tests
// ============================================================================

#[test]
fn test_extension_registry_basic() {
    let mut registry = ExtensionRegistry::new();

    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());

    // Register an extension
    registry
        .register(Box::new(SimpleValidationExtension::new()))
        .unwrap();

    assert_eq!(registry.len(), 1);
    assert!(!registry.is_empty());
    assert!(registry.has("simple_validator"));
}

#[test]
fn test_extension_registry_duplicate_error() {
    let mut registry = ExtensionRegistry::new();

    registry
        .register(Box::new(SimpleValidationExtension::new()))
        .unwrap();

    let result = registry.register(Box::new(SimpleValidationExtension::new()));
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already registered"));
}

#[test]
fn test_extension_registry_list() {
    let mut registry = ExtensionRegistry::new();

    registry
        .register(Box::new(SimpleValidationExtension::new()))
        .unwrap();
    registry
        .register(Box::new(CorkscrewExtension::new()))
        .unwrap();

    let names = registry.list();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"simple_validator".to_string()));
    assert!(names.contains(&"corkscrew".to_string()));
}

#[test]
fn test_extension_registry_metadata() {
    let mut registry = ExtensionRegistry::new();

    registry
        .register(Box::new(SimpleValidationExtension::new()))
        .unwrap();

    let metadata_list = registry.list_metadata();
    assert_eq!(metadata_list.len(), 1);

    let metadata = &metadata_list[0];
    assert_eq!(metadata.name, "simple_validator");
    assert_eq!(metadata.version, "1.0.0");
}

#[test]
fn test_extension_execution() {
    let mut registry = ExtensionRegistry::new();

    registry
        .register(Box::new(SimpleValidationExtension::new()))
        .unwrap();

    // Create a simple model
    let model = ModelBuilder::new("test_model")
        .periods("2025Q1..2025Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
            ],
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    // Create extension context
    let context = ExtensionContext::new(&model, &results);

    // Execute the extension
    let result = registry.execute("simple_validator", &context).unwrap();

    assert_eq!(result.status, ExtensionStatus::Success);
    assert_eq!(result.message, "Validation passed");
    assert_eq!(
        result.data.get("node_count").unwrap(),
        &serde_json::json!(1)
    );
    assert_eq!(
        result.data.get("period_count").unwrap(),
        &serde_json::json!(2)
    );
}

#[test]
fn test_extension_execute_all() {
    let mut registry = ExtensionRegistry::new();

    registry
        .register(Box::new(SimpleValidationExtension::new()))
        .unwrap();
    registry
        .register(Box::new(CorkscrewExtension::new()))
        .unwrap();

    // Create a simple model
    let model = ModelBuilder::new("test_model")
        .periods("2025Q1..2025Q2", None)
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let context = ExtensionContext::new(&model, &results);

    // Execute all extensions (use safe version since corkscrew lacks config)
    let extension_results = registry.execute_all_safe(&context);

    assert_eq!(extension_results.len(), 2);
    assert!(extension_results.contains_key("simple_validator"));
    assert!(extension_results.contains_key("corkscrew"));

    // Verify simple_validator succeeded
    let validator_result = extension_results["simple_validator"].as_ref().unwrap();
    assert_eq!(validator_result.status, ExtensionStatus::Success);

    // Verify corkscrew errors due to missing config
    assert!(extension_results["corkscrew"].is_err());
    assert!(extension_results["corkscrew"]
        .as_ref()
        .unwrap_err()
        .to_string()
        .contains("requires configuration"));
}

#[test]
fn test_extension_execution_order() {
    let mut registry = ExtensionRegistry::new();

    registry
        .register(Box::new(CorkscrewExtension::new()))
        .unwrap();
    registry
        .register(Box::new(SimpleValidationExtension::new()))
        .unwrap();

    // Set custom execution order
    registry
        .set_execution_order(vec!["simple_validator".into(), "corkscrew".into()])
        .unwrap();

    let model = ModelBuilder::new("test_model")
        .periods("2025Q1..2025Q2", None)
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let context = ExtensionContext::new(&model, &results);

    let extension_results = registry.execute_all_safe(&context);

    // Verify execution order (keys are still in order even with safe execution)
    let keys: Vec<_> = extension_results.keys().cloned().collect();
    assert_eq!(keys, vec!["simple_validator", "corkscrew"]);
}

#[test]
fn test_extension_execution_order_invalid() {
    let mut registry = ExtensionRegistry::new();

    registry
        .register(Box::new(SimpleValidationExtension::new()))
        .unwrap();

    // Try to set execution order with non-existent extension
    let result =
        registry.set_execution_order(vec!["simple_validator".into(), "nonexistent".into()]);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not registered"));
}

// ============================================================================
// Corkscrew Extension Tests
// ============================================================================

#[test]
fn test_corkscrew_extension_placeholder() {
    let model = ModelBuilder::new("test_model")
        .periods("2025Q1..2025Q2", None)
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let context = ExtensionContext::new(&model, &results);

    let mut extension = CorkscrewExtension::new();
    // Extension requires config, should error without it
    let result = extension.execute(&context);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("requires configuration"));
}

#[test]
fn test_corkscrew_extension_metadata() {
    let extension = CorkscrewExtension::new();
    let metadata = extension.metadata();

    assert_eq!(metadata.name, "corkscrew");
    assert_eq!(metadata.version, "0.1.0");
    assert!(metadata.description.is_some());
    assert!(extension.is_enabled());
}

#[test]
fn test_corkscrew_config_schema() {
    let extension = CorkscrewExtension::new();
    let schema = extension.config_schema();

    assert!(schema.is_some());
    let schema_value = schema.unwrap();

    // Verify schema has expected properties
    assert!(schema_value.get("properties").is_some());
    let properties = schema_value.get("properties").unwrap();
    assert!(properties.get("accounts").is_some());
    assert!(properties.get("tolerance").is_some());
}

// ============================================================================
// Credit Scorecard Extension Tests
// ============================================================================

#[test]
fn test_scorecard_extension_placeholder() {
    let model = ModelBuilder::new("test_model")
        .periods("2025Q1..2025Q2", None)
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let context = ExtensionContext::new(&model, &results);

    let mut extension = CreditScorecardExtension::new();
    // Extension requires config, should error without it
    let result = extension.execute(&context);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("requires configuration"));
}

#[test]
fn test_scorecard_extension_metadata() {
    let extension = CreditScorecardExtension::new();
    let metadata = extension.metadata();

    assert_eq!(metadata.name, "credit_scorecard");
    assert_eq!(metadata.version, "0.1.0");
    assert!(metadata.description.is_some());
    assert!(extension.is_enabled());
}

#[test]
fn test_scorecard_config_schema() {
    let extension = CreditScorecardExtension::new();
    let schema = extension.config_schema();

    assert!(schema.is_some());
    let schema_value = schema.unwrap();

    // Verify schema has expected properties
    assert!(schema_value.get("properties").is_some());
    let properties = schema_value.get("properties").unwrap();
    assert!(properties.get("rating_scale").is_some());
    assert!(properties.get("metrics").is_some());
    assert!(properties.get("min_rating").is_some());
}

// ============================================================================
// Extension Context Tests
// ============================================================================

#[test]
fn test_extension_context_creation() {
    let model = ModelBuilder::new("test_model")
        .periods("2025Q1..2025Q2", None)
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let context = ExtensionContext::new(&model, &results);

    assert_eq!(context.model.id, "test_model");
    assert!(context.config.is_none());
    assert!(context.runtime_context.is_empty());
}

#[test]
fn test_extension_context_with_config() {
    let model = ModelBuilder::new("test_model")
        .periods("2025Q1..2025Q2", None)
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let config = serde_json::json!({"key": "value"});
    let context = ExtensionContext::new(&model, &results).with_config(&config);

    assert!(context.config.is_some());
    assert_eq!(context.config.unwrap(), &config);
}

#[test]
fn test_extension_context_with_runtime_context() {
    let model = ModelBuilder::new("test_model")
        .periods("2025Q1..2025Q2", None)
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let context = ExtensionContext::new(&model, &results)
        .add_context("key1", serde_json::json!("value1"))
        .add_context("key2", serde_json::json!(42));

    assert_eq!(context.runtime_context.len(), 2);
    assert_eq!(
        context.runtime_context.get("key1").unwrap(),
        &serde_json::json!("value1")
    );
    assert_eq!(
        context.runtime_context.get("key2").unwrap(),
        &serde_json::json!(42)
    );
}

// ============================================================================
// Extension Result Tests
// ============================================================================

#[test]
fn test_extension_result_success() {
    let result = ExtensionResult::success("All checks passed");

    assert_eq!(result.status, ExtensionStatus::Success);
    assert_eq!(result.message, "All checks passed");
    assert!(result.data.is_empty());
    assert!(result.warnings.is_empty());
    assert!(result.errors.is_empty());
}

#[test]
fn test_extension_result_failure() {
    let result = ExtensionResult::failure("Validation failed");

    assert_eq!(result.status, ExtensionStatus::Failed);
    assert_eq!(result.message, "Validation failed");
}

#[test]
fn test_extension_result_with_data() {
    let result = ExtensionResult::success("Analysis complete")
        .with_data("total_revenue", serde_json::json!(1_000_000.0))
        .with_data("total_expenses", serde_json::json!(600_000.0))
        .with_warning("Minor rounding difference detected")
        .with_error("Missing data for period 2025Q3");

    assert_eq!(result.status, ExtensionStatus::Success);
    assert_eq!(result.data.len(), 2);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.errors.len(), 1);
}

#[test]
fn test_extension_result_serialization() {
    let result =
        ExtensionResult::success("Test completed").with_data("count", serde_json::json!(5));

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: ExtensionResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.status, ExtensionStatus::Success);
    assert_eq!(deserialized.message, "Test completed");
    assert_eq!(deserialized.data.len(), 1);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complete_workflow_with_extensions() {
    // Build a complete P&L model
    let model = ModelBuilder::new("Acme Corp P&L")
        .periods("2025Q1..2025Q4", Some("2025Q2"))
        .unwrap()
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(10_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(11_000_000.0),
                ),
            ],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => serde_json::json!(0.05) },
            },
        )
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .build()
        .unwrap();

    // Evaluate the model
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    // Verify results
    assert_eq!(results.nodes.len(), 3);

    // Set up extension registry
    let mut registry = ExtensionRegistry::new();
    registry
        .register(Box::new(SimpleValidationExtension::new()))
        .unwrap();
    registry
        .register(Box::new(CorkscrewExtension::new()))
        .unwrap();
    registry
        .register(Box::new(CreditScorecardExtension::new()))
        .unwrap();

    // Create context and execute extensions (use safe version since some lack config)
    let context = ExtensionContext::new(&model, &results);
    let extension_results = registry.execute_all_safe(&context);

    // Verify all extensions attempted
    assert_eq!(extension_results.len(), 3);

    // Verify simple validator succeeded
    let validator_result = extension_results["simple_validator"].as_ref().unwrap();
    assert_eq!(validator_result.status, ExtensionStatus::Success);
    assert_eq!(
        validator_result.data.get("node_count").unwrap(),
        &serde_json::json!(3)
    );

    // Verify extensions without config error out
    assert!(extension_results["corkscrew"].is_err());
    assert!(extension_results["corkscrew"]
        .as_ref()
        .unwrap_err()
        .to_string()
        .contains("requires configuration"));

    assert!(extension_results["credit_scorecard"].is_err());
    assert!(extension_results["credit_scorecard"]
        .as_ref()
        .unwrap_err()
        .to_string()
        .contains("requires configuration"));
}
