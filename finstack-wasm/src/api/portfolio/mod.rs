//! WASM bindings for the `finstack-portfolio` crate.
//!
//! Exposes portfolio spec parsing, validation, and result extraction
//! via JSON round-trip functions for JavaScript/TypeScript consumption.

use crate::utils::to_js_err;
use wasm_bindgen::prelude::*;

/// Parse and validate a portfolio specification from JSON.
///
/// Returns the re-serialized canonical JSON form.
#[wasm_bindgen(js_name = parsePortfolioSpec)]
pub fn parse_portfolio_spec(json_str: &str) -> Result<String, JsValue> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(json_str).map_err(to_js_err)?;

    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Build a runtime portfolio from a JSON spec, validate, and round-trip.
///
/// Deserializes the spec, constructs the portfolio with live instruments,
/// validates structural invariants, then re-serializes for confirmation.
#[wasm_bindgen(js_name = buildPortfolioFromSpec)]
pub fn build_portfolio_from_spec(spec_json: &str) -> Result<String, JsValue> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;

    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(to_js_err)?;

    let round_tripped = portfolio.to_spec();
    serde_json::to_string(&round_tripped).map_err(to_js_err)
}

/// Extract the total portfolio value from a JSON result.
#[wasm_bindgen(js_name = portfolioResultTotalValue)]
pub fn portfolio_result_total_value(result_json: &str) -> Result<f64, JsValue> {
    let result: finstack_portfolio::PortfolioResult =
        serde_json::from_str(result_json).map_err(to_js_err)?;

    Ok(result.total_value().amount())
}

/// Extract a specific metric from a portfolio result JSON.
///
/// Returns `undefined` (via `Option`) if the metric was not produced.
#[wasm_bindgen(js_name = portfolioResultGetMetric)]
pub fn portfolio_result_get_metric(result_json: &str, metric_id: &str) -> Result<JsValue, JsValue> {
    let result: finstack_portfolio::PortfolioResult =
        serde_json::from_str(result_json).map_err(to_js_err)?;

    match result.get_metric(metric_id) {
        Some(v) => Ok(JsValue::from_f64(v)),
        None => Ok(JsValue::UNDEFINED),
    }
}

/// Aggregate portfolio metrics from a valuation JSON.
#[wasm_bindgen(js_name = aggregateMetrics)]
pub fn aggregate_metrics(
    valuation_json: &str,
    base_ccy: &str,
    market_json: &str,
    as_of: &str,
) -> Result<String, JsValue> {
    let valuation: finstack_portfolio::valuation::PortfolioValuation =
        serde_json::from_str(valuation_json).map_err(to_js_err)?;
    let ccy: finstack_core::currency::Currency = base_ccy.parse().map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;
    let metrics =
        finstack_portfolio::aggregate_metrics(&valuation, ccy, &market, date).map_err(to_js_err)?;
    serde_json::to_string(&metrics).map_err(to_js_err)
}

/// Value a portfolio from its spec and market context.
#[wasm_bindgen(js_name = valuePortfolio)]
pub fn value_portfolio(
    spec_json: &str,
    market_json: &str,
    strict_risk: bool,
) -> Result<String, JsValue> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(to_js_err)?;
    let config = finstack_core::config::FinstackConfig::default();
    let options = finstack_portfolio::PortfolioValuationOptions {
        strict_risk,
        ..Default::default()
    };
    let valuation = finstack_portfolio::value_portfolio(&portfolio, &market, &config, &options)
        .map_err(to_js_err)?;
    serde_json::to_string(&valuation).map_err(to_js_err)
}

/// Aggregate cashflows for a portfolio from its spec and market context.
#[wasm_bindgen(js_name = aggregateCashflows)]
pub fn aggregate_cashflows(spec_json: &str, market_json: &str) -> Result<String, JsValue> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(to_js_err)?;
    let cashflows =
        finstack_portfolio::aggregate_cashflows(&portfolio, &market).map_err(to_js_err)?;
    serde_json::to_string(&cashflows).map_err(to_js_err)
}

/// Apply a scenario to a portfolio and revalue.
///
/// Returns a JSON object with `valuation` and `report` string keys.
#[wasm_bindgen(js_name = applyScenarioAndRevalue)]
pub fn apply_scenario_and_revalue(
    spec_json: &str,
    scenario_json: &str,
    market_json: &str,
) -> Result<JsValue, JsValue> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let scenario: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(to_js_err)?;
    let config = finstack_core::config::FinstackConfig::default();
    let (valuation, report) =
        finstack_portfolio::apply_and_revalue(&portfolio, &scenario, &market, &config)
            .map_err(to_js_err)?;
    let val_json = serde_json::to_string(&valuation).map_err(to_js_err)?;
    let report_json = serde_json::to_string(&report).map_err(to_js_err)?;
    let out = serde_json::json!({
        "valuation": val_json,
        "report": report_json,
    });
    serde_wasm_bindgen::to_value(&out).map_err(to_js_err)
}
