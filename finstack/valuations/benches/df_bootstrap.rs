use criterion::{black_box, criterion_group, criterion_main, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::methods::discount::DiscountCurveCalibrator;
use finstack_valuations::calibration::{Calibrator, RatesQuote};
use time::Month;

fn bench_df_bootstrap(c: &mut Criterion) {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            day_count: DayCount::Act360,
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.046,
            day_count: DayCount::Act360,
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.047,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-OIS".to_string().into(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 2),
            rate: 0.048,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-OIS".to_string().into(),
        },
    ];
    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);
    let ctx = MarketContext::new();
    c.bench_function("df_bootstrap_small", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&ctx))
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_df_bootstrap);
criterion_main!(benches);
