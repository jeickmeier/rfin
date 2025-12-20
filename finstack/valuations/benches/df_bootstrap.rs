use criterion::{criterion_group, criterion_main, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::api::schema::{
    CalibrationMethod, DiscountCurveParams, StepParams,
};
use finstack_valuations::calibration::targets::handlers::execute_step;
use finstack_valuations::calibration::CalibrationConfig;
use finstack_valuations::market::conventions::ids::IndexId;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote;
use std::hint::black_box;
use time::Month;

fn bench_df_bootstrap(c: &mut Criterion) {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let quotes = [
        RateQuote::Deposit {
            id: QuoteId::new("DEP-30D"),
            index: IndexId::new("USD-SOFR"),
            pillar: Pillar::Date(base_date + time::Duration::days(30)),
            rate: 0.045,
        },
        RateQuote::Deposit {
            id: QuoteId::new("DEP-90D"),
            index: IndexId::new("USD-SOFR"),
            pillar: Pillar::Date(base_date + time::Duration::days(90)),
            rate: 0.046,
        },
        RateQuote::Swap {
            id: QuoteId::new("SWP-1Y"),
            index: IndexId::new("USD-SOFR-OIS"),
            pillar: Pillar::Date(base_date + time::Duration::days(365)),
            rate: 0.047,
            spread: None,
        },
        RateQuote::Swap {
            id: QuoteId::new("SWP-2Y"),
            index: IndexId::new("USD-SOFR-OIS"),
            pillar: Pillar::Date(base_date + time::Duration::days(365 * 2)),
            rate: 0.048,
            spread: None,
        },
    ];
    let ctx = MarketContext::new();
    let settings = CalibrationConfig::default();
    let params = DiscountCurveParams {
        curve_id: "USD-OIS".into(),
        currency: Currency::USD,
        base_date,
        method: CalibrationMethod::Bootstrap,
        interpolation: Default::default(),
        extrapolation: finstack_core::math::interp::ExtrapolationPolicy::FlatForward,
        pricing_discount_id: None,
        pricing_forward_id: None,
        conventions: Default::default(),
    };
    let step = StepParams::Discount(params);
    let market_quotes: Vec<MarketQuote> = quotes.iter().cloned().map(MarketQuote::Rates).collect();

    c.bench_function("df_bootstrap_small", |b| {
        b.iter(|| {
            execute_step(
                black_box(&step),
                black_box(&market_quotes),
                black_box(&ctx),
                black_box(&settings),
            )
            .unwrap()
        })
    });
}

criterion_group!(benches, bench_df_bootstrap);
criterion_main!(benches);
