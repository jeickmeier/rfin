use crate::core::money::JsMoney;
use crate::statements::types::JsFinancialModelSpec;
use finstack_statements::analysis::corporate::{evaluate_dcf_with_market, DcfOptions};
use finstack_valuations::instruments::equity::dcf_equity::{TerminalValueSpec, ValuationDiscounts};
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Evaluate a corporate DCF using a financial model specification.
///
/// Parameters:
/// - `model` - Financial model specification
/// - `wacc` - Weighted average cost of capital (decimal)
/// - `terminal_growth` - Perpetual growth rate for Gordon Growth (decimal)
/// - `ufcf_node` - Node ID for unlevered free cash flow (default: "ufcf")
/// - `net_debt_override` - Optional fixed net debt value
/// - `mid_year_convention` - Enable mid-year discounting (default: false)
/// - `terminal_type` - Terminal value method: "gordon_growth" (default), "exit_multiple", or "h_model"
/// - `terminal_metric` - Terminal metric value for exit multiple (required when terminal_type="exit_multiple")
/// - `terminal_multiple` - Exit multiple (required when terminal_type="exit_multiple")
/// - `high_growth_rate` - H-model initial high growth rate (required when terminal_type="h_model")
/// - `stable_growth_rate` - H-model stable growth rate (required when terminal_type="h_model")
/// - `half_life_years` - H-model half-life of growth fade (required when terminal_type="h_model")
/// - `shares_outstanding` - Optional shares outstanding for per-share value
/// - `dlom` - Optional Discount for Lack of Marketability (0.0-1.0)
/// - `dloc` - Optional Discount for Lack of Control (0.0-1.0)
///
/// Returns a JS object with `equity_value`, `enterprise_value`, `net_debt`,
/// `terminal_value_pv` as Money objects, and optionally `equity_value_per_share`
/// and `diluted_shares` as numbers.
#[wasm_bindgen(js_name = evaluateDcf)]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_dcf_wasm(
    model: &JsFinancialModelSpec,
    wacc: f64,
    terminal_growth: f64,
    ufcf_node: Option<String>,
    net_debt_override: Option<f64>,
    mid_year_convention: Option<bool>,
    terminal_type: Option<String>,
    terminal_metric: Option<f64>,
    terminal_multiple: Option<f64>,
    high_growth_rate: Option<f64>,
    stable_growth_rate: Option<f64>,
    half_life_years: Option<f64>,
    shares_outstanding: Option<f64>,
    dlom: Option<f64>,
    dloc: Option<f64>,
) -> Result<JsValue, JsValue> {
    let node_id = ufcf_node.unwrap_or_else(|| "ufcf".to_string());
    let terminal_spec = match terminal_type.as_deref().unwrap_or("gordon_growth") {
        "gordon_growth" => TerminalValueSpec::GordonGrowth {
            growth_rate: terminal_growth,
        },
        "exit_multiple" => {
            let metric = terminal_metric.ok_or_else(|| {
                JsValue::from_str("terminal_metric is required when terminal_type='exit_multiple'")
            })?;
            let mult = terminal_multiple.ok_or_else(|| {
                JsValue::from_str(
                    "terminal_multiple is required when terminal_type='exit_multiple'",
                )
            })?;
            TerminalValueSpec::ExitMultiple {
                terminal_metric: metric,
                multiple: mult,
            }
        }
        "h_model" => {
            let hgr = high_growth_rate.ok_or_else(|| {
                JsValue::from_str("high_growth_rate is required when terminal_type='h_model'")
            })?;
            let sgr = stable_growth_rate.ok_or_else(|| {
                JsValue::from_str("stable_growth_rate is required when terminal_type='h_model'")
            })?;
            let hl = half_life_years.ok_or_else(|| {
                JsValue::from_str("half_life_years is required when terminal_type='h_model'")
            })?;
            TerminalValueSpec::HModel {
                high_growth_rate: hgr,
                stable_growth_rate: sgr,
                half_life_years: hl,
            }
        }
        other => {
            return Err(JsValue::from_str(&format!(
                "Unknown terminal_type '{}'. Expected 'gordon_growth', 'exit_multiple', or 'h_model'.",
                other
            )));
        }
    };

    let valuation_discounts = if dlom.is_some() || dloc.is_some() {
        Some(ValuationDiscounts {
            dlom,
            dloc,
            other_discount: None,
        })
    } else {
        None
    };

    let options = DcfOptions {
        mid_year_convention: mid_year_convention.unwrap_or(false),
        equity_bridge: None,
        shares_outstanding,
        valuation_discounts,
    };

    let result = evaluate_dcf_with_market(
        &model.inner,
        wacc,
        terminal_spec,
        &node_id,
        net_debt_override,
        &options,
        None,
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

    if let Some(eps) = result.equity_value_per_share {
        Reflect::set(
            &output,
            &JsValue::from_str("equity_value_per_share"),
            &JsValue::from_f64(eps),
        )?;
    }
    if let Some(ds) = result.diluted_shares {
        Reflect::set(
            &output,
            &JsValue::from_str("diluted_shares"),
            &JsValue::from_f64(ds),
        )?;
    }

    Ok(output.into())
}
