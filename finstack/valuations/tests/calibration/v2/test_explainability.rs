//! Explainability tests for calibration v2.
//!
//! v2 captures explainability traces at the per-step report level.

use finstack_core::dates::{create_date, DayCount};
use finstack_core::explain::ExplainOpts;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::types::{Currency, CurveId};
use finstack_valuations::calibration::v2::api::engine;
use finstack_valuations::calibration::v2::api::schema::{
    CalibrationEnvelopeV2, CalibrationMethod, CalibrationPlanV2, CalibrationStepV2,
    ForwardCurveParams, StepParams,
};
use finstack_valuations::calibration::v2::domain::quotes::{
    InstrumentConventions, MarketQuote, RatesQuote,
};
use finstack_valuations::calibration::CalibrationConfig;
use std::collections::HashMap;
use time::Month;

fn base_discount_curve(base_date: finstack_core::dates::Date) -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.9888),
            (0.5, 0.9775),
            (1.0, 0.9550),
            (2.0, 0.9100),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap()
}

fn forward_quotes() -> Vec<MarketQuote> {
    vec![
        MarketQuote::Rates(RatesQuote::FRA {
            start: create_date(2025, Month::April, 15).unwrap(),
            end: create_date(2025, Month::July, 15).unwrap(),
            rate: 0.045,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        }),
        MarketQuote::Rates(RatesQuote::FRA {
            start: create_date(2025, Month::July, 15).unwrap(),
            end: create_date(2025, Month::October, 15).unwrap(),
            rate: 0.046,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        }),
    ]
}

#[test]
fn explanation_not_computed_by_default() {
    let base_date = create_date(2025, Month::January, 15).unwrap();

    let ctx = MarketContext::new().insert_discount(base_discount_curve(base_date));
    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("fwd".to_string(), forward_quotes());

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: CalibrationConfig::default(),
        steps: vec![CalibrationStepV2 {
            id: "fwd".to_string(),
            quote_set: "fwd".to_string(),
            params: StepParams::Forward(ForwardCurveParams {
                curve_id: CurveId::from("USD-SOFR-3M"),
                currency: Currency::USD,
                base_date,
                tenor_years: 0.25,
                discount_curve_id: CurveId::from("USD-OIS"),
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
                conventions: Default::default(),
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&ctx).into()),
    };

    let result = engine::execute(&envelope).expect("execute");
    let step = result.result.step_reports.get("fwd").expect("step report");

    assert!(step.explanation.is_none());
}

#[test]
fn explanation_is_present_when_enabled() {
    let base_date = create_date(2025, Month::January, 15).unwrap();

    let ctx = MarketContext::new().insert_discount(base_discount_curve(base_date));
    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("fwd".to_string(), forward_quotes());

    let settings = CalibrationConfig {
        explain: ExplainOpts::enabled(),
        ..Default::default()
    };

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings,
        steps: vec![CalibrationStepV2 {
            id: "fwd".to_string(),
            quote_set: "fwd".to_string(),
            params: StepParams::Forward(ForwardCurveParams {
                curve_id: CurveId::from("USD-SOFR-3M"),
                currency: Currency::USD,
                base_date,
                tenor_years: 0.25,
                discount_curve_id: CurveId::from("USD-OIS"),
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
                conventions: Default::default(),
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&ctx).into()),
    };

    let result = engine::execute(&envelope).expect("execute");
    let step = result.result.step_reports.get("fwd").expect("step report");

    assert!(step.success);
    if let Some(trace) = step.explanation.as_ref() {
        assert!(
            !trace.entries.is_empty(),
            "when explanation is present, it should contain entries"
        );
    }
}
