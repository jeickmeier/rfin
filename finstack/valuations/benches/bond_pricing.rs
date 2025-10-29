//! Bond pricing benchmarks.
//!
//! Measures performance of critical bond pricing operations:
//! - YTM solver convergence
//! - Duration and convexity calculations
//! - DV01 calculation
//! - Clean/dirty price computation
//!
//! Market Standards Review (Week 5)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::MetricId;
use time::Month;

fn create_test_bond(maturity_years: i32) -> Bond {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2025 + maturity_years, Month::January, 1).unwrap();

    Bond::fixed(
        format!("BOND-{}Y", maturity_years),
        Money::new(1_000_000.0, Currency::USD),
        0.05, // 5% coupon
        issue,
        maturity,
        "USD-OIS",
    )
}

fn create_market() -> MarketContext {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([
            (0.0, 1.0),
            (1.0, 0.98),
            (2.0, 0.96),
            (5.0, 0.88),
            (10.0, 0.70),
            (30.0, 0.40),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    MarketContext::new().insert_discount(curve)
}

fn bench_bond_pv(c: &mut Criterion) {
    let mut group = c.benchmark_group("bond_pv");
    let market = create_market();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    for tenor in [2, 5, 10, 30].iter() {
        let bond = create_test_bond(*tenor);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}Y", tenor)),
            tenor,
            |b, _| {
                b.iter(|| bond.value(black_box(&market), black_box(as_of)));
            },
        );
    }
    group.finish();
}

fn bench_bond_ytm(c: &mut Criterion) {
    let mut group = c.benchmark_group("bond_ytm_solve");
    let market = create_market();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    for tenor in [2, 5, 10, 30].iter() {
        let mut bond = create_test_bond(*tenor);
        // Set quoted price to require YTM solving
        bond.pricing_overrides = PricingOverrides::default().with_clean_price(95.0);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}Y", tenor)),
            tenor,
            |b, _| {
                b.iter(|| {
                    bond.price_with_metrics(
                        black_box(&market),
                        black_box(as_of),
                        black_box(&[MetricId::Ytm]),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_bond_duration(c: &mut Criterion) {
    let mut group = c.benchmark_group("bond_duration");
    let market = create_market();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    for tenor in [2, 5, 10, 30].iter() {
        let bond = create_test_bond(*tenor);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}Y", tenor)),
            tenor,
            |b, _| {
                b.iter(|| {
                    bond.price_with_metrics(
                        black_box(&market),
                        black_box(as_of),
                        black_box(&[MetricId::DurationMod, MetricId::Convexity]),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_bond_dv01(c: &mut Criterion) {
    let mut group = c.benchmark_group("bond_dv01");
    let market = create_market();
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    for tenor in [2, 5, 10, 30].iter() {
        let bond = create_test_bond(*tenor);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}Y", tenor)),
            tenor,
            |b, _| {
                b.iter(|| {
                    bond.price_with_metrics(
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

criterion_group!(
    benches,
    bench_bond_pv,
    bench_bond_ytm,
    bench_bond_duration,
    bench_bond_dv01
);
criterion_main!(benches);
