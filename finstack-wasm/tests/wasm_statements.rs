//! wasm-bindgen-test suite for `api::statements`.
//!
//! Covers model_node_ids which returns JsValue.

#![cfg(target_arch = "wasm32")]

use finstack_wasm::api::statements::*;
use wasm_bindgen_test::*;

fn model_with_nodes(nodes: &[&str]) -> String {
    use finstack_core::dates::PeriodId;
    use finstack_statements::builder::ModelBuilder;
    use finstack_statements::types::AmountOrScalar;

    let q1 = PeriodId::quarter(2024, 1);
    let mut builder = ModelBuilder::new("test")
        .periods("2024Q1..Q1", None)
        .unwrap();
    for &name in nodes {
        builder = builder.value(name, &[(q1, AmountOrScalar::scalar(100.0))]);
    }
    let model = builder.build().unwrap();
    serde_json::to_string(&model).unwrap()
}

#[wasm_bindgen_test]
fn model_node_ids_returns_array() {
    let json = model_with_nodes(&["revenue"]);
    let result = model_node_ids(&json).unwrap();
    let ids: Vec<String> = serde_wasm_bindgen::from_value(result).unwrap();
    assert_eq!(ids, vec!["revenue"]);
}

#[wasm_bindgen_test]
fn model_node_ids_empty_model() {
    let model = finstack_statements::FinancialModelSpec::new("empty", vec![]);
    let json = serde_json::to_string(&model).unwrap();
    let result = model_node_ids(&json).unwrap();
    let ids: Vec<String> = serde_wasm_bindgen::from_value(result).unwrap();
    assert!(ids.is_empty());
}

#[wasm_bindgen_test]
fn model_node_ids_multiple_nodes() {
    let json = model_with_nodes(&["revenue", "cogs", "gp"]);
    let result = model_node_ids(&json).unwrap();
    let ids: Vec<String> = serde_wasm_bindgen::from_value(result).unwrap();
    assert_eq!(ids.len(), 3);
}
