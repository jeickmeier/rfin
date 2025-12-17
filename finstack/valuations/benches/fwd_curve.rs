use criterion::{criterion_group, criterion_main, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_valuations::calibration::adapters::handlers::execute_step;
use finstack_valuations::calibration::api::schema::{CalibrationMethod, ForwardCurveParams, StepParams};
use finstack_valuations::calibration::quotes::{InstrumentConventions, MarketQuote, RatesQuote};
use finstack_valuations::calibration::CalibrationConfig;
use std::hint::black_box;
use time::Month;

fn bench_forward_curve(c: &mut Criterion) {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
        .build()
        .unwrap();
    let ctx = MarketContext::new().insert_discount(disc);
    let quotes = [
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
    let settings = CalibrationConfig::default();
    let params = ForwardCurveParams {
        curve_id: "USD-SOFR-3M-FWD".into(),
        currency: Currency::USD,
        base_date,
        tenor_years: 0.25,
        discount_curve_id: "USD-OIS".into(),
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        conventions: Default::default(),
    };
    let step = StepParams::Forward(params);
    let market_quotes: Vec<MarketQuote> = quotes.iter().cloned().map(MarketQuote::Rates).collect();
    c.bench_function("fwd_curve_fra_strip", |b| {
        b.iter(|| {
            execute_step(black_box(&step), black_box(&market_quotes), black_box(&ctx), black_box(&settings))
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_forward_curve);
criterion_main!(benches);
