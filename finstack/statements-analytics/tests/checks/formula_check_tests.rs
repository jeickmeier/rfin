#![allow(clippy::unwrap_used)]

use finstack_core::dates::PeriodId;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::checks::{Check, CheckCategory, CheckContext, Severity};
use finstack_statements::evaluator::Evaluator;
use finstack_statements::types::AmountOrScalar;
use finstack_statements_analytics::analysis::checks::FormulaCheck;

fn q(quarter: u8) -> PeriodId {
    PeriodId::quarter(2025, quarter)
}

fn s(v: f64) -> AmountOrScalar {
    AmountOrScalar::scalar(v)
}

// ============================================================================
// Basic passing formula
// ============================================================================

#[test]
fn revenue_positive_passes() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value("revenue", &[(q(1), s(100.0)), (q(2), s(200.0))])
        .build()
        .unwrap();

    let mut ev = Evaluator::new();
    let results = ev.evaluate(&model).unwrap();

    let check = FormulaCheck {
        id: "revenue_positive".into(),
        name: "Revenue must be positive".into(),
        category: CheckCategory::InternalConsistency,
        severity: Severity::Error,
        formula: "revenue > 0".into(),
        message_template: "Revenue was non-positive in {period}".into(),
        tolerance: None,
    };

    let ctx = CheckContext::new(&model, &results);
    let result = check.execute(&ctx).unwrap();

    assert!(result.passed);
    assert!(result.findings.is_empty());
}

// ============================================================================
// Failing formula produces finding
// ============================================================================

#[test]
fn failing_formula_produces_finding() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value("revenue", &[(q(1), s(-50.0))])
        .build()
        .unwrap();

    let mut ev = Evaluator::new();
    let results = ev.evaluate(&model).unwrap();

    let check = FormulaCheck {
        id: "revenue_positive".into(),
        name: "Revenue must be positive".into(),
        category: CheckCategory::InternalConsistency,
        severity: Severity::Error,
        formula: "revenue > 0".into(),
        message_template: "Revenue was non-positive in {period}".into(),
        tolerance: None,
    };

    let ctx = CheckContext::new(&model, &results);
    let result = check.execute(&ctx).unwrap();

    assert!(!result.passed);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].severity, Severity::Error);
    assert!(result.findings[0].message.contains("non-positive"));
    assert_eq!(result.findings[0].period, Some(q(1)));
}

// ============================================================================
// Warning severity does not fail
// ============================================================================

#[test]
fn warning_severity_does_not_fail() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value("margin", &[(q(1), s(0.0))])
        .build()
        .unwrap();

    let mut ev = Evaluator::new();
    let results = ev.evaluate(&model).unwrap();

    let check = FormulaCheck {
        id: "margin_nonzero".into(),
        name: "Margin should not be zero".into(),
        category: CheckCategory::InternalConsistency,
        severity: Severity::Warning,
        formula: "margin > 0".into(),
        message_template: "Margin is zero in {period}".into(),
        tolerance: None,
    };

    let ctx = CheckContext::new(&model, &results);
    let result = check.execute(&ctx).unwrap();

    assert!(result.passed); // warning-only, still passes
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].severity, Severity::Warning);
}

// ============================================================================
// Arithmetic formula
// ============================================================================

#[test]
fn arithmetic_formula_evaluates() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value("assets", &[(q(1), s(100.0))])
        .value("liabilities", &[(q(1), s(60.0))])
        .build()
        .unwrap();

    let mut ev = Evaluator::new();
    let results = ev.evaluate(&model).unwrap();

    // assets - liabilities = 40 > 0  → result is 1.0 (truthy), passes.
    let check = FormulaCheck {
        id: "equity_positive".into(),
        name: "Equity positive".into(),
        category: CheckCategory::AccountingIdentity,
        severity: Severity::Error,
        formula: "assets > liabilities".into(),
        message_template: "Equity negative in {period}".into(),
        tolerance: None,
    };

    let ctx = CheckContext::new(&model, &results);
    let result = check.execute(&ctx).unwrap();

    assert!(result.passed);
    assert!(result.findings.is_empty());
}

// ============================================================================
// JSON deserialization
// ============================================================================

#[test]
fn json_deserialization_works() {
    let json = r#"{
        "id": "test_check",
        "name": "Test Check",
        "category": "data_quality",
        "severity": "warning",
        "formula": "revenue > 0",
        "message_template": "Revenue bad in {period}",
        "tolerance": 0.01
    }"#;

    let check: FormulaCheck = serde_json::from_str(json).unwrap();

    assert_eq!(check.id, "test_check");
    assert_eq!(check.category, CheckCategory::DataQuality);
    assert_eq!(check.severity, Severity::Warning);
    assert_eq!(check.tolerance, Some(0.01));
}

// ============================================================================
// Missing node skips gracefully
// ============================================================================

#[test]
fn missing_node_skips_period() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .build()
        .unwrap();

    let mut ev = Evaluator::new();
    let results = ev.evaluate(&model).unwrap();

    let check = FormulaCheck {
        id: "check_missing".into(),
        name: "Missing node".into(),
        category: CheckCategory::DataQuality,
        severity: Severity::Error,
        formula: "nonexistent > 0".into(),
        message_template: "Missing in {period}".into(),
        tolerance: None,
    };

    let ctx = CheckContext::new(&model, &results);
    let result = check.execute(&ctx).unwrap();

    assert!(result.passed);
    assert!(result.findings.is_empty());
}
