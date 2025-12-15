use criterion::{criterion_group, criterion_main, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_valuations::calibration::methods::sabr_surface::VolSurfaceCalibrator;
use finstack_valuations::calibration::Calibrator;
use finstack_valuations::calibration::VolQuote;
use std::hint::black_box;
use time::Month;

fn bench_sabr_slice(c: &mut Criterion) {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let quotes = vec![
        VolQuote::OptionVol {
            underlying: "SPY".to_string().into(),
            expiry: base_date + time::Duration::days(30),
            strike: 95.0,
            vol: 0.22,
            option_type: "Call".to_string(),
            conventions: Default::default(),
        },
        VolQuote::OptionVol {
            underlying: "SPY".to_string().into(),
            expiry: base_date + time::Duration::days(30),
            strike: 100.0,
            vol: 0.20,
            option_type: "Call".to_string(),
            conventions: Default::default(),
        },
        VolQuote::OptionVol {
            underlying: "SPY".to_string().into(),
            expiry: base_date + time::Duration::days(30),
            strike: 105.0,
            vol: 0.21,
            option_type: "Call".to_string(),
            conventions: Default::default(),
        },
    ];
    let calibrator =
        VolSurfaceCalibrator::new("SPY-VOL", 1.0, vec![1.0 / 12.0], vec![95.0, 100.0, 105.0])
            .with_base_date(base_date)
            .with_base_currency(Currency::USD);
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.78)])
        .build()
        .unwrap();
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_price(
            "SPY",
            finstack_core::market_data::scalars::MarketScalar::Unitless(100.0),
        )
        .insert_price(
            "SPY-DIVYIELD",
            finstack_core::market_data::scalars::MarketScalar::Unitless(0.02),
        );
    c.bench_function("sabr_slice_calibration", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&market))
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_sabr_slice);
criterion_main!(benches);
