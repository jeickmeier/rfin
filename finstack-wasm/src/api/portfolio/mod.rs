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

/// Optimize portfolio weights using the LP-based optimizer.
///
/// Accepts a `PortfolioOptimizationSpec` JSON (portfolio + objective +
/// constraints + options) and a `MarketContext` JSON.
#[wasm_bindgen(js_name = optimizePortfolio)]
pub fn optimize_portfolio(spec_json: &str, market_json: &str) -> Result<String, JsValue> {
    let spec: finstack_portfolio::PortfolioOptimizationSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let config = finstack_core::config::FinstackConfig::default();
    let result =
        finstack_portfolio::optimize_from_spec(&spec, &market, &config).map_err(to_js_err)?;
    serde_json::to_string_pretty(&result).map_err(to_js_err)
}

/// Replay a portfolio through dated market snapshots.
///
/// Accepts a portfolio spec, an array of dated market snapshots, and a
/// replay configuration. Returns a JSON-serialized `ReplayResult`.
#[wasm_bindgen(js_name = replayPortfolio)]
pub fn replay_portfolio(
    spec_json: &str,
    snapshots_json: &str,
    config_json: &str,
) -> Result<String, JsValue> {
    let spec: finstack_portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let portfolio = finstack_portfolio::Portfolio::from_spec(spec).map_err(to_js_err)?;

    let config: finstack_portfolio::ReplayConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;

    // Parse snapshots: [{"date": "YYYY-MM-DD", "market": {...}}, ...]
    let raw: Vec<serde_json::Value> =
        serde_json::from_str(snapshots_json).map_err(to_js_err)?;

    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let mut snapshots = Vec::with_capacity(raw.len());
    for entry in &raw {
        let date_str = entry["date"]
            .as_str()
            .ok_or_else(|| to_js_err("each snapshot must have a 'date' string field"))?;
        let date = time::Date::parse(date_str, &format).map_err(to_js_err)?;
        let market: finstack_core::market_data::context::MarketContext =
            serde_json::from_value(entry["market"].clone()).map_err(to_js_err)?;
        snapshots.push((date, market));
    }

    let timeline = finstack_portfolio::ReplayTimeline::new(snapshots).map_err(to_js_err)?;
    let finstack_config = finstack_core::config::FinstackConfig::default();

    let result =
        finstack_portfolio::replay_portfolio(&portfolio, &timeline, &config, &finstack_config)
            .map_err(to_js_err)?;

    serde_json::to_string(&result).map_err(to_js_err)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn minimal_portfolio_spec_json() -> String {
        serde_json::json!({
            "id": "test_portfolio",
            "name": "Test",
            "base_ccy": "USD",
            "as_of": "2024-01-15",
            "entities": {},
            "positions": []
        })
        .to_string()
    }

    #[test]
    fn parse_portfolio_spec_roundtrip() {
        let json = minimal_portfolio_spec_json();
        let result = parse_portfolio_spec(&json).expect("parse");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(parsed["id"], "test_portfolio");
    }

    #[test]
    fn build_portfolio_from_spec_empty() {
        let json = minimal_portfolio_spec_json();
        let result = build_portfolio_from_spec(&json).expect("build");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(parsed["id"], "test_portfolio");
    }

    #[test]
    fn parse_and_rebuild_roundtrip() {
        let json = minimal_portfolio_spec_json();
        let canonical = parse_portfolio_spec(&json).expect("parse");
        let rebuilt = build_portfolio_from_spec(&canonical).expect("rebuild");
        let a: serde_json::Value = serde_json::from_str(&canonical).expect("a");
        let b: serde_json::Value = serde_json::from_str(&rebuilt).expect("b");
        assert_eq!(a["id"], b["id"]);
    }

    fn empty_market_json() -> String {
        let ctx = finstack_core::market_data::context::MarketContext::new();
        serde_json::to_string(&ctx).expect("serialize")
    }

    #[test]
    fn value_empty_portfolio() {
        let spec = minimal_portfolio_spec_json();
        let market = empty_market_json();
        let result = value_portfolio(&spec, &market, false).expect("value");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(parsed.is_object());
    }

    #[test]
    fn aggregate_cashflows_empty_portfolio() {
        let spec = minimal_portfolio_spec_json();
        let market = empty_market_json();
        let result = aggregate_cashflows(&spec, &market).expect("aggregate");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(parsed.is_object() || parsed.is_array());
    }

    #[test]
    fn portfolio_result_total_value_from_valuation() {
        let spec = minimal_portfolio_spec_json();
        let market = empty_market_json();
        let valuation_json = value_portfolio(&spec, &market, false).expect("value");
        let result = finstack_portfolio::PortfolioResult {
            valuation: serde_json::from_str(&valuation_json).expect("deser"),
            metrics: Default::default(),
            meta: Default::default(),
        };
        let result_json = serde_json::to_string(&result).expect("ser");
        let total = portfolio_result_total_value(&result_json).expect("total");
        assert!(total.is_finite());
    }

    #[test]
    fn aggregate_metrics_empty_portfolio() {
        let spec = minimal_portfolio_spec_json();
        let market = empty_market_json();
        let valuation_json = value_portfolio(&spec, &market, false).expect("value");
        let result = aggregate_metrics(&valuation_json, "USD", &market, "2024-01-15").expect("agg");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(parsed.is_object());
    }

    /// Tests the replay_portfolio WASM binding logic by exercising the same
    /// JSON parsing / domain call / serialization pipeline directly.
    /// We call the domain functions instead of the wasm wrapper because
    /// `JsValue::from_str` panics on non-wasm32 targets when an error is
    /// produced.
    #[test]
    fn replay_portfolio_empty_portfolio() {
        let spec_json = minimal_portfolio_spec_json();
        let spec: finstack_portfolio::PortfolioSpec =
            serde_json::from_str(&spec_json).expect("parse spec");
        let portfolio =
            finstack_portfolio::Portfolio::from_spec(spec).expect("build portfolio");

        let market_json = empty_market_json();
        let market_val: serde_json::Value =
            serde_json::from_str(&market_json).expect("parse market");
        let snapshots_raw = serde_json::json!([
            {"date": "2024-01-15", "market": market_val.clone()},
            {"date": "2024-01-16", "market": market_val}
        ]);

        let format = time::format_description::well_known::Iso8601::DEFAULT;
        let mut snapshots = Vec::new();
        for entry in snapshots_raw.as_array().expect("array") {
            let date_str = entry["date"].as_str().expect("date string");
            let date = time::Date::parse(date_str, &format).expect("parse date");
            let market: finstack_core::market_data::context::MarketContext =
                serde_json::from_value(entry["market"].clone()).expect("parse market ctx");
            snapshots.push((date, market));
        }

        let timeline =
            finstack_portfolio::ReplayTimeline::new(snapshots).expect("build timeline");

        let config_json = serde_json::json!({
            "mode": "PvOnly",
            "attribution_method": "Parallel"
        })
        .to_string();
        let config: finstack_portfolio::ReplayConfig =
            serde_json::from_str(&config_json).expect("parse config");

        let finstack_config = finstack_core::config::FinstackConfig::default();

        let result = finstack_portfolio::replay_portfolio(
            &portfolio,
            &timeline,
            &config,
            &finstack_config,
        )
        .expect("replay");

        let json = serde_json::to_string(&result).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse json");
        assert!(parsed["steps"].is_array());
        assert_eq!(parsed["steps"].as_array().expect("array").len(), 2);
    }
}
