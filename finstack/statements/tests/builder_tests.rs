//! Builder tests for Phase 1 (PR #1.1, #1.2, #1.3)

use finstack_statements::prelude::*;

// ============================================================================
// PR #1.1 — Crate Bootstrap Tests
// ============================================================================

#[test]
fn test_builder_creation() {
    let builder = ModelBuilder::new("test_model");
    // Type-state ensures we can't call .build() yet
    // This test just verifies construction works
    let _ = builder;
}

// ============================================================================
// PR #1.2 — Period Integration Tests
// ============================================================================

#[test]
fn test_periods_parsing() {
    let result = ModelBuilder::new("test").periods("2025Q1..Q4", None);
    assert!(result.is_ok());

    let builder = result.unwrap();
    let model = builder.build().unwrap();

    assert_eq!(model.periods.len(), 4);
    assert_eq!(model.periods[0].id, PeriodId::quarter(2025, 1));
    assert_eq!(model.periods[3].id, PeriodId::quarter(2025, 4));
}

#[test]
fn test_periods_with_actuals_cutoff() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q2"))
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(model.periods.len(), 4);

    // Q1 and Q2 should be actuals
    assert!(model.periods[0].is_actual);
    assert!(model.periods[1].is_actual);

    // Q3 and Q4 should be forecast
    assert!(!model.periods[2].is_actual);
    assert!(!model.periods[3].is_actual);
}

#[test]
fn test_periods_explicit() {
    let periods = build_periods("2025Q1..Q2", None).unwrap().periods;
    let model = ModelBuilder::new("test")
        .periods_explicit(periods.clone())
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(model.periods.len(), 2);
}

#[test]
fn test_empty_periods_error() {
    let result = ModelBuilder::new("test").periods_explicit(vec![]);
    assert!(result.is_err());
}

#[test]
fn test_model_serialization() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .build()
        .unwrap();

    // Test JSON serialization roundtrip
    let json = serde_json::to_string(&model).unwrap();
    let deserialized: FinancialModelSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(model.id, deserialized.id);
    assert_eq!(model.periods.len(), deserialized.periods.len());
}

// ============================================================================
// PR #1.3 — Value Node Tests
// ============================================================================

#[test]
fn test_value_node_single_period() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0))],
        )
        .build()
        .unwrap();

    assert_eq!(model.nodes.len(), 1);
    assert!(model.has_node("revenue"));

    let node = model.get_node("revenue").unwrap();
    assert_eq!(node.node_type, NodeType::Value);
    assert!(node.values.is_some());

    let values = node.values.as_ref().unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(
        values[&PeriodId::quarter(2025, 1)].value(),
        100_000.0
    );
}

#[test]
fn test_value_node_multiple_periods() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", None)
        .unwrap()
        .value(
            "revenue",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110_000.0)),
                (PeriodId::quarter(2025, 3), AmountOrScalar::scalar(121_000.0)),
            ],
        )
        .build()
        .unwrap();

    let node = model.get_node("revenue").unwrap();
    let values = node.values.as_ref().unwrap();

    assert_eq!(values.len(), 3);
    assert_eq!(values[&PeriodId::quarter(2025, 1)].value(), 100_000.0);
    assert_eq!(values[&PeriodId::quarter(2025, 2)].value(), 110_000.0);
    assert_eq!(values[&PeriodId::quarter(2025, 3)].value(), 121_000.0);
}

#[test]
fn test_value_node_with_currency() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::amount(1_000_000.0, Currency::USD),
            )],
        )
        .build()
        .unwrap();

    let node = model.get_node("revenue").unwrap();
    let values = node.values.as_ref().unwrap();
    let amount = &values[&PeriodId::quarter(2025, 1)];

    assert!(amount.is_amount());
    assert_eq!(amount.currency(), Some(Currency::USD));
    assert_eq!(amount.value(), 1_000_000.0);
}

#[test]
fn test_calculated_node() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(model.nodes.len(), 1);
    let node = model.get_node("gross_profit").unwrap();

    assert_eq!(node.node_type, NodeType::Calculated);
    assert_eq!(node.formula_text.as_ref().unwrap(), "revenue - cogs");
    assert!(node.values.is_none());
}

#[test]
fn test_empty_formula_error() {
    let result = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .compute("invalid", "");

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::FormulaParse(_)));
}

#[test]
fn test_whitespace_only_formula_error() {
    let result = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .compute("invalid", "   ");

    assert!(result.is_err());
}

#[test]
fn test_multiple_nodes() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q2"))
        .unwrap()
        .value(
            "revenue",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110_000.0)),
            ],
        )
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(model.nodes.len(), 3);
    assert!(model.has_node("revenue"));
    assert!(model.has_node("cogs"));
    assert!(model.has_node("gross_profit"));
}

#[test]
fn test_metadata() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .with_meta("author", serde_json::json!("Test User"))
        .with_meta("version", serde_json::json!("1.0.0"))
        .build()
        .unwrap();

    assert_eq!(model.meta.len(), 2);
    assert_eq!(model.meta["author"], "Test User");
    assert_eq!(model.meta["version"], "1.0.0");
}

#[test]
fn test_schema_version() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(model.schema_version, 1);
}

// ============================================================================
// Complex Integration Tests
// ============================================================================

#[test]
fn test_basic_pl_model() {
    let model = ModelBuilder::new("Simple P&L")
        .periods("2025Q1..2025Q4", Some("2025Q2"))
        .unwrap()
        // Revenue
        .value(
            "revenue",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(10_000_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(11_000_000.0)),
            ],
        )
        // COGS as 60% of revenue
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        // Operating expenses
        .value(
            "opex",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(2_000_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(2_100_000.0)),
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

    assert_eq!(model.id, "Simple P&L");
    assert_eq!(model.periods.len(), 4);
    assert_eq!(model.nodes.len(), 6);

    // Verify actuals periods
    assert!(model.periods[0].is_actual);
    assert!(model.periods[1].is_actual);
    assert!(!model.periods[2].is_actual);
    assert!(!model.periods[3].is_actual);

    // Verify node types
    assert_eq!(
        model.get_node("revenue").unwrap().node_type,
        NodeType::Value
    );
    assert_eq!(
        model.get_node("cogs").unwrap().node_type,
        NodeType::Calculated
    );
    assert_eq!(
        model.get_node("gross_profit").unwrap().node_type,
        NodeType::Calculated
    );
}

#[test]
fn test_model_with_multiple_currencies() {
    let model = ModelBuilder::new("Multi-currency")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "usd_revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::amount(1_000_000.0, Currency::USD),
            )],
        )
        .value(
            "eur_revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::amount(900_000.0, Currency::EUR),
            )],
        )
        .build()
        .unwrap();

    let usd_node = model.get_node("usd_revenue").unwrap();
    let eur_node = model.get_node("eur_revenue").unwrap();

    let usd_values = usd_node.values.as_ref().unwrap();
    let eur_values = eur_node.values.as_ref().unwrap();

    assert_eq!(
        usd_values[&PeriodId::quarter(2025, 1)].currency(),
        Some(Currency::USD)
    );
    assert_eq!(
        eur_values[&PeriodId::quarter(2025, 1)].currency(),
        Some(Currency::EUR)
    );
}

