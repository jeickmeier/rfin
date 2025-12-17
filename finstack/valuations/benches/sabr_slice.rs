use criterion::{criterion_group, criterion_main, Criterion};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_valuations::calibration::adapters::handlers::execute_step;
use finstack_valuations::calibration::api::schema::{StepParams, VolSurfaceParams};
use finstack_valuations::calibration::domain::quotes::{MarketQuote, VolQuote};
use finstack_valuations::calibration::CalibrationConfig;
use std::hint::black_box;
use time::Month;

fn bench_sabr_slice(c: &mut Criterion) {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let quotes = [
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
    let settings = CalibrationConfig::default();
    let params = VolSurfaceParams {
        surface_id: "SPY-VOL".to_string(),
        base_date,
        underlying_id: "SPY".to_string(),
        model: "SABR".to_string(),
        discount_curve_id: Some("USD-OIS".into()),
        beta: 1.0,
        target_expiries: vec![1.0 / 12.0],
        target_strikes: vec![95.0, 100.0, 105.0],
        spot_override: None,
        dividend_yield_override: None,
        expiry_extrapolation: Default::default(),
    };
    let step = StepParams::VolSurface(params);
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
    let market_quotes: Vec<MarketQuote> = quotes.iter().cloned().map(MarketQuote::Vol).collect();
    c.bench_function("sabr_slice_calibration", |b| {
        b.iter(|| {
            execute_step(black_box(&step), black_box(&market_quotes), black_box(&market), black_box(&settings))
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_sabr_slice);
criterion_main!(benches);
