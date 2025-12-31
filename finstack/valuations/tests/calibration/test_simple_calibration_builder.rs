//! Tests for the plan-driven calibration v2 API.

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::ExtrapolationPolicy;
use finstack_core::types::Currency;
use finstack_core::HashMap;
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelope, CalibrationPlan, CalibrationStep, DiscountCurveParams, StepParams,
};
use finstack_valuations::calibration::CalibrationMethod;
use finstack_valuations::market::conventions::ids::IndexId;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote;
use time::Month;

#[test]
fn missing_quote_set_fails_fast() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let currency = Currency::USD;

    let plan = CalibrationPlan {
        id: "plan".to_string(),
        description: None,
        quote_sets: HashMap::default(),
        settings: Default::default(),
        steps: vec![CalibrationStep {
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

    let envelope = CalibrationEnvelope {
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

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::default();
    quote_sets.insert(
        "usd_ois".to_string(),
        vec![MarketQuote::Rates(RateQuote::Deposit {
            id: QuoteId::new(format!("DEP-{:?}", base_date + time::Duration::days(30))),
            index: IndexId::new("USD-Deposit"),
            pillar: Pillar::Date(base_date + time::Duration::days(30)),
            rate: 0.05,
        })],
    );

    let plan = CalibrationPlan {
        id: "plan".to_string(),
        description: Some("serde smoke".to_string()),
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStep {
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

    let envelope = CalibrationEnvelope {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: None,
    };

    let json = serde_json::to_string_pretty(&envelope).expect("serialize");
    let decoded: CalibrationEnvelope = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded.schema, "finstack.calibration/2");
    assert_eq!(decoded.plan.steps.len(), 1);
}
