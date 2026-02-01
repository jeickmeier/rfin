//! ModelBuilder integration tests.
#![allow(clippy::expect_used)]

use finstack_core::dates::PeriodId;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::types::{AmountOrScalar, NodeType};

#[test]
fn test_builder_type_state() {
    let result = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .expect("valid period range")
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_periods_validation() {
    let _result = ModelBuilder::new("test").periods("2025Q1..Q1", None);

    let empty_result = ModelBuilder::new("test").periods_explicit(vec![]);
    assert!(empty_result.is_err());
}

#[test]
fn test_value_node() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .expect("valid period range")
        .value(
            "revenue",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
            ],
        )
        .build()
        .expect("valid model");

    assert_eq!(model.nodes.len(), 1);
    assert!(model.has_node("revenue"));
    assert_eq!(
        model
            .get_node("revenue")
            .expect("revenue node should exist")
            .node_type,
        NodeType::Value
    );
}

#[test]
fn test_computed_node() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .expect("valid period range")
        .compute("gross_profit", "revenue - cogs")
        .expect("valid formula")
        .build()
        .expect("valid model");

    assert_eq!(model.nodes.len(), 1);
    let node = model
        .get_node("gross_profit")
        .expect("gross_profit node should exist");
    assert_eq!(node.node_type, NodeType::Calculated);
    assert_eq!(
        node.formula_text
            .as_ref()
            .expect("formula_text should exist"),
        "revenue - cogs"
    );
}

#[test]
fn test_empty_formula_error() {
    let result = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .expect("valid period range")
        .compute("invalid", "");

    assert!(result.is_err());
}

#[test]
fn test_multiple_nodes() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q2"))
        .expect("valid period range")
        .value(
            "revenue",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110.0)),
            ],
        )
        .compute("cogs", "revenue * 0.6")
        .expect("valid formula")
        .compute("gross_profit", "revenue - cogs")
        .expect("valid formula")
        .build()
        .expect("valid model");

    assert_eq!(model.nodes.len(), 3);
    assert!(model.has_node("revenue"));
    assert!(model.has_node("cogs"));
    assert!(model.has_node("gross_profit"));

    assert_eq!(model.periods.len(), 4);
    assert!(model.periods[0].is_actual);
    assert!(model.periods[1].is_actual);
    assert!(!model.periods[2].is_actual);
    assert!(!model.periods[3].is_actual);
}
