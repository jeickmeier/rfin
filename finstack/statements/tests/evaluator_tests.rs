//! Evaluator tests for Phase 3

use finstack_statements::prelude::*;

// ============================================================================
// PR #3.1 — Evaluation Context Tests
// ============================================================================

#[test]
fn test_context_set_and_get_value() {
    use finstack_statements::evaluator::EvaluationContext;
    use indexmap::IndexMap;

    let mut node_to_column = IndexMap::new();
    node_to_column.insert("revenue".to_string(), 0);
    node_to_column.insert("cogs".to_string(), 1);

    let mut ctx =
        EvaluationContext::new(PeriodId::quarter(2025, 1), node_to_column, IndexMap::new());

    ctx.set_value("revenue", 100_000.0).unwrap();
    ctx.set_value("cogs", 60_000.0).unwrap();

    assert_eq!(ctx.get_value("revenue").unwrap(), 100_000.0);
    assert_eq!(ctx.get_value("cogs").unwrap(), 60_000.0);
}

#[test]
fn test_context_unknown_node_error() {
    use finstack_statements::evaluator::EvaluationContext;
    use indexmap::IndexMap;

    let ctx = EvaluationContext::new(PeriodId::quarter(2025, 1), IndexMap::new(), IndexMap::new());

    let result = ctx.get_value("unknown");
    assert!(result.is_err());
}

// ============================================================================
// PR #3.2 — Basic Evaluator Tests
// ============================================================================

#[test]
fn test_evaluate_value_nodes() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
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
        .value(
            "opex",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(20_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(21_000.0)),
            ],
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 1)),
        Some(100_000.0)
    );
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 2)),
        Some(110_000.0)
    );
    assert_eq!(
        results.get("opex", &PeriodId::quarter(2025, 1)),
        Some(20_000.0)
    );
    assert_eq!(
        results.get("opex", &PeriodId::quarter(2025, 2)),
        Some(21_000.0)
    );
}

#[test]
fn test_evaluate_calculated_nodes() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
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
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    // Check COGS (60% of revenue)
    assert!((results.get("cogs", &PeriodId::quarter(2025, 1)).unwrap() - 60_000.0).abs() < 0.01);
    assert!((results.get("cogs", &PeriodId::quarter(2025, 2)).unwrap() - 66_000.0).abs() < 0.01);

    // Check gross profit
    assert!(
        (results
            .get("gross_profit", &PeriodId::quarter(2025, 1))
            .unwrap()
            - 40_000.0)
            .abs()
            < 0.01
    );
    assert!(
        (results
            .get("gross_profit", &PeriodId::quarter(2025, 2))
            .unwrap()
            - 44_000.0)
            .abs()
            < 0.01
    );
}

#[test]
fn test_evaluate_arithmetic_operations() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value(
            "a",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(10.0))],
        )
        .value(
            "b",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(3.0))],
        )
        .compute("add", "a + b")
        .unwrap()
        .compute("sub", "a - b")
        .unwrap()
        .compute("mul", "a * b")
        .unwrap()
        .compute("div", "a / b")
        .unwrap()
        .compute("mod", "a % b")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let q1 = PeriodId::quarter(2025, 1);
    assert_eq!(results.get("add", &q1), Some(13.0));
    assert_eq!(results.get("sub", &q1), Some(7.0));
    assert_eq!(results.get("mul", &q1), Some(30.0));
    assert!((results.get("div", &q1).unwrap() - 3.333333).abs() < 0.001);
    assert_eq!(results.get("mod", &q1), Some(1.0));
}

#[test]
fn test_evaluate_comparison_operations() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            )],
        )
        .compute("gt", "revenue > 50000")
        .unwrap()
        .compute("lt", "revenue < 200000")
        .unwrap()
        .compute("eq", "revenue == 100000")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let q1 = PeriodId::quarter(2025, 1);
    assert_eq!(results.get("gt", &q1), Some(1.0)); // true
    assert_eq!(results.get("lt", &q1), Some(1.0)); // true
    assert_eq!(results.get("eq", &q1), Some(1.0)); // true
}

