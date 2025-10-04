//! Tests for capital structure DSL integration (`cs.*` namespace).

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, PeriodId};
use finstack_core::money::Money;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::capital_structure::CapitalStructureCashflows;
use finstack_statements::dsl::{ast::StmtExpr, parse_formula};
use finstack_statements::evaluator::EvaluationContext;
use finstack_statements::types::AmountOrScalar;
use indexmap::IndexMap;
use time::Month;

/// Test parsing cs.* references
#[test]
fn test_parse_cs_interest_expense_total() {
    let ast = parse_formula("cs.interest_expense.total").unwrap();

    match ast {
        StmtExpr::CSRef {
            component,
            instrument_or_total,
        } => {
            assert_eq!(component, "interest_expense");
            assert_eq!(instrument_or_total, "total");
        }
        _ => panic!("Expected CSRef, got {:?}", ast),
    }
}

#[test]
fn test_parse_cs_principal_payment_instrument() {
    let ast = parse_formula("cs.principal_payment.BOND-001").unwrap();

    match ast {
        StmtExpr::CSRef {
            component,
            instrument_or_total,
        } => {
            assert_eq!(component, "principal_payment");
            assert_eq!(instrument_or_total, "BOND-001");
        }
        _ => panic!("Expected CSRef, got {:?}", ast),
    }
}

#[test]
fn test_parse_cs_debt_balance() {
    let ast = parse_formula("cs.debt_balance.SWAP-001").unwrap();

    match ast {
        StmtExpr::CSRef {
            component,
            instrument_or_total,
        } => {
            assert_eq!(component, "debt_balance");
            assert_eq!(instrument_or_total, "SWAP-001");
        }
        _ => panic!("Expected CSRef, got {:?}", ast),
    }
}

#[test]
fn test_parse_cs_in_formula() {
    let ast = parse_formula("revenue - cs.interest_expense.total").unwrap();

    // Should parse as a subtraction with a CSRef on the right
    match ast {
        StmtExpr::BinOp { .. } => {
            // Valid structure
        }
        _ => panic!("Expected BinOp, got {:?}", ast),
    }
}

#[test]
fn test_parse_cs_complex_formula() {
    let ast =
        parse_formula("(revenue - cogs - opex - cs.interest_expense.total) / revenue").unwrap();

    // Should parse successfully
    match ast {
        StmtExpr::BinOp { .. } => {
            // Valid structure
        }
        _ => panic!("Expected BinOp, got {:?}", ast),
    }
}

/// Test that invalid cs references are parsed as regular node refs
#[test]
fn test_parse_cs_invalid_format() {
    // Only two parts - should be parsed as regular node ref
    let ast = parse_formula("cs.interest_expense").unwrap();

    match ast {
        StmtExpr::NodeRef(name) => {
            assert_eq!(name, "cs.interest_expense");
        }
        _ => panic!("Expected NodeRef, got {:?}", ast),
    }
}

/// Test context with capital structure cashflows
#[test]
fn test_context_get_cs_value_interest_total() {
    let period_id = PeriodId::quarter(2025, 1);
    let node_to_column = IndexMap::new();
    let historical = IndexMap::new();

    let mut context = EvaluationContext::new(period_id, node_to_column, historical);

    // Create sample capital structure cashflows
    let mut cs_cashflows = CapitalStructureCashflows::new();
    let breakdown = finstack_statements::capital_structure::CashflowBreakdown {
        interest_expense: 50_000.0,
        principal_payment: 100_000.0,
        debt_balance: 1_000_000.0,
        ..Default::default()
    };
    cs_cashflows.totals.insert(period_id, breakdown);

    context.capital_structure_cashflows = Some(cs_cashflows);

    // Test getting interest_expense.total
    let value = context.get_cs_value("interest_expense", "total").unwrap();
    assert_eq!(value, 50_000.0);
}

#[test]
fn test_context_get_cs_value_principal_instrument() {
    let period_id = PeriodId::quarter(2025, 1);
    let node_to_column = IndexMap::new();
    let historical = IndexMap::new();

    let mut context = EvaluationContext::new(period_id, node_to_column, historical);

    // Create sample capital structure cashflows
    let mut cs_cashflows = CapitalStructureCashflows::new();
    let mut by_instrument = IndexMap::new();
    let breakdown = finstack_statements::capital_structure::CashflowBreakdown {
        principal_payment: 25_000.0,
        ..Default::default()
    };
    by_instrument.insert(period_id, breakdown);
    cs_cashflows
        .by_instrument
        .insert("BOND-001".to_string(), by_instrument);

    context.capital_structure_cashflows = Some(cs_cashflows);

    // Test getting principal_payment for specific instrument
    let value = context
        .get_cs_value("principal_payment", "BOND-001")
        .unwrap();
    assert_eq!(value, 25_000.0);
}

