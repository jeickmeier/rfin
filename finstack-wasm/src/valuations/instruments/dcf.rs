use crate::core::money::JsMoney;
use crate::statements::types::JsFinancialModelSpec;
use finstack_statements::analysis::corporate::evaluate_dcf;
use finstack_valuations::instruments::equity::dcf_equity::TerminalValueSpec;
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Evaluate a corporate DCF using a financial model specification.
///
/// This is a thin wrapper that converts JS types to Rust, delegates to the
/// core DCF engine, and returns Money results back to JS.
#[wasm_bindgen(js_name = evaluateDcf)]
pub fn evaluate_dcf_wasm(
    model: &JsFinancialModelSpec,
    wacc: f64,
    terminal_growth: f64,
    ufcf_node: Option<String>,
    net_debt_override: Option<f64>,
) -> Result<JsValue, JsValue> {
    let node_id = ufcf_node.unwrap_or_else(|| "ufcf".to_string());
    let terminal_spec = TerminalValueSpec::GordonGrowth {
        growth_rate: terminal_growth,
    };

    let result = evaluate_dcf(
        &model.inner,
        wacc,
        terminal_spec,
        &node_id,
        net_debt_override,
    )
    .map_err(|e| JsValue::from_str(&format!("DCF evaluation failed: {}", e)))?;

    let output = Object::new();
    Reflect::set(
        &output,
        &JsValue::from_str("equity_value"),
        &JsValue::from(JsMoney::from_inner(result.equity_value)),
    )?;
    Reflect::set(
        &output,
        &JsValue::from_str("enterprise_value"),
        &JsValue::from(JsMoney::from_inner(result.enterprise_value)),
    )?;
    Reflect::set(
        &output,
        &JsValue::from_str("net_debt"),
        &JsValue::from(JsMoney::from_inner(result.net_debt)),
    )?;
    Reflect::set(
        &output,
        &JsValue::from_str("terminal_value_pv"),
        &JsValue::from(JsMoney::from_inner(result.terminal_value_pv)),
    )?;

    Ok(output.into())
}
