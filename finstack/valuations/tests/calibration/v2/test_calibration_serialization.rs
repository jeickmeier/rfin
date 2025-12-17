//! Serialization tests for the plan-driven calibration v2 API.
//!
//! v2 introduces a strict JSON contract for plan-driven execution:
//! - `CalibrationEnvelopeV2` and `CalibrationPlanV2` (`deny_unknown_fields`)
//! - `StepParams` for discount/forward/hazard/inflation/vol/swaption/base-correlation
//! - Domain quotes (`MarketQuote` + concrete quote enums)

use finstack_core::dates::Date;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::math::interp::ExtrapolationPolicy;
use finstack_core::types::{Currency, CurveId};
use finstack_valuations::calibration::v2::api::schema::{
    BaseCorrelationParams, CalibrationEnvelopeV2, CalibrationMethod, CalibrationPlanV2,
    CalibrationStepV2, DiscountCurveParams, ForwardCurveParams, HazardCurveParams,
    InflationCurveParams, StepParams, SurfaceExtrapolationPolicy, SwaptionVolParams,
    VolSurfaceParams,
};
use finstack_valuations::calibration::v2::domain::quotes::{
    CreditQuote, MarketQuote, RatesQuote, VolQuote,
};
use std::collections::HashMap;
use time::Month;

fn maybe_print_json(json: &str) {
    if std::env::var("FINSTACK_TEST_LOG_JSON").is_ok() {
        println!("JSON representation:\n{}\n", json);
    }
}

fn roundtrip_json<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    let json = serde_json::to_string_pretty(value).expect("serialize");
    maybe_print_json(&json);
    serde_json::from_str(&json).expect("deserialize")
}

#[test]
fn envelope_v2_roundtrips() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let currency = Currency::USD;

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "rates".to_string(),
        vec![MarketQuote::Rates(RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.05,
            conventions: Default::default(),
        })],
    );

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "step_1".to_string(),
            quote_set: "rates".to_string(),
            params: StepParams::Discount(DiscountCurveParams {
                curve_id: CurveId::from("USD-OIS"),
                currency,
                base_date,
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
                extrapolation: ExtrapolationPolicy::FlatForward,
                pricing_discount_id: None,
                pricing_forward_id: None,
                conventions: Default::default(),
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: None,
    };

    let decoded = roundtrip_json(&envelope);
    assert_eq!(decoded.schema, "finstack.calibration/2");
    assert_eq!(decoded.plan.steps.len(), 1);
}

#[test]
fn step_params_v2_roundtrip_for_all_variants() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let currency = Currency::USD;

    let discount = StepParams::Discount(DiscountCurveParams {
        curve_id: "USD-OIS".into(),
        currency,
        base_date,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        extrapolation: ExtrapolationPolicy::FlatForward,
        pricing_discount_id: None,
        pricing_forward_id: None,
        conventions: Default::default(),
    });
    let _ = roundtrip_json(&discount);

    let forward = StepParams::Forward(ForwardCurveParams {
        curve_id: "USD-3M".into(),
        currency,
        base_date,
        tenor_years: 0.25,
        discount_curve_id: "USD-OIS".into(),
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        conventions: Default::default(),
    });
    let _ = roundtrip_json(&forward);

    let hazard = StepParams::Hazard(HazardCurveParams {
        curve_id: "ACME-SENIOR".into(),
        entity: "ACME".to_string(),
        seniority: Seniority::Senior,
        currency,
        base_date,
        discount_curve_id: "USD-OIS".into(),
        recovery_rate: 0.40,
        notional: 1.0,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        par_interp: finstack_core::market_data::term_structures::ParInterp::Linear,
    });
    let _ = roundtrip_json(&hazard);

    let inflation = StepParams::Inflation(InflationCurveParams {
        curve_id: "USD-CPI".into(),
        currency,
        base_date,
        discount_curve_id: "USD-OIS".into(),
        index: "USA-CPI-U".to_string(),
        observation_lag: "3M".to_string(),
        base_cpi: 100.0,
        notional: 1.0,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
    });
    let _ = roundtrip_json(&inflation);

    let vol_surface = StepParams::VolSurface(VolSurfaceParams {
        surface_id: "SPX-VOL".to_string(),
        base_date,
        underlying_id: "SPX".to_string(),
        model: "SABR".to_string(),
        discount_curve_id: Some("USD-OIS".into()),
        beta: 0.5,
        target_expiries: vec![0.5, 1.0],
        target_strikes: vec![0.9, 1.0, 1.1],
        spot_override: Some(100.0),
        dividend_yield_override: Some(0.01),
        expiry_extrapolation: SurfaceExtrapolationPolicy::Error,
    });
    let _ = roundtrip_json(&vol_surface);

    let swaption_vol = StepParams::SwaptionVol(SwaptionVolParams {
        surface_id: "USD-SWPT".to_string(),
        base_date,
        discount_curve_id: "USD-OIS".into(),
        forward_id: None,
        currency,
        vol_convention: Default::default(),
        atm_convention: Default::default(),
        sabr_beta: 0.0,
        target_expiries: vec![1.0, 2.0],
        target_tenors: vec![1.0, 5.0],
        sabr_interpolation: Default::default(),
        calendar_id: None,
        fixed_day_count: None,
        vol_tolerance: None,
        sabr_tolerance: None,
        sabr_extrapolation: SurfaceExtrapolationPolicy::Error,
        allow_sabr_missing_bucket_fallback: false,
    });
    let _ = roundtrip_json(&swaption_vol);

    let base_corr = StepParams::BaseCorrelation(BaseCorrelationParams {
        index_id: "CDX".to_string(),
        series: 40,
        maturity_years: 5.0,
        base_date,
        discount_curve_id: "USD-OIS".into(),
        currency,
        notional: 1.0,
        payment_frequency: Some(Tenor::quarterly()),
        day_count: Some(DayCount::Act360),
        business_day_convention: Some(BusinessDayConvention::Following),
        calendar_id: Some("usny".to_string()),
        detachment_points: vec![0.03, 0.07, 0.1],
        use_imm_dates: true,
    });
    let _ = roundtrip_json(&base_corr);
}

#[test]
fn market_quote_roundtrip_smoke() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();

    let rq = MarketQuote::Rates(RatesQuote::Deposit {
        maturity: base_date + time::Duration::days(30),
        rate: 0.05,
        conventions: Default::default(),
    });
    let _ = roundtrip_json(&rq);

    let cq = MarketQuote::Credit(CreditQuote::CDS {
        entity: "ACME".to_string(),
        maturity: base_date + time::Duration::days(365 * 5),
        spread_bp: 120.0,
        recovery_rate: 0.40,
        currency: Currency::USD,
        conventions: Default::default(),
    });
    let _ = roundtrip_json(&cq);

    let vq = MarketQuote::Vol(VolQuote::SwaptionVol {
        expiry: base_date + time::Duration::days(365),
        tenor: base_date + time::Duration::days(365 * 5),
        strike: 0.04,
        vol: 0.01,
        quote_type: "ATM".to_string(),
        conventions: Default::default(),
        fixed_leg_conventions: Default::default(),
        float_leg_conventions: Default::default(),
    });
    let _ = roundtrip_json(&vq);
}

#[test]
fn envelope_unknown_field_is_rejected() {
    let payload = r#"{
        "schema": "finstack.calibration/2",
        "plan": {
            "id": "p",
            "quote_sets": {},
            "steps": [],
            "settings": {},
            "oops": true
        }
    }"#;
    serde_json::from_str::<CalibrationEnvelopeV2>(payload)
        .expect_err("unknown field should be rejected");
}
