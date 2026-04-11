//! WASM bindings for the `finstack-scenarios` crate.
//!
//! Exposes scenario specification parsing, validation, composition,
//! and built-in template access via JSON round-trip functions.

use crate::utils::to_js_err;
use wasm_bindgen::prelude::*;

/// Parse and validate a scenario specification from JSON.
///
/// Returns the validated, re-serialized JSON.
#[wasm_bindgen(js_name = parseScenarioSpec)]
pub fn parse_scenario_spec(json_str: &str) -> Result<String, JsValue> {
    let spec: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(json_str).map_err(to_js_err)?;

    spec.validate().map_err(to_js_err)?;

    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Compose multiple scenario specs (JSON array) into a single scenario.
///
/// Specs are merged in priority order (lower number runs first).
#[wasm_bindgen(js_name = composeScenarios)]
pub fn compose_scenarios(specs_json: &str) -> Result<String, JsValue> {
    let specs: Vec<finstack_scenarios::ScenarioSpec> =
        serde_json::from_str(specs_json).map_err(to_js_err)?;

    let engine = finstack_scenarios::ScenarioEngine::new();
    let composed = engine.compose(specs);

    serde_json::to_string(&composed).map_err(to_js_err)
}

/// Validate a scenario specification JSON without executing it.
///
/// Returns `true` if valid, throws on error.
#[wasm_bindgen(js_name = validateScenarioSpec)]
pub fn validate_scenario_spec(json_str: &str) -> Result<bool, JsValue> {
    let spec: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(json_str).map_err(to_js_err)?;

    spec.validate().map_err(to_js_err)?;
    Ok(true)
}

/// List all built-in template identifiers.
///
/// Returns a JSON array of template ID strings.
#[wasm_bindgen(js_name = listBuiltinTemplates)]
pub fn list_builtin_templates() -> Result<JsValue, JsValue> {
    let registry =
        finstack_scenarios::TemplateRegistry::with_embedded_builtins().map_err(to_js_err)?;

    let ids: Vec<String> = registry.list().iter().map(|m| m.id.clone()).collect();
    serde_wasm_bindgen::to_value(&ids).map_err(to_js_err)
}

/// Get metadata for all built-in templates as a JSON string.
#[wasm_bindgen(js_name = listBuiltinTemplateMetadata)]
pub fn list_builtin_template_metadata() -> Result<String, JsValue> {
    let registry =
        finstack_scenarios::TemplateRegistry::with_embedded_builtins().map_err(to_js_err)?;

    let metadata: Vec<&finstack_scenarios::TemplateMetadata> = registry.list();
    serde_json::to_string(&metadata).map_err(to_js_err)
}

/// Build a scenario spec from a built-in template.
///
/// Returns JSON-serialized `ScenarioSpec`.
#[wasm_bindgen(js_name = buildFromTemplate)]
pub fn build_from_template(template_id: &str) -> Result<String, JsValue> {
    let registry =
        finstack_scenarios::TemplateRegistry::with_embedded_builtins().map_err(to_js_err)?;

    let entry = registry
        .get(template_id)
        .ok_or_else(|| to_js_err(format!("Unknown template: '{template_id}'")))?;

    let spec = entry.builder().build().map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// List component IDs for a built-in composite template.
///
/// Returns a JS array of component ID strings.
#[wasm_bindgen(js_name = listTemplateComponents)]
pub fn list_template_components(template_id: &str) -> Result<JsValue, JsValue> {
    let registry =
        finstack_scenarios::TemplateRegistry::with_embedded_builtins().map_err(to_js_err)?;

    let entry = registry
        .get(template_id)
        .ok_or_else(|| to_js_err(format!("Unknown template: '{template_id}'")))?;

    let ids: Vec<String> = entry
        .component_ids()
        .into_iter()
        .map(str::to_string)
        .collect();
    serde_wasm_bindgen::to_value(&ids).map_err(to_js_err)
}

/// Build a specific component from a built-in composite template.
#[wasm_bindgen(js_name = buildTemplateComponent)]
pub fn build_template_component(template_id: &str, component_id: &str) -> Result<String, JsValue> {
    let registry =
        finstack_scenarios::TemplateRegistry::with_embedded_builtins().map_err(to_js_err)?;
    let entry = registry
        .get(template_id)
        .ok_or_else(|| to_js_err(format!("Unknown template: '{template_id}'")))?;
    let builder = entry.component(component_id).ok_or_else(|| {
        to_js_err(format!(
            "Unknown component '{component_id}' in template '{template_id}'"
        ))
    })?;
    let spec = builder.build().map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Build a scenario spec from fields.
#[wasm_bindgen(js_name = buildScenarioSpec)]
pub fn build_scenario_spec(
    id: &str,
    operations_json: &str,
    name: Option<String>,
    description: Option<String>,
    priority: i32,
) -> Result<String, JsValue> {
    let operations: Vec<finstack_scenarios::OperationSpec> =
        serde_json::from_str(operations_json).map_err(to_js_err)?;
    let spec = finstack_scenarios::ScenarioSpec {
        id: id.to_string(),
        name,
        description,
        operations,
        priority,
        resolution_mode: Default::default(),
    };
    spec.validate().map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Apply a scenario to a market context and financial model.
///
/// Returns a JSON object with `market_json`, `model_json`,
/// `operations_applied`, and `warnings`.
#[wasm_bindgen(js_name = applyScenario)]
pub fn apply_scenario(
    scenario_json: &str,
    market_json: &str,
    model_json: &str,
    as_of: &str,
) -> Result<JsValue, JsValue> {
    let spec: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(to_js_err)?;
    let mut market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let mut model: finstack_statements::FinancialModelSpec =
        serde_json::from_str(model_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;

    let engine = finstack_scenarios::ScenarioEngine::new();
    let mut ctx = finstack_scenarios::ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: date,
    };
    let report = engine.apply(&spec, &mut ctx).map_err(to_js_err)?;

    let out = serde_json::json!({
        "market_json": serde_json::to_string(&market).map_err(to_js_err)?,
        "model_json": serde_json::to_string(&model).map_err(to_js_err)?,
        "operations_applied": report.operations_applied,
        "warnings": report.warnings,
    });
    serde_wasm_bindgen::to_value(&out).map_err(to_js_err)
}

/// Apply a scenario to a market context only (no model mutations).
#[wasm_bindgen(js_name = applyScenarioToMarket)]
pub fn apply_scenario_to_market(
    scenario_json: &str,
    market_json: &str,
    as_of: &str,
) -> Result<JsValue, JsValue> {
    let spec: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(to_js_err)?;
    let mut market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let mut model = finstack_statements::FinancialModelSpec::new("__scenario_temp__", vec![]);
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;

    let engine = finstack_scenarios::ScenarioEngine::new();
    let mut ctx = finstack_scenarios::ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: date,
    };
    let report = engine.apply(&spec, &mut ctx).map_err(to_js_err)?;

    let out = serde_json::json!({
        "market_json": serde_json::to_string(&market).map_err(to_js_err)?,
        "operations_applied": report.operations_applied,
        "warnings": report.warnings,
    });
    serde_wasm_bindgen::to_value(&out).map_err(to_js_err)
}
