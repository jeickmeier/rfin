//! Equity option pricing benchmarks.
//!
//! Measures performance of Black-Scholes pricing and Greeks:
//! - Present value (call and put)
//! - Full Greeks calculation (delta, gamma, vega, theta, rho)
//! - Individual Greeks
//!
//! Market Standards Review (Week 5)

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::instruments::equity::equity_option::EquityOption;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
#[allow(dead_code, unused_imports, clippy::expect_used, clippy::unwrap_used)]
mod test_utils {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/test_utils.rs"
    ));
}
use std::hint::black_box;
use time::Month;

fn create_call_option(expiry_months: i64) -> EquityOption {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let expiry = base + time::Duration::days(expiry_months * 30);

    test_utils::equity_option_european_call(
        format!("CALL-{}M", expiry_months),
        "AAPL",
        100.0, // ATM strike
        expiry,
        100.0, // 100 shares
    )
    .expect("equity option should build in benchmarks")
}

fn create_market() -> MarketContext {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([
            (0.0, 1.0),
            (0.5, 0.98),
            (1.0, 0.96),
            (2.0, 0.92),
            (5.0, 0.80),
        ])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let vol_surface = VolSurface::from_grid(
        "EQUITY-VOL",
        &[0.25, 0.5, 1.0, 2.0],
        &[80.0, 90.0, 100.0, 110.0, 120.0],
        &[0.25; 20], // Flat 25% vol
    )
    .unwrap();

    MarketContext::new()
        .insert_discount(disc)
        .insert_surface(vol_surface)
        .insert_price(
            "EQUITY-SPOT",
            MarketScalar::Price(Money::new(100.0, Currency::USD)),
        )
        .insert_price("EQUITY-DIVYIELD", MarketScalar::Unitless(0.02))
}

fn bench_option_pv(c: &mut Criterion) {
    let mut group = c.benchmark_group("option_pv");
    let market = create_market();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    for months in [3, 6, 12, 24].iter() {
        let option = create_call_option(*months);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}M", months)),
            months,
            |b, _| {
                b.iter(|| option.value(black_box(&market), black_box(as_of)));
            },
        );
    }
    group.finish();
}

fn bench_option_greeks(c: &mut Criterion) {
    let mut group = c.benchmark_group("option_greeks");
    let market = create_market();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let greeks = vec![
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Theta,
        MetricId::Rho,
    ];

    for months in [3, 6, 12].iter() {
        let option = create_call_option(*months);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}M", months)),
            months,
            |b, _| {
                b.iter(|| {
                    option.price_with_metrics(
                        black_box(&market),
                        black_box(as_of),
                        black_box(&greeks),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_option_pv, bench_option_greeks);
criterion_main!(benches);
