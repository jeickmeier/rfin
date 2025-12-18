//! Strict pricing tests for calibration v2.
//!
//! Strict pricing is an opt-in, vendor-matching mode: it rejects hidden convention
//! fallbacks and requires explicit step-level defaults and/or quote conventions.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::ScalarTimeSeries;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::math::interp::ExtrapolationPolicy;
use finstack_core::math::interp::InterpStyle;
use finstack_core::prelude::DateExt;
use finstack_core::types::Currency;
use finstack_core::types::CurveId;
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelopeV2, CalibrationMethod, CalibrationPlanV2, CalibrationStepV2,
    DiscountCurveParams, RatesStepConventions, StepParams,
};
use finstack_valuations::calibration::pricing::{
    CalibrationPricer, ConvexityParameters, VolatilitySource,
};
use finstack_valuations::calibration::quotes::FutureSpecs;
use finstack_valuations::calibration::quotes::{InstrumentConventions, MarketQuote, RatesQuote};
use std::collections::HashMap;
use time::Month;

fn base_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 2).expect("base_date")
}

fn usd_ois_quotes(base: Date, fixed_leg: InstrumentConventions) -> Vec<MarketQuote> {
    vec![
        MarketQuote::Rates(RatesQuote::Deposit {
            maturity: base + time::Duration::days(1),
            rate: 0.05,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360)
                .with_settlement_days(0),
        }),
        MarketQuote::Rates(RatesQuote::Swap {
            maturity: base.add_months(12),
            rate: 0.051,
            is_ois: true,
            conventions: InstrumentConventions::default(),
            fixed_leg_conventions: fixed_leg,
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::annual())
                .with_day_count(DayCount::Act360)
                .with_index("USD-SOFR"),
        }),
    ]
}

#[test]
fn strict_pricing_requires_step_level_defaults() {
    let base = base_date();
    let currency = Currency::USD;

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "ois".to_string(),
        usd_ois_quotes(
            base,
            InstrumentConventions::default()
                .with_payment_frequency(Tenor::annual())
                .with_day_count(DayCount::Act360),
        ),
    );

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "disc".to_string(),
            quote_set: "ois".to_string(),
            params: StepParams::Discount(DiscountCurveParams {
                curve_id: "USD-OIS".into(),
                currency,
                base_date: base,
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
                extrapolation: ExtrapolationPolicy::FlatForward,
                pricing_discount_id: None,
                pricing_forward_id: None,
                conventions: RatesStepConventions {
                    strict_pricing: Some(true),
                    ..Default::default()
                },
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&MarketContext::new()).into()),
    };

    let err =
        engine::execute(&envelope).expect_err("strict pricing should reject missing defaults");
    assert!(matches!(err, finstack_core::Error::Validation(_)));
}

