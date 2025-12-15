use criterion::{criterion_group, criterion_main, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_valuations::calibration::methods::forward_curve::ForwardCurveCalibrator;
use finstack_valuations::calibration::quotes::InstrumentConventions;
use finstack_valuations::calibration::{Calibrator, RatesQuote};
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
    let quotes = vec![
        RatesQuote::FRA {
            start: base_date + time::Duration::days(90),
            end: base_date + time::Duration::days(180),
            rate: 0.047,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360),
        },
        RatesQuote::FRA {
            start: base_date + time::Duration::days(180),
            end: base_date + time::Duration::days(270),
            rate: 0.048,
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360),
        },
    ];
    let calibrator =
        ForwardCurveCalibrator::new("USD-SOFR-3M-FWD", 0.25, base_date, Currency::USD, "USD-OIS");
    c.bench_function("fwd_curve_fra_strip", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&ctx))
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_forward_curve);
criterion_main!(benches);