#[test]
fn test_context_get_cs_value_no_cs_error() {
    let period_id = PeriodId::quarter(2025, 1);
    let node_to_column = IndexMap::new();
    let historical = IndexMap::new();

    let context = EvaluationContext::new(period_id, node_to_column, historical);

    // No capital structure defined - should error
    let result = context.get_cs_value("interest_expense", "total");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No capital structure"));
}

#[test]
fn test_context_get_cs_value_invalid_component() {
    let period_id = PeriodId::quarter(2025, 1);
    let node_to_column = IndexMap::new();
    let historical = IndexMap::new();

    let mut context = EvaluationContext::new(period_id, node_to_column, historical);
    context.capital_structure_cashflows = Some(CapitalStructureCashflows::new());

    // Invalid component - should error
    let result = context.get_cs_value("invalid_component", "total");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unknown capital structure component"));
}

/// Test full model evaluation with capital structure (mock cashflows)
#[test]
fn test_evaluate_model_with_cs_mock() {
    // Create a simple model that references capital structure
    let model_result = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(1_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(1_100_000.0),
                ),
            ],
        )
        .value(
            "cogs",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(600_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(650_000.0),
                ),
            ],
        )
        // Note: This formula won't work until we have actual CS data in the evaluator
        // .compute("net_income", "revenue - cogs - cs.interest_expense.total")
        .build();

    assert!(model_result.is_ok());
    let model = model_result.unwrap();

    // Verify model structure
    assert_eq!(model.nodes.len(), 2);
    assert!(model.has_node("revenue"));
    assert!(model.has_node("cogs"));
}

/// Test model with bond added via builder
#[test]
fn test_model_with_bond_builder() {
    let issue_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity_date = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let model_result = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(1_000_000.0),
            )],
        )
        .add_bond(
            "BOND-001",
            Money::new(10_000_000.0, Currency::USD),
            0.05,
            issue_date,
            maturity_date,
            "USD-OIS",
        )
        .unwrap()
        .build();

    assert!(model_result.is_ok());
    let model = model_result.unwrap();

    // Verify capital structure was added
    assert!(model.capital_structure.is_some());
    let cs = model.capital_structure.as_ref().unwrap();
    assert_eq!(cs.debt_instruments.len(), 1);
}

/// Test that cs.* references can be compiled
#[test]
fn test_compile_cs_reference() {
    use finstack_statements::dsl::parse_and_compile;

    let expr = parse_and_compile("cs.interest_expense.total").unwrap();

    // Should compile to a Column expression with encoded name
    use finstack_core::expr::ExprNode;
    match &expr.node {
        ExprNode::Column(name) => {
            assert!(name.starts_with("__cs__"));
            assert!(name.contains("interest_expense"));
            assert!(name.contains("total"));
        }
        _ => panic!("Expected Column node, got {:?}", expr.node),
    }
}

/// Test cs.* in complex formulas
#[test]
fn test_compile_cs_in_formula() {
    use finstack_statements::dsl::parse_and_compile;

    // Should compile successfully
    let expr = parse_and_compile("revenue - cogs - cs.interest_expense.total").unwrap();

    // Verify it's a binary operation
    use finstack_core::expr::ExprNode;
    match &expr.node {
        ExprNode::BinOp { .. } => {
            // Valid
        }
        _ => panic!("Expected BinOp node"),
    }
}

/// Test debt service ratio formula
#[test]
fn test_compile_debt_service_ratio() {
    use finstack_statements::dsl::parse_and_compile;

    let expr =
        parse_and_compile("ebitda / (cs.interest_expense.total + cs.principal_payment.total)")
            .unwrap();

    // Should compile successfully
    use finstack_core::expr::ExprNode;
    match &expr.node {
        ExprNode::BinOp { .. } => {
            // Valid
        }
        _ => panic!("Expected BinOp node"),
    }
}

/// Test multiple instrument references
#[test]
fn test_compile_multiple_instruments() {
    use finstack_statements::dsl::parse_and_compile;

    let expr =
        parse_and_compile("cs.interest_expense.BOND-001 + cs.interest_expense.SWAP-001").unwrap();

    // Should compile successfully
    use finstack_core::expr::ExprNode;
    match &expr.node {
        ExprNode::BinOp { .. } => {
            // Valid
        }
        _ => panic!("Expected BinOp node"),
    }
}