#[test]
fn strict_pricing_succeeds_with_explicit_step_defaults() {
    let base = base_date();
    let currency = Currency::USD;

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    quote_sets.insert(
        "ois".to_string(),
        usd_ois_quotes(
            base,
            InstrumentConventions::default()
                .with_payment_frequency(Tenor::annual())
                .with_day_count(DayCount::Act360),
        ),
    );

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "disc".to_string(),
            quote_set: "ois".to_string(),
            params: StepParams::Discount(DiscountCurveParams {
                curve_id: "USD-OIS".into(),
                currency,
                base_date: base,
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
                extrapolation: ExtrapolationPolicy::FlatForward,
                pricing_discount_id: None,
                pricing_forward_id: None,
                conventions: RatesStepConventions {
                    strict_pricing: Some(true),
                    settlement_days: Some(0),
                    calendar_id: Some("usny".to_string()),
                    business_day_convention: Some(BusinessDayConvention::ModifiedFollowing),
                    default_payment_delay_days: Some(0),
                    default_reset_lag_days: Some(-2),
                    ..Default::default()
                },
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        // Provide minimal fixings for USD-OIS in case the first compounded coupon
        // references observation dates before as_of (e.g., T+0 start with SOFR lookback).
        initial_market: {
            let base = base_date();
            let fixings: Vec<(Date, f64)> =
                (1..=10).map(|i| (base.add_weekdays(-i), 0.05)).collect();
            let m = MarketContext::new()
                .insert_series(ScalarTimeSeries::new("FIXING:USD-OIS", fixings, None).unwrap());
            Some((&m).into())
        },
    };

    let result = engine::execute(&envelope).expect("execute");
    assert!(result.result.report.success);
}

#[test]
fn strict_pricing_rejects_missing_fixed_leg_conventions() {
    let base = base_date();
    let currency = Currency::USD;

    let mut quote_sets: HashMap<String, Vec<MarketQuote>> = HashMap::new();
    // Fixed leg conventions intentionally missing freq/daycount.
    quote_sets.insert(
        "ois".to_string(),
        usd_ois_quotes(base, InstrumentConventions::default()),
    );

    let plan = CalibrationPlanV2 {
        id: "plan".to_string(),
        description: None,
        quote_sets,
        settings: Default::default(),
        steps: vec![CalibrationStepV2 {
            id: "disc".to_string(),
            quote_set: "ois".to_string(),
            params: StepParams::Discount(DiscountCurveParams {
                curve_id: "USD-OIS".into(),
                currency,
                base_date: base,
                method: CalibrationMethod::Bootstrap,
                interpolation: Default::default(),
                extrapolation: ExtrapolationPolicy::FlatForward,
                pricing_discount_id: None,
                pricing_forward_id: None,
                conventions: RatesStepConventions {
                    strict_pricing: Some(true),
                    settlement_days: Some(0),
                    calendar_id: Some("usny".to_string()),
                    business_day_convention: Some(BusinessDayConvention::ModifiedFollowing),
                    default_payment_delay_days: Some(0),
                    default_reset_lag_days: Some(-2),
                    ..Default::default()
                },
            }),
        }],
    };

    let envelope = CalibrationEnvelopeV2 {
        schema: "finstack.calibration/2".to_string(),
        plan,
        initial_market: Some((&MarketContext::new()).into()),
    };

    let err = engine::execute(&envelope)
        .expect_err("strict pricing should reject missing fixed leg conventions");
    assert!(matches!(
        err,
        finstack_core::Error::Validation(_) | finstack_core::Error::Calibration { .. }
    ));
}

#[test]
fn strict_pricing_rejects_future_without_convexity_inputs() {
    let base = base_date();
    let currency = Currency::USD;

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .day_count(DayCount::Act365F)
        .knots([(0.0, 1.0), (10.0, (-0.03_f64 * 10.0).exp())])
        .set_interp(InterpStyle::Linear)
        .extrapolation(ExtrapolationPolicy::FlatForward)
        .build()
        .expect("discount curve");

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.03), (10.0, 0.03)])
        .build()
        .expect("forward curve");

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd);

    let quote = RatesQuote::Future {
        expiry: base + time::Duration::days(60),
        period_start: base + time::Duration::days(90),
        period_end: base + time::Duration::days(180),
        fixing_date: None,
        price: 95.0,
        specs: Some(FutureSpecs::default()),
        conventions: Default::default(),
    };

    let pricer = CalibrationPricer::for_forward_curve(
        base,
        CurveId::from("USD-SOFR-3M"),
        CurveId::from("USD-OIS"),
        0.25,
    )
    .with_strict_pricing(true);

    let err = pricer
        .price_instrument_for_calibration(&quote, currency, &ctx)
        .expect_err("strict pricing should require explicit futures convexity inputs");
    assert!(matches!(err, finstack_core::Error::Validation(_)));
}

#[test]
fn strict_pricing_allows_future_with_explicit_convexity() {
    let base = base_date();
    let currency = Currency::USD;

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .day_count(DayCount::Act365F)
        .knots([(0.0, 1.0), (10.0, (-0.03_f64 * 10.0).exp())])
        .set_interp(InterpStyle::Linear)
        .extrapolation(ExtrapolationPolicy::FlatForward)
        .build()
        .expect("discount curve");

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.03), (10.0, 0.03)])
        .build()
        .expect("forward curve");

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd);

    let quote = RatesQuote::Future {
        expiry: base + time::Duration::days(60),
        period_start: base + time::Duration::days(90),
        period_end: base + time::Duration::days(180),
        fixing_date: None,
        price: 95.0,
        specs: Some(FutureSpecs {
            convexity_adjustment: Some(0.0001),
            ..FutureSpecs::default()
        }),
        conventions: Default::default(),
    };

    let pricer = CalibrationPricer::for_forward_curve(
        base,
        CurveId::from("USD-SOFR-3M"),
        CurveId::from("USD-OIS"),
        0.25,
    )
    .with_strict_pricing(true);

    let pv = pricer
        .price_instrument_for_calibration(&quote, currency, &ctx)
        .expect("pricing should succeed with quote-level convexity adjustment");
    assert!(pv.is_finite());

    let pricer_with_params = CalibrationPricer::for_forward_curve(
        base,
        CurveId::from("USD-SOFR-3M"),
        CurveId::from("USD-OIS"),
        0.25,
    )
    .with_strict_pricing(true)
    .with_convexity_params(ConvexityParameters {
        base_volatility: 0.02,
        mean_reversion: 0.01,
        use_ho_lee: false,
        vol_source: VolatilitySource::Explicit(0.02),
        explicit_adjustment: None,
    });

    let quote_no_adj = RatesQuote::Future {
        expiry: base + time::Duration::days(60),
        period_start: base + time::Duration::days(90),
        period_end: base + time::Duration::days(180),
        fixing_date: None,
        price: 95.0,
        specs: Some(FutureSpecs::default()),
        conventions: Default::default(),
    };

    let pv2 = pricer_with_params
        .price_instrument_for_calibration(&quote_no_adj, currency, &ctx)
        .expect("pricing should succeed with step-level convexity params");
    assert!(pv2.is_finite());
}
