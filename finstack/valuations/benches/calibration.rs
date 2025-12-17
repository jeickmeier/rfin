//! Calibration benchmarks (v2 plan-driven API).
//!
//! This benchmark suite focuses on the plan-driven calibration step engine.

use criterion::{criterion_group, criterion_main, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::adapters::handlers::execute_step;
use finstack_valuations::calibration::api::schema::{
    CalibrationMethod, DiscountCurveParams, ForwardCurveParams, StepParams,
};
use finstack_valuations::calibration::quotes::{InstrumentConventions, MarketQuote, RatesQuote};
use finstack_valuations::calibration::CalibrationConfig;
use std::hint::black_box;
use time::Month;

fn bench_discount_and_forward_steps(c: &mut Criterion) {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let settings = CalibrationConfig::default();

    // Discount curve inputs
    let disc_quotes: Vec<RatesQuote> = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.046,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.047,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
    ];
    let disc_mq: Vec<MarketQuote> = disc_quotes
        .iter()
        .cloned()
        .map(MarketQuote::Rates)
        .collect();
    let disc_step = StepParams::Discount(DiscountCurveParams {
        curve_id: "USD-OIS".into(),
        currency: Currency::USD,
        base_date,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        extrapolation: finstack_core::math::interp::ExtrapolationPolicy::FlatForward,
        pricing_discount_id: None,
        pricing_forward_id: None,
        conventions: Default::default(),
    });

    // Forward curve inputs
    let fwd_quotes: Vec<RatesQuote> = vec![
        RatesQuote::FRA {
            start: base_date + time::Duration::days(90),
            end: base_date + time::Duration::days(180),
            rate: 0.047,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::FRA {
            start: base_date + time::Duration::days(180),
            end: base_date + time::Duration::days(270),
            rate: 0.048,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
    ];
    let fwd_mq: Vec<MarketQuote> = fwd_quotes.iter().cloned().map(MarketQuote::Rates).collect();
    let fwd_step = StepParams::Forward(ForwardCurveParams {
        curve_id: "USD-SOFR-3M-FWD".into(),
        currency: Currency::USD,
        base_date,
        tenor_years: 0.25,
        discount_curve_id: "USD-OIS".into(),
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        conventions: Default::default(),
    });

    c.bench_function("calibration_v2_discount_step", |b| {
        let base = MarketContext::new();
        b.iter(|| {
            execute_step(
                black_box(&disc_step),
                black_box(&disc_mq),
                black_box(&base),
                black_box(&settings),
            )
            .unwrap()
        })
    });

    c.bench_function("calibration_v2_discount_then_forward_steps", |b| {
        let base = MarketContext::new();
        b.iter(|| {
            let (ctx_after_disc, _) = execute_step(
                black_box(&disc_step),
                black_box(&disc_mq),
                black_box(&base),
                black_box(&settings),
            )
            .unwrap();
            execute_step(
                black_box(&fwd_step),
                black_box(&fwd_mq),
                black_box(&ctx_after_disc),
                black_box(&settings),
            )
            .unwrap()
        })
    });
}

criterion_group!(benches, bench_discount_and_forward_steps);
criterion_main!(benches);
