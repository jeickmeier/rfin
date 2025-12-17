//! Tests for the plan-driven calibration v2 API.
//!
//! v1 used `CalibrationSpec` / `CalibrationStep`. v2 uses `CalibrationPlanV2`
//! and `CalibrationEnvelopeV2`, executed via `v2::api::engine`.

use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::ExtrapolationPolicy;
use finstack_core::types::Currency;
use finstack_valuations::calibration::v2::api::engine;
use finstack_valuations::calibration::v2::api::schema::{
    CalibrationEnvelopeV2, CalibrationMethod, CalibrationPlanV2, CalibrationStepV2,
    DiscountCurveParams, StepParams,
};
use finstack_valuations::calibration::v2::domain::quotes::{
    InstrumentConventions, MarketQuote, RatesQuote,
};
use std::collections::HashMap;
use time::Month;

#[test]
fn missing_quote_set_fails_fast() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let currency = Currency::USD;

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets: HashMap::new(),
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "step_1".to_string(),
            quote_set: "does_not_exist".to_string(),
            params: StepParams::Discount(DiscountCurveParams {
                curve_id: "USD-OIS".into(),
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
        initial_market: Some((&MarketContext::new()).into()),
    };

    let err = engine::execute(&envelope).expect_err("missing quote set should error");
    assert!(matches!(err, finstack_core::Error::Input(_)));
}

#[test]
fn plan_and_envelope_serde_roundtrip() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let currency = Currency::USD;

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "usd_ois".to_string(),
        vec![MarketQuote::Rates(RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.05,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        })],
    );

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: Some("serde smoke".to_string()),
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "step_1".to_string(),
            quote_set: "usd_ois".to_string(),
            params: StepParams::Discount(DiscountCurveParams {
                curve_id: "USD-OIS".into(),
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

    let json = serde_json::to_string_pretty(&envelope).expect("serialize");
    let decoded: CalibrationEnvelopeV2 = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded.schema, "finstack.calibration/2");
    assert_eq!(decoded.plan.steps.len(), 1);
}
