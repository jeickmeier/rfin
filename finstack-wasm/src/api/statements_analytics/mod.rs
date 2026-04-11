//! WASM bindings for the `finstack-statements-analytics` crate.
//!
//! Exposes financial statement analysis functions that accept and return
//! JSON strings, suitable for consumption from JavaScript/TypeScript.

use crate::utils::to_js_err;
use wasm_bindgen::prelude::*;

/// Run a sensitivity analysis on a financial model.
///
/// Accepts JSON strings for the model spec and sensitivity configuration,
/// evaluates all perturbation scenarios, and returns JSON results.
#[wasm_bindgen(js_name = runSensitivity)]
pub fn run_sensitivity(model_json: &str, config_json: &str) -> Result<String, JsValue> {
    let model: finstack_statements::FinancialModelSpec =
        serde_json::from_str(model_json).map_err(to_js_err)?;

    let config: finstack_statements_analytics::analysis::SensitivityConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;

    let analyzer = finstack_statements_analytics::analysis::SensitivityAnalyzer::new(&model);
    let result = analyzer.run(&config).map_err(to_js_err)?;

    serde_json::to_string(&result).map_err(to_js_err)
}

/// Run a variance analysis comparing two evaluated statement results.
///
/// Returns JSON-serialized variance report.
#[wasm_bindgen(js_name = runVariance)]
pub fn run_variance(
    base_json: &str,
    comparison_json: &str,
    config_json: &str,
) -> Result<String, JsValue> {
    let base: finstack_statements::evaluator::StatementResult =
        serde_json::from_str(base_json).map_err(to_js_err)?;

    let comparison: finstack_statements::evaluator::StatementResult =
        serde_json::from_str(comparison_json).map_err(to_js_err)?;

    let config: finstack_statements_analytics::analysis::VarianceConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;

    let analyzer =
        finstack_statements_analytics::analysis::VarianceAnalyzer::new(&base, &comparison);
    let report = analyzer.compute(&config).map_err(to_js_err)?;

    serde_json::to_string(&report).map_err(to_js_err)
}

/// Evaluate all scenarios in a scenario set against a base model.
///
/// Returns a JSON object mapping scenario names to their statement results.
#[wasm_bindgen(js_name = evaluateScenarioSet)]
pub fn evaluate_scenario_set(model_json: &str, scenario_set_json: &str) -> Result<String, JsValue> {
    let model: finstack_statements::FinancialModelSpec =
        serde_json::from_str(model_json).map_err(to_js_err)?;

    let scenario_set: finstack_statements_analytics::analysis::ScenarioSet =
        serde_json::from_str(scenario_set_json).map_err(to_js_err)?;

    let results = scenario_set.evaluate_all(&model).map_err(to_js_err)?;

    let map: indexmap::IndexMap<&String, &finstack_statements::evaluator::StatementResult> =
        results.scenarios.iter().collect();
    serde_json::to_string(&map).map_err(to_js_err)
}

/// Compute forecast accuracy metrics (MAE, MAPE, RMSE).
///
/// Takes two float arrays (actual, forecast) and returns a JSON object
/// with keys `mae`, `mape`, `rmse`, `n`.
#[wasm_bindgen(js_name = backtestForecast)]
pub fn backtest_forecast(actual: JsValue, forecast: JsValue) -> Result<JsValue, JsValue> {
    let actual_vec: Vec<f64> = serde_wasm_bindgen::from_value(actual).map_err(to_js_err)?;
    let forecast_vec: Vec<f64> = serde_wasm_bindgen::from_value(forecast).map_err(to_js_err)?;

    let metrics =
        finstack_statements_analytics::analysis::backtest_forecast(&actual_vec, &forecast_vec)
            .map_err(to_js_err)?;

    let result = serde_json::json!({
        "mae": metrics.mae,
        "mape": metrics.mape,
        "rmse": metrics.rmse,
        "n": metrics.n,
    });
    serde_wasm_bindgen::to_value(&result).map_err(to_js_err)
}

/// Generate tornado chart entries for a sensitivity result.
#[wasm_bindgen(js_name = generateTornadoEntries)]
pub fn generate_tornado_entries(
    result_json: &str,
    metric_node: &str,
    period: Option<String>,
) -> Result<String, JsValue> {
    let result: finstack_statements_analytics::analysis::SensitivityResult =
        serde_json::from_str(result_json).map_err(to_js_err)?;
    let period_id: Option<finstack_core::dates::PeriodId> =
        period.map(|p| p.parse().map_err(to_js_err)).transpose()?;
    let entries = finstack_statements_analytics::analysis::generate_tornado_entries(
        &result,
        metric_node,
        period_id,
    );
    serde_json::to_string(&entries).map_err(to_js_err)
}

