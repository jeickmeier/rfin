//! Scorecard extension integration tests.
#![allow(clippy::expect_used)]

use finstack_statements::extensions::{
    CreditScorecardExtension, Extension, ExtensionContext, ScorecardConfig, ScorecardMetric,
};

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
    assert_eq!(
        extension
            .config()
            .expect("test should succeed")
            .metrics
            .len(),
        1
    );
}

#[test]
fn test_scorecard_execute_requires_config() {
    use finstack_statements::evaluator::Results;
    use finstack_statements::types::FinancialModelSpec;

    let model = FinancialModelSpec::new("test", Vec::new());
    let results = Results::new();
    let context = ExtensionContext::new(&model, &results);

    let mut extension = CreditScorecardExtension::new();
    let result = extension.execute(&context);

    assert!(result.is_err());
    assert!(result
        .expect_err("should fail")
        .to_string()
        .contains("requires configuration"));
}

#[test]
fn test_scorecard_config_schema() {
    let extension = CreditScorecardExtension::new();
    let schema = extension.config_schema();

    assert!(schema.is_some());
    let schema_obj = schema.expect("test should succeed");
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