#[test]
fn test_evaluate_conditional() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(80_000.0)),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(120_000.0),
                ),
            ],
        )
        .compute("bonus", "if(revenue > 100000, revenue * 0.1, 0)")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    // Q1: revenue < 100k, no bonus
    assert_eq!(results.get("bonus", &PeriodId::quarter(2025, 1)), Some(0.0));

    // Q2: revenue > 100k, get bonus
    assert!((results.get("bonus", &PeriodId::quarter(2025, 2)).unwrap() - 12_000.0).abs() < 0.01);
}

#[test]
fn test_evaluate_complex_expression() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            )],
        )
        .value(
            "cogs",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(60_000.0))],
        )
        .compute("gross_margin", "(revenue - cogs) / revenue")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let margin = results
        .get("gross_margin", &PeriodId::quarter(2025, 1))
        .unwrap();
    assert!((margin - 0.4).abs() < 0.001); // 40% margin
}

// ============================================================================
// PR #3.3 — DAG Construction Tests
// ============================================================================

#[test]
fn test_dag_simple_chain() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value(
            "a",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(10.0))],
        )
        .compute("b", "a * 2")
        .unwrap()
        .compute("c", "b + 5")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let q1 = PeriodId::quarter(2025, 1);
    assert_eq!(results.get("a", &q1), Some(10.0));
    assert_eq!(results.get("b", &q1), Some(20.0));
    assert_eq!(results.get("c", &q1), Some(25.0));
}

#[test]
fn test_dag_multiple_dependencies() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value(
            "revenue",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0))],
        )
        .value(
            "opex",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(20.0))],
        )
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .compute("operating_income", "gross_profit - opex")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let q1 = PeriodId::quarter(2025, 1);
    assert_eq!(results.get("revenue", &q1), Some(100.0));
    assert_eq!(results.get("cogs", &q1), Some(60.0));
    assert_eq!(results.get("gross_profit", &q1), Some(40.0));
    assert_eq!(results.get("opex", &q1), Some(20.0));
    assert_eq!(results.get("operating_income", &q1), Some(20.0));
}

#[test]
fn test_circular_dependency_detection() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .compute("a", "b + 1")
        .unwrap()
        .compute("b", "c + 1")
        .unwrap()
        .compute("c", "a + 1")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let result = evaluator.evaluate(&model, false);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, Error::CircularDependency { .. }));
}

#[test]
fn test_self_reference_cycle() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .compute("a", "a + 1")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let result = evaluator.evaluate(&model, false);

    assert!(result.is_err());
}

// ============================================================================
// PR #3.4 — Precedence Resolution Tests
// ============================================================================

#[test]
fn test_precedence_value_over_formula() {
    use finstack_statements::evaluator::resolve_node_value;
    use finstack_statements::types::NodeSpec;
    use indexmap::IndexMap;

    let mut values = IndexMap::new();
    values.insert(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0));

    let node = NodeSpec::new("revenue", NodeType::Mixed)
        .with_values(values)
        .with_formula("999999"); // Should be ignored

    let source = resolve_node_value(&node, &PeriodId::quarter(2025, 1), true).unwrap();

    // Should use explicit value, not formula
    assert!(source.is_value());
    assert_eq!(source.as_value(), Some(100.0));
}