/// Run Monte Carlo simulation on a financial model (JSON in/out).
#[wasm_bindgen(js_name = runMonteCarlo)]
pub fn run_monte_carlo(model_json: &str, config_json: &str) -> Result<String, JsValue> {
    let model: finstack_statements::FinancialModelSpec =
        serde_json::from_str(model_json).map_err(to_js_err)?;
    let config: finstack_statements::evaluator::MonteCarloConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;
    let mut evaluator = finstack_statements::evaluator::Evaluator::new();
    let results = evaluator
        .evaluate_monte_carlo(&model, &config)
        .map_err(to_js_err)?;
    serde_json::to_string(&results).map_err(to_js_err)
}

/// Find the driver value that makes a target node reach a target value.
#[wasm_bindgen(js_name = goalSeek)]
#[allow(clippy::too_many_arguments)]
pub fn goal_seek(
    model_json: &str,
    target_node: &str,
    target_period: &str,
    target_value: f64,
    driver_node: &str,
    driver_period: &str,
    update_model: bool,
    bounds_lo: Option<f64>,
    bounds_hi: Option<f64>,
) -> Result<JsValue, JsValue> {
    let mut model: finstack_statements::FinancialModelSpec =
        serde_json::from_str(model_json).map_err(to_js_err)?;
    let tp: finstack_core::dates::PeriodId = target_period.parse().map_err(to_js_err)?;
    let dp: finstack_core::dates::PeriodId = driver_period.parse().map_err(to_js_err)?;
    let bounds = match (bounds_lo, bounds_hi) {
        (Some(lo), Some(hi)) => Some((lo, hi)),
        _ => None,
    };

    let result = finstack_statements_analytics::analysis::goal_seek(
        &mut model,
        target_node,
        tp,
        target_value,
        driver_node,
        dp,
        update_model,
        bounds,
    )
    .map_err(to_js_err)?;

    let updated_json = serde_json::to_string_pretty(&model).map_err(to_js_err)?;
    let out = serde_json::json!({
        "solved_value": result,
        "updated_model_json": updated_json,
    });
    serde_wasm_bindgen::to_value(&out).map_err(to_js_err)
}

/// Trace dependencies for a node and return ASCII tree.
#[wasm_bindgen(js_name = traceDependencies)]
pub fn trace_dependencies(model_json: &str, node_id: &str) -> Result<String, JsValue> {
    let model: finstack_statements::FinancialModelSpec =
        serde_json::from_str(model_json).map_err(to_js_err)?;
    let graph =
        finstack_statements::evaluator::DependencyGraph::from_model(&model).map_err(to_js_err)?;
    let tracer = finstack_statements_analytics::analysis::DependencyTracer::new(&model, &graph);
    let tree = tracer.dependency_tree(node_id).map_err(to_js_err)?;
    Ok(finstack_statements_analytics::analysis::render_tree_ascii(
        &tree,
    ))
}

/// Explain a formula for a specific node and period (JSON in/out).
#[wasm_bindgen(js_name = explainFormula)]
pub fn explain_formula(
    model_json: &str,
    results_json: &str,
    node_id: &str,
    period: &str,
) -> Result<String, JsValue> {
    let model: finstack_statements::FinancialModelSpec =
        serde_json::from_str(model_json).map_err(to_js_err)?;
    let results: finstack_statements::evaluator::StatementResult =
        serde_json::from_str(results_json).map_err(to_js_err)?;
    let pid: finstack_core::dates::PeriodId = period.parse().map_err(to_js_err)?;
    let explainer =
        finstack_statements_analytics::analysis::FormulaExplainer::new(&model, &results);
    let explanation = explainer.explain(node_id, &pid).map_err(to_js_err)?;
    Ok(explanation.to_string_detailed())
}

/// Generate a P&L summary report as formatted text.
#[wasm_bindgen(js_name = plSummaryReport)]
pub fn pl_summary_report(
    results_json: &str,
    line_items: JsValue,
    periods: JsValue,
) -> Result<String, JsValue> {
    use finstack_statements_analytics::analysis::Report;

    let results: finstack_statements::evaluator::StatementResult =
        serde_json::from_str(results_json).map_err(to_js_err)?;
    let items: Vec<String> = serde_wasm_bindgen::from_value(line_items).map_err(to_js_err)?;
    let period_strs: Vec<String> = serde_wasm_bindgen::from_value(periods).map_err(to_js_err)?;
    let period_ids: Vec<finstack_core::dates::PeriodId> = period_strs
        .iter()
        .map(|p| p.parse().map_err(to_js_err))
        .collect::<Result<Vec<_>, _>>()?;
    let report =
        finstack_statements_analytics::analysis::PLSummaryReport::new(&results, items, period_ids);
    Ok(report.to_string())
}

/// Generate a credit assessment report as formatted text.
#[wasm_bindgen(js_name = creditAssessmentReport)]
pub fn credit_assessment_report(results_json: &str, as_of: &str) -> Result<String, JsValue> {
    use finstack_statements_analytics::analysis::Report;

    let results: finstack_statements::evaluator::StatementResult =
        serde_json::from_str(results_json).map_err(to_js_err)?;
    let period: finstack_core::dates::PeriodId = as_of.parse().map_err(to_js_err)?;
    let report =
        finstack_statements_analytics::analysis::CreditAssessmentReport::new(&results, period);
    Ok(report.to_string())
}
