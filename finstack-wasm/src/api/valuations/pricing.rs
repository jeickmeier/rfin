//! WASM bindings for instrument pricing and metric introspection.

use crate::utils::to_js_err;
use finstack_valuations::instruments::credit_derivatives::cds::CreditDefaultSwap;
use finstack_valuations::instruments::credit_derivatives::cds_index::CDSIndex;
use finstack_valuations::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_valuations::instruments::credit_derivatives::cds_tranche::CDSTranche;
use finstack_valuations::instruments::models::credit::{
    CreditState, CreditStateVariable, DynamicRecoverySpec, EndogenousHazardSpec, MertonModel,
    OptimalToggle, ThresholdDirection, ToggleExerciseModel,
};
use finstack_valuations::instruments::InstrumentJson;
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
    finstack_valuations::pricer::validate_instrument_json(json).map_err(to_js_err)
}

/// Price an instrument from its tagged JSON and return a ValuationResult JSON.
///
/// Pass `model = "default"` to use the instrument-native default model.
#[wasm_bindgen(js_name = priceInstrument)]
pub fn price_instrument(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
) -> Result<String, JsValue> {
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let result =
        finstack_valuations::pricer::price_instrument_json(instrument_json, &market, as_of, model)
            .map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

/// Price an instrument with explicit metric requests.
///
/// Pass `model = "default"` to use the instrument-native default model.
#[wasm_bindgen(js_name = priceInstrumentWithMetrics)]
pub fn price_instrument_with_metrics(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
    metrics: JsValue,
    pricing_options: Option<String>,
) -> Result<String, JsValue> {
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let metric_strs: Vec<String> = serde_wasm_bindgen::from_value(metrics).map_err(to_js_err)?;
    let result = finstack_valuations::pricer::price_instrument_json_with_metrics(
        instrument_json,
        &market,
        as_of,
        model,
        &metric_strs,
        pricing_options.as_deref(),
    )
    .map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

/// Per-flow cashflow envelope (DF / survival / PV) for a discountable instrument.
///
/// `model` must be `"discounting"` or `"hazard_rate"`. Unsupported models or
/// incompatible instrument types throw. For supported pairs, the envelope's
/// `total_pv` matches the instrument's `base_value` within rounding.
#[wasm_bindgen(js_name = instrumentCashflowsJson)]
pub fn instrument_cashflows_json(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    model: &str,
) -> Result<String, JsValue> {
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    finstack_valuations::instruments::cashflow_export::instrument_cashflows_json(
        instrument_json,
        &market,
        as_of,
        model,
    )
    .map_err(to_js_err)
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

/// Build a structural Merton model JSON payload.
#[wasm_bindgen(js_name = mertonModelJson)]
pub fn merton_model_json(
    asset_value: f64,
    asset_vol: f64,
    debt_barrier: f64,
    risk_free_rate: f64,
) -> Result<String, JsValue> {
    let model = MertonModel::new(asset_value, asset_vol, debt_barrier, risk_free_rate)
        .map_err(to_js_err)?;
    serde_json::to_string(&model).map_err(to_js_err)
}

/// Build a CreditGrades structural model JSON payload.
#[wasm_bindgen(js_name = creditGradesModelJson)]
pub fn credit_grades_model_json(
    equity_value: f64,
    equity_vol: f64,
    total_debt: f64,
    risk_free_rate: f64,
    barrier_uncertainty: f64,
    mean_recovery: f64,
) -> Result<String, JsValue> {
    let model = MertonModel::credit_grades(
        equity_value,
        equity_vol,
        total_debt,
        risk_free_rate,
        barrier_uncertainty,
        mean_recovery,
    )
    .map_err(to_js_err)?;
    serde_json::to_string(&model).map_err(to_js_err)
}

/// Compute structural default probability from model JSON.
#[wasm_bindgen(js_name = mertonDefaultProbability)]
pub fn merton_default_probability(model_json: &str, horizon: f64) -> Result<f64, JsValue> {
    let model: MertonModel = serde_json::from_str(model_json).map_err(to_js_err)?;
    Ok(model.default_probability(horizon))
}

/// Compute distance-to-default from a Merton model JSON payload.
///
/// Distance-to-default is `ln(V/B)/(sigma*sqrt(T))` plus drift adjustments.
/// Lower values indicate higher default risk.
#[wasm_bindgen(js_name = mertonDistanceToDefault)]
pub fn merton_distance_to_default(model_json: &str, horizon: f64) -> Result<f64, JsValue> {
    let model: MertonModel = serde_json::from_str(model_json).map_err(to_js_err)?;
    Ok(model.distance_to_default(horizon))
}

/// Compute the implied credit spread (per year) from a Merton model JSON
/// payload, given a recovery rate. Matches the structural-model-implied
/// spread used to back into a hazard curve.
#[wasm_bindgen(js_name = mertonImpliedSpread)]
pub fn merton_implied_spread(
    model_json: &str,
    horizon: f64,
    recovery: f64,
) -> Result<f64, JsValue> {
    let model: MertonModel = serde_json::from_str(model_json).map_err(to_js_err)?;
    Ok(model.implied_spread(horizon, recovery))
}

/// Evaluate a `DynamicRecoverySpec` JSON payload at a given accreted
/// notional, returning the implied recovery rate. Result is clamped to
/// `[0, base_recovery]`.
#[wasm_bindgen(js_name = dynamicRecoveryAtNotional)]
pub fn dynamic_recovery_at_notional(spec_json: &str, notional: f64) -> Result<f64, JsValue> {
    let spec: DynamicRecoverySpec = serde_json::from_str(spec_json).map_err(to_js_err)?;
    Ok(spec.recovery_at_notional(notional))
}

/// Evaluate an `EndogenousHazardSpec` JSON payload at a given leverage
/// level, returning the implied hazard rate. Floored at 0.
#[wasm_bindgen(js_name = endogenousHazardAtLeverage)]
pub fn endogenous_hazard_at_leverage(spec_json: &str, leverage: f64) -> Result<f64, JsValue> {
    let spec: EndogenousHazardSpec = serde_json::from_str(spec_json).map_err(to_js_err)?;
    Ok(spec.hazard_at_leverage(leverage))
}

/// Convenience evaluator: hazard rate after a PIK accrual updates the
/// outstanding notional. Computes leverage = `accreted_notional / asset_value`
/// then evaluates the hazard mapping.
#[wasm_bindgen(js_name = endogenousHazardAfterPikAccrual)]
pub fn endogenous_hazard_after_pik_accrual(
    spec_json: &str,
    accreted_notional: f64,
    asset_value: f64,
) -> Result<f64, JsValue> {
    let spec: EndogenousHazardSpec = serde_json::from_str(spec_json).map_err(to_js_err)?;
    Ok(spec.hazard_after_pik_accrual(accreted_notional, asset_value))
}

/// Build a constant dynamic-recovery spec JSON payload.
#[wasm_bindgen(js_name = dynamicRecoveryConstantJson)]
pub fn dynamic_recovery_constant_json(recovery: f64) -> Result<String, JsValue> {
    let spec = DynamicRecoverySpec::constant(recovery).map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Build an endogenous hazard power-law spec JSON payload.
#[wasm_bindgen(js_name = endogenousHazardPowerLawJson)]
pub fn endogenous_hazard_power_law_json(
    base_hazard: f64,
    base_leverage: f64,
    exponent: f64,
) -> Result<String, JsValue> {
    let spec =
        EndogenousHazardSpec::power_law(base_hazard, base_leverage, exponent).map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Build a credit-state JSON payload for toggle-exercise decisions.
#[wasm_bindgen(js_name = creditStateJson)]
pub fn credit_state_json(
    hazard_rate: f64,
    leverage: f64,
    accreted_notional: f64,
    coupon_due: f64,
    distance_to_default: Option<f64>,
    asset_value: Option<f64>,
) -> Result<String, JsValue> {
    let state = CreditState {
        hazard_rate,
        distance_to_default,
        leverage,
        accreted_notional,
        coupon_due,
        asset_value,
    };
    serde_json::to_string(&state).map_err(to_js_err)
}

/// Build a threshold toggle-exercise model JSON payload.
#[wasm_bindgen(js_name = toggleExerciseThresholdJson)]
pub fn toggle_exercise_threshold_json(
    variable: &str,
    threshold: f64,
    direction: &str,
) -> Result<String, JsValue> {
    let variable = parse_credit_state_variable(variable)?;
    let direction = parse_threshold_direction(direction)?;
    let model = ToggleExerciseModel::threshold(variable, threshold, direction);
    serde_json::to_string(&model).map_err(to_js_err)
}

/// Build an optimal toggle-exercise model JSON payload.
#[wasm_bindgen(js_name = toggleExerciseOptimalJson)]
pub fn toggle_exercise_optimal_json(
    nested_paths: usize,
    equity_discount_rate: f64,
    asset_vol: f64,
    risk_free_rate: f64,
    horizon: f64,
) -> Result<String, JsValue> {
    let model = ToggleExerciseModel::OptimalExercise(OptimalToggle {
        nested_paths,
        equity_discount_rate,
        asset_vol,
        risk_free_rate,
        horizon,
    });
    serde_json::to_string(&model).map_err(to_js_err)
}

/// Example tagged `CreditDefaultSwap` instrument JSON.
#[wasm_bindgen(js_name = creditDefaultSwapExampleJson)]
pub fn credit_default_swap_example_json() -> Result<String, JsValue> {
    serde_json::to_string(&InstrumentJson::CreditDefaultSwap(
        CreditDefaultSwap::example(),
    ))
    .map_err(to_js_err)
}

/// Example tagged `CDSIndex` instrument JSON.
#[wasm_bindgen(js_name = cdsIndexExampleJson)]
pub fn cds_index_example_json() -> Result<String, JsValue> {
    serde_json::to_string(&InstrumentJson::CDSIndex(CDSIndex::example())).map_err(to_js_err)
}

/// Example tagged `CDSTranche` instrument JSON.
#[wasm_bindgen(js_name = cdsTrancheExampleJson)]
pub fn cds_tranche_example_json() -> Result<String, JsValue> {
    serde_json::to_string(&InstrumentJson::CDSTranche(CDSTranche::example())).map_err(to_js_err)
}

/// Example tagged `CDSOption` instrument JSON.
#[wasm_bindgen(js_name = cdsOptionExampleJson)]
pub fn cds_option_example_json() -> Result<String, JsValue> {
    let option = CDSOption::example().map_err(to_js_err)?;
    serde_json::to_string(&InstrumentJson::CDSOption(option)).map_err(to_js_err)
}

fn parse_credit_state_variable(value: &str) -> Result<CreditStateVariable, JsValue> {
    match value {
        "hazard_rate" => Ok(CreditStateVariable::HazardRate),
        "distance_to_default" => Ok(CreditStateVariable::DistanceToDefault),
        "leverage" => Ok(CreditStateVariable::Leverage),
        other => Err(JsValue::from_str(&format!(
            "unknown credit state variable: {other}"
        ))),
    }
}

fn parse_threshold_direction(value: &str) -> Result<ThresholdDirection, JsValue> {
    match value {
        "above" => Ok(ThresholdDirection::Above),
        "below" => Ok(ThresholdDirection::Below),
        other => Err(JsValue::from_str(&format!(
            "unknown threshold direction: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_model_key_recognizes_standard_keys() {
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("discounting").expect("ok"),
            finstack_valuations::pricer::ModelKey::Discounting
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("tree").expect("ok"),
            finstack_valuations::pricer::ModelKey::Tree
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("black76").expect("ok"),
            finstack_valuations::pricer::ModelKey::Black76
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("hull_white_1f").expect("ok"),
            finstack_valuations::pricer::ModelKey::HullWhite1F
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("hazard_rate").expect("ok"),
            finstack_valuations::pricer::ModelKey::HazardRate
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("normal").expect("ok"),
            finstack_valuations::pricer::ModelKey::Normal
        );
        assert_eq!(
            finstack_valuations::pricer::parse_model_key("monte_carlo_gbm").expect("ok"),
            finstack_valuations::pricer::ModelKey::MonteCarloGBM
        );
    }

    pub(crate) fn bond_instrument_json() -> String {
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

    pub(crate) fn bermudan_swaption_json() -> String {
        use finstack_valuations::instruments::rates::swaption::BermudanSwaption;
        use finstack_valuations::instruments::InstrumentJson;

        serde_json::to_string(&InstrumentJson::BermudanSwaption(
            BermudanSwaption::example(),
        ))
        .expect("serialize")
    }

    pub(crate) fn tarn_json() -> String {
        use finstack_core::dates::{Date, DayCount, Tenor};
        use finstack_core::money::Money;
        use finstack_core::types::{CurveId, InstrumentId};
        use finstack_valuations::instruments::rates::tarn::Tarn;
        use finstack_valuations::instruments::{InstrumentJson, PricingOverrides};
        use time::Month;

        let mut pricing_overrides = PricingOverrides::default();
        pricing_overrides.model_config.mc_paths = Some(32);
        pricing_overrides.model_config.mean_reversion = Some(0.05);
        pricing_overrides.model_config.tree_volatility = Some(1e-12);

        let tarn = Tarn {
            id: InstrumentId::new("TARN-WASM-E2E"),
            fixed_rate: 0.06,
            coupon_floor: 0.0,
            target_coupon: 1.0,
            notional: Money::new(1_000_000.0, finstack_core::currency::Currency::USD),
            coupon_dates: vec![
                Date::from_calendar_date(2025, Month::January, 1).expect("date"),
                Date::from_calendar_date(2025, Month::July, 1).expect("date"),
                Date::from_calendar_date(2026, Month::January, 1).expect("date"),
                Date::from_calendar_date(2026, Month::July, 1).expect("date"),
            ],
            floating_tenor: Tenor::semi_annual(),
            floating_index_id: CurveId::new("USD-SOFR-6M"),
            discount_curve_id: CurveId::new("USD-OIS"),
            day_count: DayCount::Act365F,
            pricing_overrides,
            attributes: Default::default(),
        };
        serde_json::to_string(&InstrumentJson::Tarn(tarn)).expect("serialize")
    }

    pub(crate) fn snowball_json() -> String {
        use finstack_core::dates::{Date, DayCount, Tenor};
        use finstack_core::money::Money;
        use finstack_core::types::{CurveId, InstrumentId};
        use finstack_valuations::instruments::rates::snowball::{Snowball, SnowballVariant};
        use finstack_valuations::instruments::{InstrumentJson, PricingOverrides};
        use time::Month;

        let mut pricing_overrides = PricingOverrides::default();
        pricing_overrides.model_config.mc_paths = Some(32);
        pricing_overrides.model_config.mean_reversion = Some(0.05);
        pricing_overrides.model_config.tree_volatility = Some(1e-12);

        let snowball = Snowball {
            id: InstrumentId::new("SNOWBALL-WASM-E2E"),
            variant: SnowballVariant::Snowball,
            initial_coupon: 0.03,
            fixed_rate: 0.05,
            leverage: 1.0,
            coupon_floor: 0.0,
            coupon_cap: None,
            notional: Money::new(1_000_000.0, finstack_core::currency::Currency::USD),
            coupon_dates: vec![
                Date::from_calendar_date(2025, Month::January, 1).expect("date"),
                Date::from_calendar_date(2025, Month::July, 1).expect("date"),
                Date::from_calendar_date(2026, Month::January, 1).expect("date"),
                Date::from_calendar_date(2026, Month::July, 1).expect("date"),
            ],
            floating_index_id: CurveId::new("USD-SOFR-6M"),
            floating_tenor: Tenor::semi_annual(),
            discount_curve_id: CurveId::new("USD-OIS"),
            callable: None,
            day_count: DayCount::Act365F,
            pricing_overrides,
            attributes: Default::default(),
        };
        serde_json::to_string(&InstrumentJson::Snowball(snowball)).expect("serialize")
    }

    pub(crate) fn inverse_floater_json() -> String {
        use finstack_core::dates::{Date, DayCount, Tenor};
        use finstack_core::money::Money;
        use finstack_core::types::{CurveId, InstrumentId};
        use finstack_valuations::instruments::rates::snowball::{Snowball, SnowballVariant};
        use finstack_valuations::instruments::{InstrumentJson, PricingOverrides};
        use time::Month;

        let inverse_floater = Snowball {
            id: InstrumentId::new("INV-FLOATER-WASM-E2E"),
            variant: SnowballVariant::InverseFloater,
            initial_coupon: 0.0,
            fixed_rate: 0.08,
            leverage: 1.5,
            coupon_floor: 0.0,
            coupon_cap: Some(0.10),
            notional: Money::new(500_000.0, finstack_core::currency::Currency::USD),
            coupon_dates: vec![
                Date::from_calendar_date(2025, Month::January, 1).expect("date"),
                Date::from_calendar_date(2025, Month::July, 1).expect("date"),
                Date::from_calendar_date(2026, Month::January, 1).expect("date"),
                Date::from_calendar_date(2026, Month::July, 1).expect("date"),
            ],
            floating_index_id: CurveId::new("USD-SOFR-6M"),
            floating_tenor: Tenor::semi_annual(),
            discount_curve_id: CurveId::new("USD-OIS"),
            callable: None,
            day_count: DayCount::Act365F,
            pricing_overrides: PricingOverrides::default(),
            attributes: Default::default(),
        };
        serde_json::to_string(&InstrumentJson::Snowball(inverse_floater)).expect("serialize")
    }

    pub(crate) fn callable_range_accrual_json() -> String {
        use finstack_core::dates::{Date, DayCount};
        use finstack_core::money::Money;
        use finstack_core::types::{CurveId, InstrumentId};
        use finstack_valuations::instruments::rates::callable_range_accrual::CallableRangeAccrual;
        use finstack_valuations::instruments::rates::range_accrual::{BoundsType, RangeAccrual};
        use finstack_valuations::instruments::rates::shared::bermudan_call::BermudanCallProvision;
        use finstack_valuations::instruments::{InstrumentJson, PricingOverrides};
        use time::Month;

        let mut pricing_overrides = PricingOverrides::default();
        pricing_overrides.model_config.mc_paths = Some(8);
        pricing_overrides.model_config.mean_reversion = Some(0.05);
        pricing_overrides.model_config.tree_volatility = Some(1e-12);

        let range_accrual = RangeAccrual::builder()
            .id(InstrumentId::new("RA-WASM-E2E"))
            .underlying_ticker("SOFR".to_string())
            .observation_dates(vec![
                Date::from_calendar_date(2025, Month::July, 1).expect("date"),
                Date::from_calendar_date(2026, Month::January, 1).expect("date"),
                Date::from_calendar_date(2026, Month::July, 1).expect("date"),
            ])
            .lower_bound(0.02)
            .upper_bound(0.04)
            .bounds_type(BoundsType::Absolute)
            .coupon_rate(0.06)
            .notional(Money::new(
                1_000_000.0,
                finstack_core::currency::Currency::USD,
            ))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SOFR-RATE".into())
            .vol_surface_id(CurveId::new("SOFR-VOL"))
            .div_yield_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Default::default())
            .payment_date_opt(None)
            .past_fixings_in_range_opt(None)
            .total_past_observations_opt(None)
            .build()
            .expect("range accrual");

        let callable = CallableRangeAccrual {
            id: InstrumentId::new("CALLABLE-RA-WASM-E2E"),
            range_accrual,
            call_provision: BermudanCallProvision::new(
                vec![Date::from_calendar_date(2025, Month::July, 1).expect("date")],
                1.0,
                0,
            ),
            pricing_overrides,
            attributes: Default::default(),
        };
        serde_json::to_string(&InstrumentJson::CallableRangeAccrual(Box::new(callable)))
            .expect("serialize")
    }

    pub(crate) fn cms_spread_option_json() -> String {
        use finstack_core::dates::{Date, DayCount, Tenor, TenorUnit};
        use finstack_core::money::Money;
        use finstack_core::types::{CurveId, InstrumentId};
        use finstack_valuations::instruments::rates::cms_spread_option::{
            CmsSpreadOption, CmsSpreadOptionType,
        };
        use finstack_valuations::instruments::{InstrumentJson, PricingOverrides};
        use time::Month;

        let option = CmsSpreadOption {
            id: InstrumentId::new("CMS-SPREAD-WASM-E2E"),
            long_cms_tenor: Tenor::new(10, TenorUnit::Years),
            short_cms_tenor: Tenor::new(2, TenorUnit::Years),
            strike: 0.005,
            option_type: CmsSpreadOptionType::Call,
            notional: Money::new(10_000_000.0, finstack_core::currency::Currency::USD),
            expiry_date: Date::from_calendar_date(2026, Month::January, 1).expect("date"),
            payment_date: Date::from_calendar_date(2026, Month::January, 5).expect("date"),
            long_vol_surface_id: CurveId::new("USD-SWAPTION-VOL-10Y"),
            short_vol_surface_id: CurveId::new("USD-SWAPTION-VOL-2Y"),
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            spread_correlation: 0.5,
            day_count: DayCount::Act365F,
            pricing_overrides: PricingOverrides::default(),
            attributes: Default::default(),
        };
        serde_json::to_string(&InstrumentJson::CmsSpreadOption(option)).expect("serialize")
    }

    pub(crate) fn market_context_json() -> String {
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

    pub(crate) fn tarn_market_context_json() -> String {
        use finstack_core::dates::DayCount;
        use finstack_core::market_data::context::MarketContext;
        use finstack_core::market_data::scalars::MarketScalar;
        use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
        let base = time::Date::from_calendar_date(2025, time::Month::January, 1).expect("date");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (6.0, (-0.02_f64 * 6.0).exp())])
            .build()
            .expect("discount curve");
        let fwd = ForwardCurve::builder("USD-SOFR-6M", 0.5)
            .base_date(base)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 0.03), (6.0, 0.03)])
            .build()
            .expect("forward curve");
        let ctx = MarketContext::new()
            .insert(disc)
            .insert(fwd)
            .insert_price("SOFR-RATE", MarketScalar::Unitless(0.03));
        serde_json::to_string(&ctx).expect("serialize")
    }

    pub(crate) fn cms_spread_market_context_json() -> String {
        use finstack_core::dates::DayCount;
        use finstack_core::market_data::context::MarketContext;
        use finstack_core::market_data::surfaces::VolCube;
        use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
        use finstack_core::math::volatility::sabr::SabrParams;

        fn sabr_cube(id: &str, alpha: f64, forward: f64) -> VolCube {
            let params = SabrParams::new(alpha, 0.5, -0.20, 0.40).expect("valid SABR params");
            VolCube::builder(id)
                .expiries(&[0.25, 1.0, 5.0])
                .tenors(&[2.0, 10.0])
                .node(params, forward)
                .node(params, forward)
                .node(params, forward)
                .node(params, forward)
                .node(params, forward)
                .node(params, forward)
                .build()
                .expect("vol cube")
        }

        let base = time::Date::from_calendar_date(2025, time::Month::January, 1).expect("date");
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 1.0), (30.0, (-0.035_f64 * 30.0).exp())])
            .build()
            .expect("discount curve");
        let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(base)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 0.025), (2.0, 0.030), (10.0, 0.045), (30.0, 0.055)])
            .build()
            .expect("forward curve");
        let ctx = MarketContext::new()
            .insert(disc)
            .insert(fwd)
            .insert_vol_cube(sabr_cube("USD-SWAPTION-VOL-10Y", 0.035, 0.045))
            .insert_vol_cube(sabr_cube("USD-SWAPTION-VOL-2Y", 0.035, 0.030));
        serde_json::to_string(&ctx).expect("serialize")
    }

    fn amount_from_result(parsed: &serde_json::Value) -> f64 {
        parsed["value"]["amount"]
            .as_f64()
            .or_else(|| {
                parsed["value"]["amount"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
            })
            .expect("amount")
    }

    #[test]
    fn validate_instrument_json_bond() {
        let json = bond_instrument_json();
        let canonical = validate_instrument_json(&json).expect("validate");
        assert!(!canonical.is_empty());
    }

    #[test]
    fn validate_instrument_json_bermudan_swaption() {
        let json = bermudan_swaption_json();
        let canonical = validate_instrument_json(&json).expect("validate");
        let parsed: serde_json::Value = serde_json::from_str(&canonical).expect("json");
        assert_eq!(parsed["type"], "bermudan_swaption");
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
    fn price_instrument_tarn_hull_white_mc() {
        let inst = tarn_json();
        let mkt = tarn_market_context_json();
        let result = price_instrument(&inst, &mkt, "2025-01-01", "monte_carlo_hull_white_1f")
            .expect("price");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        let amount = amount_from_result(&parsed);
        assert!(amount > 0.0);
        assert_eq!(parsed["measures"]["mc_num_paths"], 32.0);
    }

    #[test]
    fn price_instrument_snowball_hull_white_mc() {
        let inst = snowball_json();
        let mkt = tarn_market_context_json();
        let result = price_instrument(&inst, &mkt, "2025-01-01", "monte_carlo_hull_white_1f")
            .expect("price");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(amount_from_result(&parsed) > 0.0);
        assert_eq!(parsed["measures"]["mc_num_paths"], 32.0);
    }

    #[test]
    fn price_instrument_inverse_floater_discounting() {
        let inst = inverse_floater_json();
        let mkt = tarn_market_context_json();
        let result = price_instrument(&inst, &mkt, "2025-01-01", "discounting").expect("price");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(amount_from_result(&parsed) > 0.0);
    }

    #[test]
    fn price_instrument_callable_range_accrual_hull_white_mc() {
        let inst = callable_range_accrual_json();
        let mkt = tarn_market_context_json();
        let result = price_instrument(&inst, &mkt, "2025-01-01", "monte_carlo_hull_white_1f")
            .expect("price");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(amount_from_result(&parsed) > 0.0);
        assert_eq!(parsed["measures"]["mc_num_paths"], 8.0);
    }

    #[test]
    fn price_instrument_cms_spread_option_static_replication() {
        let inst = cms_spread_option_json();
        let mkt = cms_spread_market_context_json();
        let result =
            price_instrument(&inst, &mkt, "2025-01-01", "static_replication").expect("price");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert!(amount_from_result(&parsed) > 0.0);
        assert!(
            parsed["measures"]["cms_spread_forward"]
                .as_f64()
                .expect("cms spread forward")
                > 0.0
        );
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

    // -------------------------------------------------------------------------
    // Credit-model evaluator parity (mirrors finstack-py PyMertonModel etc.)
    // -------------------------------------------------------------------------

    #[test]
    fn merton_distance_to_default_matches_native() {
        let json = merton_model_json(100.0, 0.20, 80.0, 0.05).expect("merton json");
        let dd_wasm = merton_distance_to_default(&json, 1.0).expect("dd");
        let model = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("merton");
        let dd_native = model.distance_to_default(1.0);
        assert!(
            (dd_wasm - dd_native).abs() < 1e-12,
            "WASM dd ({dd_wasm}) must match native ({dd_native})"
        );
    }

    #[test]
    fn merton_implied_spread_matches_native() {
        let json = merton_model_json(100.0, 0.20, 80.0, 0.05).expect("merton json");
        let spread_wasm = merton_implied_spread(&json, 5.0, 0.40).expect("spread");
        let model = MertonModel::new(100.0, 0.20, 80.0, 0.05).expect("merton");
        let spread_native = model.implied_spread(5.0, 0.40);
        assert!(
            (spread_wasm - spread_native).abs() < 1e-12,
            "WASM spread ({spread_wasm}) must match native ({spread_native})"
        );
    }

    #[test]
    fn dynamic_recovery_at_notional_matches_native() {
        let json = dynamic_recovery_constant_json(0.40).expect("spec json");
        let r_wasm = dynamic_recovery_at_notional(&json, 100.0).expect("r");
        let spec = DynamicRecoverySpec::constant(0.40).expect("spec");
        let r_native = spec.recovery_at_notional(100.0);
        assert!((r_wasm - r_native).abs() < 1e-12);
    }

    #[test]
    fn endogenous_hazard_at_leverage_matches_native() {
        let json =
            endogenous_hazard_power_law_json(0.10, 1.5, 2.5).expect("spec json");
        let h_wasm = endogenous_hazard_at_leverage(&json, 2.0).expect("h");
        let spec = EndogenousHazardSpec::power_law(0.10, 1.5, 2.5).expect("spec");
        let h_native = spec.hazard_at_leverage(2.0);
        assert!((h_wasm - h_native).abs() < 1e-12);
    }

    #[test]
    fn endogenous_hazard_after_pik_accrual_matches_native() {
        let json =
            endogenous_hazard_power_law_json(0.10, 1.5, 2.5).expect("spec json");
        let h_wasm =
            endogenous_hazard_after_pik_accrual(&json, 120.0, 66.67).expect("h");
        let spec = EndogenousHazardSpec::power_law(0.10, 1.5, 2.5).expect("spec");
        let h_native = spec.hazard_after_pik_accrual(120.0, 66.67);
        assert!((h_wasm - h_native).abs() < 1e-12);
    }
}