#[test]
fn test_precedence_formula_fallback() {
    use finstack_statements::evaluator::resolve_node_value;
    use finstack_statements::types::NodeSpec;

    let node = NodeSpec::new("cogs", NodeType::Calculated).with_formula("revenue * 0.6");

    let source = resolve_node_value(&node, &PeriodId::quarter(2025, 1), true).unwrap();

    // Should use formula
    assert!(source.is_formula());
    assert_eq!(source.as_formula(), Some("revenue * 0.6"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complete_pl_model() {
    let model = ModelBuilder::new("P&L Model")
        .periods("2025Q1..2025Q2", None)
        .unwrap() // Just Q1-Q2 for Phase 3 (no forecasts yet)
        // Revenue
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
        // COGS as 60% of revenue
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        // Operating expenses
        .value(
            "opex",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(2_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(2_100_000.0),
                ),
            ],
        )
        // Derived metrics
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .compute("operating_income", "gross_profit - opex")
        .unwrap()
        .compute("gross_margin", "gross_profit / revenue")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    // Check Q1 results
    let q1 = PeriodId::quarter(2025, 1);
    assert_eq!(results.get("revenue", &q1), Some(10_000_000.0));
    assert_eq!(results.get("cogs", &q1), Some(6_000_000.0));
    assert_eq!(results.get("gross_profit", &q1), Some(4_000_000.0));
    assert_eq!(results.get("opex", &q1), Some(2_000_000.0));
    assert_eq!(results.get("operating_income", &q1), Some(2_000_000.0));
    assert!((results.get("gross_margin", &q1).unwrap() - 0.4).abs() < 0.001);

    // Check Q2 results
    let q2 = PeriodId::quarter(2025, 2);
    assert_eq!(results.get("revenue", &q2), Some(11_000_000.0));
    assert_eq!(results.get("cogs", &q2), Some(6_600_000.0));
    assert_eq!(results.get("gross_profit", &q2), Some(4_400_000.0));
    assert_eq!(results.get("opex", &q2), Some(2_100_000.0));
    assert_eq!(results.get("operating_income", &q2), Some(2_300_000.0));
    assert!((results.get("gross_margin", &q2).unwrap() - 0.4).abs() < 0.001);

    // Check metadata
    assert_eq!(results.meta.num_nodes, 6);
    assert_eq!(results.meta.num_periods, 2);
    assert!(!results.meta.parallel);
    assert!(results.meta.eval_time_ms.is_some());
}

#[test]
fn test_multiple_periods_sequential() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..2025Q4", None)
        .unwrap()
        .value(
            "base",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
                (PeriodId::quarter(2025, 3), AmountOrScalar::scalar(121.0)),
                (PeriodId::quarter(2025, 4), AmountOrScalar::scalar(133.1)),
            ],
        )
        .compute("doubled", "base * 2")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    assert_eq!(
        results.get("doubled", &PeriodId::quarter(2025, 1)),
        Some(200.0)
    );
    assert_eq!(
        results.get("doubled", &PeriodId::quarter(2025, 2)),
        Some(220.0)
    );
    assert_eq!(
        results.get("doubled", &PeriodId::quarter(2025, 3)),
        Some(242.0)
    );
    assert!((results.get("doubled", &PeriodId::quarter(2025, 4)).unwrap() - 266.2).abs() < 0.01);
}

#[test]
fn test_nested_parentheses() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q1", None)
        .unwrap()
        .value(
            "revenue",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0))],
        )
        .value(
            "cogs",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(60.0))],
        )
        .value(
            "opex",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(20.0))],
        )
        .compute("result", "((revenue - cogs) - opex) / revenue")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    let expected = ((100.0 - 60.0) - 20.0) / 100.0; // 0.2
    let actual = results.get("result", &PeriodId::quarter(2025, 1)).unwrap();
    assert!((actual - expected).abs() < 0.001);
}

#[test]
fn test_results_metadata() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "a",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(1.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(2.0)),
            ],
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, false).unwrap();

    assert_eq!(results.meta.num_nodes, 1);
    assert_eq!(results.meta.num_periods, 2);
    assert!(!results.meta.parallel);
    assert!(results.meta.eval_time_ms.is_some());
    assert!(results.meta.eval_time_ms.unwrap() < 1000); // Should be fast
}
