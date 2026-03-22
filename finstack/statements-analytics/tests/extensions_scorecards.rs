//! Scorecard extension integration tests.
#![allow(clippy::expect_used)]

use finstack_statements::extensions::{Extension, ExtensionContext};
use finstack_statements_analytics::extensions::{
    CreditScorecardExtension, ScorecardConfig, ScorecardMetric,
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
    use finstack_statements::evaluator::StatementResult;
    use finstack_statements::types::FinancialModelSpec;

    let model = FinancialModelSpec::new("test", Vec::new());
    let results = StatementResult::new();
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
fn test_scorecard_config_validation_invalid_scale() {
    let extension = CreditScorecardExtension::new();

    let invalid_config = serde_json::json!({
        "rating_scale": "UnknownScale",
        "metrics": []
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

#[test]
fn test_scorecard_ttm_formula_uses_full_history() {
    use finstack_core::dates::PeriodId;
    use finstack_statements::builder::ModelBuilder;
    use finstack_statements::evaluator::Evaluator;
    use finstack_statements::types::AmountOrScalar;

    let model = ModelBuilder::new("scorecard-ttm")
        .periods("2025Q1..Q4", None)
        .expect("valid periods")
        .value(
            "ebitda",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
                (PeriodId::quarter(2025, 3), AmountOrScalar::scalar(120.0)),
                (PeriodId::quarter(2025, 4), AmountOrScalar::scalar(130.0)),
            ],
        )
        .value(
            "total_debt",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(1000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(1000.0)),
                (PeriodId::quarter(2025, 3), AmountOrScalar::scalar(1000.0)),
                (PeriodId::quarter(2025, 4), AmountOrScalar::scalar(1000.0)),
            ],
        )
        .build()
        .expect("valid model");

    let mut evaluator = Evaluator::new();
    let results = evaluator
        .evaluate(&model)
        .expect("evaluation should succeed");

    let context = ExtensionContext::new(&model, &results);
    let config = ScorecardConfig {
        rating_scale: "S&P".into(),
        metrics: vec![ScorecardMetric {
            name: "leverage".into(),
            formula: "total_debt / ttm(ebitda)".into(),
            weight: 1.0,
            thresholds: indexmap::IndexMap::new(),
            description: None,
        }],
        min_rating: None,
    };
    let mut extension = CreditScorecardExtension::with_config(config);
    let result = extension
        .execute(&context)
        .expect("scorecard should succeed");

    let score_data = result.data.get("total_score").expect("total_score");
    let leverage_data = result.data.get("metric_scores").expect("metric_scores");
    let leverage_value = leverage_data[0]["value"].as_f64().expect("leverage value");

    // ttm(ebitda) = 100 + 110 + 120 + 130 = 460
    // leverage = 1000 / 460 ≈ 2.174
    assert!(
        (leverage_value - 1000.0 / 460.0).abs() < 0.01,
        "leverage should be ~2.174, got {}",
        leverage_value
    );
    let _ = score_data;
}

#[test]
fn test_scorecard_warns_when_thresholds_do_not_cover_metric_value() {
    use finstack_core::dates::PeriodId;
    use finstack_statements::builder::ModelBuilder;
    use finstack_statements::evaluator::Evaluator;
    use finstack_statements::types::AmountOrScalar;

    let model = ModelBuilder::new("scorecard-threshold-gap")
        .periods("2025Q1..Q1", None)
        .expect("valid periods")
        .value(
            "ebitda",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0))],
        )
        .value(
            "total_debt",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(250.0))],
        )
        .build()
        .expect("valid model");

    let mut evaluator = Evaluator::new();
    let results = evaluator
        .evaluate(&model)
        .expect("evaluation should succeed");
    let context = ExtensionContext::new(&model, &results);

    let mut thresholds = indexmap::IndexMap::new();
    thresholds.insert("AAA".into(), (0.0, 1.0));

    let config = ScorecardConfig {
        rating_scale: "S&P".into(),
        metrics: vec![ScorecardMetric {
            name: "leverage".into(),
            formula: "total_debt / ebitda".into(),
            weight: 1.0,
            thresholds,
            description: None,
        }],
        min_rating: None,
    };

    let mut extension = CreditScorecardExtension::with_config(config);
    let result = extension
        .execute(&context)
        .expect("scorecard should succeed");

    assert_eq!(
        result.data.get("total_score").and_then(|v| v.as_f64()),
        Some(50.0)
    );
}
