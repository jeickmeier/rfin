//! WASM bindings for the `finstack-valuations` crate.
//!
//! Exposes JSON round-trip for valuation results, instrument validation,
//! and P&L attribution across multiple methodologies.

use crate::utils::to_js_err;
use finstack_valuations::factor_model::FactorSensitivityEngine;
use wasm_bindgen::prelude::*;

/// Deserialize a `ValuationResult` from JSON and return the canonical JSON.
///
/// Validates the input conforms to the `ValuationResult` schema.
#[wasm_bindgen(js_name = validateValuationResultJson)]
pub fn validate_valuation_result_json(json: &str) -> Result<String, JsValue> {
    let result: finstack_valuations::results::ValuationResult =
        serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

/// Validate a tagged instrument JSON string.
///
/// Deserializes the input against the known instrument schema and
/// returns the canonical (re-serialized) JSON.
#[wasm_bindgen(js_name = validateInstrumentJson)]
pub fn validate_instrument_json(json: &str) -> Result<String, JsValue> {
    let parsed: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string(&parsed).map_err(to_js_err)
}

/// Price an instrument from its tagged JSON and return a ValuationResult JSON.
#[wasm_bindgen(js_name = priceInstrument)]
pub fn price_instrument(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
) -> Result<String, JsValue> {
    let inst: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(to_js_err)?;
    let boxed = inst.into_boxed().map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;
    let model_key = parse_model_key(model)?;
    let registry = finstack_valuations::pricer::standard_registry();
    let result = registry
        .price(boxed.as_ref(), model_key, &market, date, None)
        .map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

/// Price an instrument with explicit metric requests.
#[wasm_bindgen(js_name = priceInstrumentWithMetrics)]
pub fn price_instrument_with_metrics(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
    metrics: JsValue,
) -> Result<String, JsValue> {
    let inst: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(to_js_err)?;
    let boxed = inst.into_boxed().map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;
    let model_key = parse_model_key(model)?;
    let metric_strs: Vec<String> = serde_wasm_bindgen::from_value(metrics).map_err(to_js_err)?;
    let metric_ids: Vec<finstack_valuations::metrics::MetricId> = metric_strs
        .iter()
        .map(|m| finstack_valuations::metrics::MetricId::custom(m.as_str()))
        .collect();
    let registry = finstack_valuations::pricer::standard_registry();
    let result = registry
        .price_with_metrics(
            boxed.as_ref(),
            model_key,
            &market,
            date,
            &metric_ids,
            Default::default(),
        )
        .map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

/// List all metric IDs in the standard metric registry.
#[wasm_bindgen(js_name = listStandardMetrics)]
pub fn list_standard_metrics() -> Result<JsValue, JsValue> {
    let ids: Vec<String> = finstack_valuations::metrics::standard_registry()
        .available_metrics()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    serde_wasm_bindgen::to_value(&ids).map_err(to_js_err)
}

/// List all standard metrics organized by group.
///
/// Returns a JSON object `{ group_name: [metric_id, ...], ... }` where
/// each key is a human-readable group name (e.g. "Pricing", "Greeks",
/// "Sensitivity") and the value is a sorted array of metric ID strings.
#[wasm_bindgen(js_name = listStandardMetricsGrouped)]
pub fn list_standard_metrics_grouped() -> Result<JsValue, JsValue> {
    let grouped: Vec<(String, Vec<String>)> = finstack_valuations::metrics::standard_registry()
        .available_metrics_grouped()
        .into_iter()
        .map(|(group, metrics)| {
            (
                group.display_name().to_string(),
                metrics.into_iter().map(|m| m.to_string()).collect(),
            )
        })
        .collect();
    let map: std::collections::BTreeMap<String, Vec<String>> = grouped.into_iter().collect();
    serde_wasm_bindgen::to_value(&map).map_err(to_js_err)
}

// ---------------------------------------------------------------------------
// Attribution
// ---------------------------------------------------------------------------

/// Run P&L attribution for a single instrument.
///
/// Accepts the instrument JSON, two market snapshots, dates, and a
/// method descriptor.  Returns the `PnlAttribution` result as JSON.
#[wasm_bindgen(js_name = attributePnl)]
pub fn attribute_pnl(
    instrument_json: &str,
    market_t0_json: &str,
    market_t1_json: &str,
    as_of_t0: &str,
    as_of_t1: &str,
    method_json: &str,
    config_json: Option<String>,
) -> Result<String, JsValue> {
    let spec = build_attribution_spec(
        instrument_json,
        market_t0_json,
        market_t1_json,
        as_of_t0,
        as_of_t1,
        method_json,
        config_json.as_deref(),
    )?;
    let result = spec.execute().map_err(to_js_err)?;
    serde_json::to_string(&result.attribution).map_err(to_js_err)
}

/// Run attribution from a full JSON `AttributionEnvelope` and return JSON.
///
/// Power-user variant for full envelope round-trip workflows.
#[wasm_bindgen(js_name = attributePnlFromSpec)]
pub fn attribute_pnl_from_spec(spec_json: &str) -> Result<String, JsValue> {
    let envelope: finstack_valuations::attribution::AttributionEnvelope =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let result_envelope = envelope.execute().map_err(to_js_err)?;
    serde_json::to_string(&result_envelope).map_err(to_js_err)
}

/// Validate an attribution specification JSON.
///
/// Deserializes against the `AttributionEnvelope` schema and returns
/// the canonical JSON.
#[wasm_bindgen(js_name = validateAttributionJson)]
pub fn validate_attribution_json(json: &str) -> Result<String, JsValue> {
    let envelope: finstack_valuations::attribution::AttributionEnvelope =
        serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string(&envelope).map_err(to_js_err)
}

/// Return the default waterfall factor ordering as a JSON array.
#[wasm_bindgen(js_name = defaultWaterfallOrder)]
pub fn default_waterfall_order() -> Result<JsValue, JsValue> {
    let factors: Vec<String> = finstack_valuations::attribution::default_waterfall_order()
        .into_iter()
        .map(|f| f.to_string())
        .collect();
    serde_wasm_bindgen::to_value(&factors).map_err(to_js_err)
}

/// Return the default metric IDs used by metrics-based attribution.
#[wasm_bindgen(js_name = defaultAttributionMetrics)]
pub fn default_attribution_metrics() -> Result<JsValue, JsValue> {
    let metrics: Vec<String> = finstack_valuations::attribution::default_attribution_metrics()
        .into_iter()
        .map(|m| m.to_string())
        .collect();
    serde_wasm_bindgen::to_value(&metrics).map_err(to_js_err)
}

// ---------------------------------------------------------------------------
// Factor Sensitivity
// ---------------------------------------------------------------------------

/// JSON input for a single position in the factor-sensitivity pipeline.
#[derive(serde::Deserialize)]
struct PositionInput {
    /// Position identifier.
    id: String,
    /// Tagged instrument JSON.
    instrument: serde_json::Value,
    /// Position weight (notional multiplier).
    weight: f64,
}

/// Compute first-order factor sensitivities and return the matrix as JSON.
///
/// Accepts a JSON array of positions, a JSON array of `FactorDefinition`,
/// a `MarketContext` JSON, an ISO 8601 date, and an optional `BumpSizeConfig`
/// JSON.  Returns a JSON object with `position_ids`, `factor_ids`, and a
/// row-major `data` matrix.
#[wasm_bindgen(js_name = computeFactorSensitivities)]
pub fn compute_factor_sensitivities(
    positions_json: &str,
    factors_json: &str,
    market_json: &str,
    as_of: &str,
    bump_config_json: Option<String>,
) -> Result<String, JsValue> {
    let specs: Vec<PositionInput> = serde_json::from_str(positions_json).map_err(to_js_err)?;
    let instruments: Vec<Box<finstack_valuations::instruments::common::traits::DynInstrument>> =
        specs
            .iter()
            .map(|p| {
                let inst: finstack_valuations::instruments::InstrumentJson =
                    serde_json::from_value(p.instrument.clone()).map_err(to_js_err)?;
                inst.into_boxed().map_err(to_js_err)
            })
            .collect::<Result<Vec<_>, _>>()?;
    let positions: Vec<(
        String,
        &dyn finstack_valuations::instruments::internal::InstrumentExt,
        f64,
    )> = specs
        .iter()
        .zip(instruments.iter())
        .map(|(s, inst)| {
            (
                s.id.clone(),
                inst.as_ref() as &dyn finstack_valuations::instruments::internal::InstrumentExt,
                s.weight,
            )
        })
        .collect();

    let factors: Vec<finstack_core::factor_model::FactorDefinition> =
        serde_json::from_str(factors_json).map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;
    let bump_config: finstack_core::factor_model::BumpSizeConfig = match bump_config_json {
        Some(ref json) => serde_json::from_str(json).map_err(to_js_err)?,
        None => finstack_core::factor_model::BumpSizeConfig::default(),
    };

    let engine = finstack_valuations::factor_model::DeltaBasedEngine::new(bump_config);
    let matrix = engine
        .compute_sensitivities(&positions, &factors, &market, date)
        .map_err(to_js_err)?;

    let result = serde_json::json!({
        "position_ids": matrix.position_ids(),
        "factor_ids": matrix.factor_ids().iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        "data": (0..matrix.n_positions())
            .map(|pi| matrix.position_deltas(pi).to_vec())
            .collect::<Vec<Vec<f64>>>(),
    });
    serde_json::to_string(&result).map_err(to_js_err)
}

/// Compute scenario P&L profiles via full repricing and return as JSON.
///
/// Same position/factor/market inputs as `computeFactorSensitivities`, plus
/// an optional `n_scenario_points` integer (default 5).
#[wasm_bindgen(js_name = computePnlProfiles)]
pub fn compute_pnl_profiles(
    positions_json: &str,
    factors_json: &str,
    market_json: &str,
    as_of: &str,
    bump_config_json: Option<String>,
    n_scenario_points: Option<usize>,
) -> Result<String, JsValue> {
    let n_points = n_scenario_points.unwrap_or(5);
    let specs: Vec<PositionInput> = serde_json::from_str(positions_json).map_err(to_js_err)?;
    let instruments: Vec<Box<finstack_valuations::instruments::common::traits::DynInstrument>> =
        specs
            .iter()
            .map(|p| {
                let inst: finstack_valuations::instruments::InstrumentJson =
                    serde_json::from_value(p.instrument.clone()).map_err(to_js_err)?;
                inst.into_boxed().map_err(to_js_err)
            })
            .collect::<Result<Vec<_>, _>>()?;
    let positions: Vec<(
        String,
        &dyn finstack_valuations::instruments::internal::InstrumentExt,
        f64,
    )> = specs
        .iter()
        .zip(instruments.iter())
        .map(|(s, inst)| {
            (
                s.id.clone(),
                inst.as_ref() as &dyn finstack_valuations::instruments::internal::InstrumentExt,
                s.weight,
            )
        })
        .collect();

    let factors: Vec<finstack_core::factor_model::FactorDefinition> =
        serde_json::from_str(factors_json).map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;
    let bump_config: finstack_core::factor_model::BumpSizeConfig = match bump_config_json {
        Some(ref json) => serde_json::from_str(json).map_err(to_js_err)?,
        None => finstack_core::factor_model::BumpSizeConfig::default(),
    };

    let engine = finstack_valuations::factor_model::FullRepricingEngine::new(bump_config, n_points);
    let profiles = engine
        .compute_pnl_profiles(&positions, &factors, &market, date)
        .map_err(to_js_err)?;

    let result: Vec<serde_json::Value> = profiles
        .iter()
        .map(|p| {
            serde_json::json!({
                "factor_id": p.factor_id.to_string(),
                "shifts": p.shifts,
                "position_pnls": p.position_pnls,
            })
        })
        .collect();
    serde_json::to_string(&result).map_err(to_js_err)
}

/// Decompose portfolio risk into factor and position contributions.
///
/// Uses the parametric (covariance-based) Euler decomposition.  Accepts
/// a JSON sensitivity matrix (same schema as the output of
/// `computeFactorSensitivities`), a `FactorCovarianceMatrix` JSON, and an
/// optional `RiskMeasure` JSON.
///
/// Returns a JSON object with `total_risk`, `measure`, `residual_risk`,
/// `factor_contributions` (array), and `position_factor_contributions` (array).
#[wasm_bindgen(js_name = decomposeFactorRisk)]
pub fn decompose_factor_risk(
    sensitivities_json: &str,
    covariance_json: &str,
    risk_measure_json: Option<String>,
) -> Result<String, JsValue> {
    #[derive(serde::Deserialize)]
    struct SensInput {
        position_ids: Vec<String>,
        factor_ids: Vec<String>,
        data: Vec<Vec<f64>>,
    }

    let input: SensInput = serde_json::from_str(sensitivities_json).map_err(to_js_err)?;
    let factor_ids: Vec<finstack_core::factor_model::FactorId> = input
        .factor_ids
        .iter()
        .map(finstack_core::factor_model::FactorId::new)
        .collect();

    let mut matrix =
        finstack_valuations::factor_model::SensitivityMatrix::zeros(input.position_ids, factor_ids);
    for (pi, row) in input.data.iter().enumerate() {
        for (fi, &val) in row.iter().enumerate() {
            matrix.set_delta(pi, fi, val);
        }
    }

    let covariance: finstack_core::factor_model::FactorCovarianceMatrix =
        serde_json::from_str(covariance_json).map_err(to_js_err)?;

    let measure: finstack_core::factor_model::RiskMeasure = match risk_measure_json {
        Some(ref json) => serde_json::from_str(json).map_err(to_js_err)?,
        None => finstack_core::factor_model::RiskMeasure::Variance,
    };

    let decomposer = finstack_portfolio::factor_model::ParametricDecomposer;
    let result = finstack_portfolio::factor_model::RiskDecomposer::decompose(
        &decomposer,
        &matrix,
        &covariance,
        &measure,
    )
    .map_err(to_js_err)?;

    let output = serde_json::json!({
        "total_risk": result.total_risk,
        "measure": format!("{:?}", result.measure),
        "residual_risk": result.residual_risk,
        "factor_contributions": result.factor_contributions.iter().map(|c| {
            serde_json::json!({
                "factor_id": c.factor_id.to_string(),
                "absolute_risk": c.absolute_risk,
                "relative_risk": c.relative_risk,
                "marginal_risk": c.marginal_risk,
            })
        }).collect::<Vec<_>>(),
        "position_factor_contributions": result.position_factor_contributions.iter().map(|c| {
            serde_json::json!({
                "position_id": c.position_id.to_string(),
                "factor_id": c.factor_id.to_string(),
                "risk_contribution": c.risk_contribution,
            })
        }).collect::<Vec<_>>(),
    });
    serde_json::to_string(&output).map_err(to_js_err)
}

fn build_attribution_spec(
    instrument_json: &str,
    market_t0_json: &str,
    market_t1_json: &str,
    as_of_t0: &str,
    as_of_t1: &str,
    method_json: &str,
    config_json: Option<&str>,
) -> Result<finstack_valuations::attribution::AttributionSpec, JsValue> {
    let instrument: finstack_valuations::instruments::InstrumentJson =
        serde_json::from_str(instrument_json).map_err(to_js_err)?;
    let market_t0: finstack_core::market_data::context::MarketContextState =
        serde_json::from_str(market_t0_json).map_err(to_js_err)?;
    let market_t1: finstack_core::market_data::context::MarketContextState =
        serde_json::from_str(market_t1_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let t0 = time::Date::parse(as_of_t0, &format).map_err(to_js_err)?;
    let t1 = time::Date::parse(as_of_t1, &format).map_err(to_js_err)?;
    let method: finstack_valuations::attribution::AttributionMethod =
        serde_json::from_str(method_json).map_err(to_js_err)?;
    let config: Option<finstack_valuations::attribution::AttributionConfig> = match config_json {
        Some(json) => Some(serde_json::from_str(json).map_err(to_js_err)?),
        None => None,
    };
    Ok(finstack_valuations::attribution::AttributionSpec {
        instrument,
        market_t0,
        market_t1,
        as_of_t0: t0,
        as_of_t1: t1,
        method,
        model_params_t0: None,
        config,
    })
}

fn parse_model_key(s: &str) -> Result<finstack_valuations::pricer::ModelKey, JsValue> {
    s.parse::<finstack_valuations::pricer::ModelKey>()
        .map_err(|e| to_js_err(format!("Unknown model key: '{s}'. {e}")))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_valuations::pricer::ModelKey;

    #[test]
    fn parse_model_key_recognizes_standard_keys() {
        assert_eq!(
            parse_model_key("discounting").expect("ok"),
            ModelKey::Discounting
        );
        assert_eq!(parse_model_key("tree").expect("ok"), ModelKey::Tree);
        assert_eq!(parse_model_key("black76").expect("ok"), ModelKey::Black76);
        assert_eq!(
            parse_model_key("hull_white_1f").expect("ok"),
            ModelKey::HullWhite1F
        );
        assert_eq!(
            parse_model_key("hazard_rate").expect("ok"),
            ModelKey::HazardRate
        );
        assert_eq!(parse_model_key("normal").expect("ok"), ModelKey::Normal);
        assert_eq!(
            parse_model_key("monte_carlo_gbm").expect("ok"),
            ModelKey::MonteCarloGBM
        );
    }

    fn bond_instrument_json() -> String {
        use finstack_core::currency::Currency;
        use finstack_core::money::Money;
        use finstack_valuations::instruments::fixed_income::bond::Bond;
        use finstack_valuations::instruments::InstrumentJson;

        let bond = Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            time::Date::from_calendar_date(2024, time::Month::January, 1).expect("date"),
            time::Date::from_calendar_date(2034, time::Month::January, 1).expect("date"),
            "USD-OIS",
        )
        .expect("bond");
        serde_json::to_string(&InstrumentJson::Bond(bond)).expect("serialize")
    }

    fn market_context_json() -> String {
        use finstack_core::market_data::context::MarketContext;
        use finstack_core::market_data::term_structures::DiscountCurve;
        let base = time::Date::from_calendar_date(2024, time::Month::January, 1).expect("date");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.5, 0.99), (1.0, 0.98), (5.0, 0.90), (10.0, 0.80)])
            .build()
            .expect("curve");
        let ctx = MarketContext::new().insert(disc);
        serde_json::to_string(&ctx).expect("serialize")
    }

    #[test]
    fn validate_instrument_json_bond() {
        let json = bond_instrument_json();
        let canonical = validate_instrument_json(&json).expect("validate");
        assert!(!canonical.is_empty());
    }

    #[test]
    fn price_instrument_bond() {
        let inst = bond_instrument_json();
        let mkt = market_context_json();
        let result = price_instrument(&inst, &mkt, "2024-01-01", "discounting").expect("price");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(parsed.is_object());
    }

    #[test]
    fn validate_valuation_result_json_roundtrip() {
        let inst = bond_instrument_json();
        let mkt = market_context_json();
        let result_json =
            price_instrument(&inst, &mkt, "2024-01-01", "discounting").expect("price");
        let canonical = validate_valuation_result_json(&result_json).expect("validate");
        assert!(!canonical.is_empty());
        let parsed: serde_json::Value = serde_json::from_str(&canonical).expect("json");
        assert!(parsed.is_object());
    }
}
