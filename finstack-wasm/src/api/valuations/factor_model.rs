//! WASM bindings for factor-model sensitivities and risk decomposition.

use crate::utils::to_js_err;
use finstack_valuations::factor_model::FactorSensitivityEngine;
use wasm_bindgen::prelude::*;

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
    let parsed_positions = finstack_valuations::factor_model::parse_positions_json(positions_json)
        .map_err(to_js_err)?;
    let positions = finstack_valuations::factor_model::pricing_positions(&parsed_positions);

    let factors: Vec<finstack_core::factor_model::FactorDefinition> =
        serde_json::from_str(factors_json).map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let date = finstack_valuations::pricer::parse_as_of_date(as_of).map_err(to_js_err)?;
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
    let parsed_positions = finstack_valuations::factor_model::parse_positions_json(positions_json)
        .map_err(to_js_err)?;
    let positions = finstack_valuations::factor_model::pricing_positions(&parsed_positions);

    let factors: Vec<finstack_core::factor_model::FactorDefinition> =
        serde_json::from_str(factors_json).map_err(to_js_err)?;
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let date = finstack_valuations::pricer::parse_as_of_date(as_of).map_err(to_js_err)?;
    let bump_config: finstack_core::factor_model::BumpSizeConfig = match bump_config_json {
        Some(ref json) => serde_json::from_str(json).map_err(to_js_err)?,
        None => finstack_core::factor_model::BumpSizeConfig::default(),
    };

    let engine =
        finstack_valuations::factor_model::FullRepricingEngine::try_new(bump_config, n_points)
            .map_err(to_js_err)?;
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
