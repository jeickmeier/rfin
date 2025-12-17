//! Hazard curve calibration tests (v2).

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::math::interp::InterpStyle;
use finstack_core::types::{Currency, CurveId};
use finstack_valuations::calibration::v2::api::engine;
use finstack_valuations::calibration::v2::api::schema::{
    CalibrationEnvelopeV2, CalibrationMethod, CalibrationPlanV2, CalibrationStepV2,
    HazardCurveParams, StepParams,
};
use finstack_valuations::calibration::v2::domain::quotes::{CreditQuote, MarketQuote};
use std::collections::HashMap;
use time::Month;

fn create_test_discount_curve(base: Date) -> DiscountCurve {
    DiscountCurve::builder("TEST-DISC")
        .base_date(base)
        .knots(vec![
            (0.0, 1.0),
            (1.0, 0.98),
            (3.0, 0.94),
            (5.0, 0.88),
            (10.0, 0.75),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

#[test]
fn hazard_calibration_positive_rates() {
    // Use ISDA-friendly dates (IMM 20th) because v2 hazard bootstrapping builds
    // canonical CDS instruments under ISDA conventions.
    let base = Date::from_calendar_date(2025, Month::March, 20).unwrap();
    let currency = Currency::USD;

    let disc = create_test_discount_curve(base);
    let initial_market = MarketContext::new().insert_discount(disc);

    let conventions = finstack_valuations::calibration::v2::domain::quotes::InstrumentConventions::default()
        .with_day_count(DayCount::Act360)
        .with_payment_frequency(Tenor::quarterly())
        .with_settlement_days(0)
        .with_calendar_id("usny")
        .with_business_day_convention(BusinessDayConvention::ModifiedFollowing);

    let quotes = vec![
        MarketQuote::Credit(CreditQuote::CDS {
            entity: "ACME-Corp".to_string(),
            maturity: Date::from_calendar_date(2026, Month::March, 20).unwrap(),
            spread_bp: 100.0,
            recovery_rate: 0.40,
            currency,
            conventions: conventions.clone(),
        }),
        MarketQuote::Credit(CreditQuote::CDS {
            entity: "ACME-Corp".to_string(),
            maturity: Date::from_calendar_date(2028, Month::March, 20).unwrap(),
            spread_bp: 150.0,
            recovery_rate: 0.40,
            currency,
            conventions: conventions.clone(),
        }),
        MarketQuote::Credit(CreditQuote::CDS {
            entity: "ACME-Corp".to_string(),
            maturity: Date::from_calendar_date(2030, Month::March, 20).unwrap(),
            spread_bp: 200.0,
            recovery_rate: 0.40,
            currency,
            conventions,
        }),
    ];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("credit".to_string(), quotes);

    let hazard_id: CurveId = "ACME-Corp-SENIOR".into();

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "haz".to_string(),
            quote_set: "credit".to_string(),
            params: StepParams::Hazard(HazardCurveParams {
                curve_id: hazard_id.clone(),
                entity: "ACME-Corp".to_string(),
                seniority: Seniority::Senior,
                currency,
                base_date: base,
                discount_curve_id: "TEST-DISC".into(),
                recovery_rate: 0.40,
                notional: 1.0,
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&initial_market).into()),
    };

    let result = engine::execute(&envelope).expect("execute");
    assert!(result.result.report.success);

    let ctx = MarketContext::try_from(result.result.final_market).expect("restore context");
    let curve = ctx.get_hazard(hazard_id.as_str()).expect("hazard curve");

    for (_t, lambda) in curve.knot_points() {
        assert!(lambda > 0.0, "hazard rate should be positive, got {lambda}");
    }
}

#[test]
fn hazard_calibration_rejects_zero_spread() {
    let base = Date::from_calendar_date(2025, Month::March, 20).unwrap();
    let currency = Currency::USD;

    let disc = create_test_discount_curve(base);
    let initial_market = MarketContext::new().insert_discount(disc);

    let conventions = finstack_valuations::calibration::v2::domain::quotes::InstrumentConventions::default()
        .with_day_count(DayCount::Act360)
        .with_payment_frequency(Tenor::quarterly())
        .with_settlement_days(0)
        .with_calendar_id("usny")
        .with_business_day_convention(BusinessDayConvention::ModifiedFollowing);

    let quotes = vec![MarketQuote::Credit(CreditQuote::CDS {
        entity: "ZERO-SPREAD".to_string(),
        maturity: Date::from_calendar_date(2026, Month::March, 20).unwrap(),
        spread_bp: 0.0,
        recovery_rate: 0.40,
        currency,
        conventions,
    })];

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert("credit".to_string(), quotes);

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "haz".to_string(),
            quote_set: "credit".to_string(),
            params: StepParams::Hazard(HazardCurveParams {
                curve_id: "ZERO-SPREAD-SENIOR".into(),
                entity: "ZERO-SPREAD".to_string(),
                seniority: Seniority::Senior,
                currency,
                base_date: base,
                discount_curve_id: "TEST-DISC".into(),
                recovery_rate: 0.40,
                notional: 1.0,
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&initial_market).into()),
    };

    let err = engine::execute(&envelope).expect_err("zero spread should be invalid");
    assert!(matches!(
        err,
        finstack_core::Error::Validation(_)
            | finstack_core::Error::Input(_)
            | finstack_core::Error::Calibration { .. }
    ));
}

