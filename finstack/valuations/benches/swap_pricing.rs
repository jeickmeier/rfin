//! Interest rate swap pricing benchmarks.
//!
//! Measures performance of IRS operations:
//! - Present value calculation
//! - DV01 (bump-and-revalue)
//! - Annuity factor calculation
//! - Par rate calculation
//!
//! Market Standards Review (Week 5)

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
use finstack_valuations::metrics::MetricId;
use std::hint::black_box;
use time::Month;

fn create_swap(tenor_years: i32) -> InterestRateSwap {
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2025 + tenor_years, Month::January, 1).unwrap();

    InterestRateSwap::create_usd_swap(
        format!("IRS-{}Y", tenor_years).into(),
        Money::new(10_000_000.0, Currency::USD),
        0.04, // 4% fixed rate
        start,
        end,
        PayReceive::PayFixed,
    )
    .expect("Failed to create swap for benchmark")
}

fn create_market() -> MarketContext {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([
            (0.0, 1.0),
            (1.0, 0.98),
            (2.0, 0.96),
            (5.0, 0.88),
            (10.0, 0.70),
            (30.0, 0.40),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base)
        .knots([
            (0.0, 0.035),
            (1.0, 0.038),
            (2.0, 0.040),
            (5.0, 0.045),
            (10.0, 0.050),
            (30.0, 0.055),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
}

fn bench_swap_pv(c: &mut Criterion) {
    let mut group = c.benchmark_group("swap_pv");
    let market = create_market();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    for tenor in [2, 5, 10, 30].iter() {
        let swap = create_swap(*tenor);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}Y", tenor)),
            tenor,
            |b, _| {
                b.iter(|| swap.value(black_box(&market), black_box(as_of)));
            },
        );
    }
    group.finish();
}

fn bench_swap_dv01(c: &mut Criterion) {
    let mut group = c.benchmark_group("swap_dv01");
    let market = create_market();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    for tenor in [2, 5, 10, 30].iter() {
        let swap = create_swap(*tenor);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}Y", tenor)),
            tenor,
            |b, _| {
                b.iter(|| {
                    swap.price_with_metrics(
                        black_box(&market),
                        black_box(as_of),
                        black_box(&[MetricId::Dv01]),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_swap_par_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("swap_par_rate");
    let market = create_market();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    for tenor in [2, 5, 10, 30].iter() {
        let swap = create_swap(*tenor);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}Y", tenor)),
            tenor,
            |b, _| {
                b.iter(|| {
                    swap.price_with_metrics(
                        black_box(&market),
                        black_box(as_of),
                        black_box(&[MetricId::ParRate, MetricId::Annuity]),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_swap_pv, bench_swap_dv01, bench_swap_par_rate);
criterion_main!(benches);
