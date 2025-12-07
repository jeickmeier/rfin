//! Structured credit pricing benchmarks.
//!
//! Measures performance of structured credit operations across deal types:
//! - NPV calculation (ABS, CLO, CMBS, RMBS)
//! - Cashflow generation with waterfall
//! - Risk metrics (WAL, duration, spread duration)
//! - Pool metrics (WAC, WAS, WARF, diversity)
//! - Deal-specific metrics
//! - Scaling with pool size
//!
//! Market Standards Review (Week 5)

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::types::ratings::CreditRating;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::structured_credit::{
    DealType, PaymentCalculation, Pool, PoolAsset, Recipient, RecipientType, Seniority,
    StructuredCredit, Tranche, TrancheCoupon, TrancheStructure, Waterfall,
};
use finstack_valuations::metrics::MetricId;
use time::Month;

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

fn maturity_date() -> Date {
    Date::from_calendar_date(2030, Month::December, 31).unwrap()
}

fn closing_date() -> Date {
    Date::from_calendar_date(2024, Month::January, 1).unwrap()
}

// Helper to create a pool with a specified number of assets
fn create_pool(deal_type: DealType, num_assets: usize) -> Pool {
    let mut pool = Pool::new("POOL", deal_type, Currency::USD);
    let asset_balance = 10_000_000.0 / (num_assets as f64);

    for i in 0..num_assets {
        let asset = PoolAsset::fixed_rate_bond(
            format!("ASSET-{:03}", i + 1),
            Money::new(asset_balance, Currency::USD),
            0.05 + (i as f64 * 0.001), // Vary rates slightly
            Date::from_calendar_date(2028 + (i % 3) as i32, Month::January, 1).unwrap(),
            DayCount::Act360,
        )
        .with_rating(CreditRating::BBB);

        pool.assets.push(asset);
    }
    pool
}

// Create tranches with realistic structure
fn create_tranches(total_balance: f64) -> TrancheStructure {
    let senior = Tranche::new(
        "SENIOR",
        0.0,
        70.0, // 70% senior
        Seniority::Senior,
        Money::new(total_balance * 0.70, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.035 },
        maturity_date(),
    )
    .unwrap();

    let mezz = Tranche::new(
        "MEZZ",
        70.0,
        90.0, // 20% mezzanine
        Seniority::Mezzanine,
        Money::new(total_balance * 0.20, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.06 },
        maturity_date(),
    )
    .unwrap();

    let equity = Tranche::new(
        "EQUITY",
        90.0,
        100.0, // 10% equity
        Seniority::Equity,
        Money::new(total_balance * 0.10, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.12 },
        maturity_date(),
    )
    .unwrap();

    TrancheStructure::new(vec![senior, mezz, equity]).unwrap()
}

fn create_waterfall(tranches: &TrancheStructure) -> Waterfall {
    let fees = vec![
        Recipient::new(
            "trustee",
            RecipientType::ServiceProvider("Trustee".to_string()),
            PaymentCalculation::FixedAmount {
                amount: Money::new(10_000.0, Currency::USD),
                rounding: None,
            },
        ),
        Recipient::new(
            "manager",
            RecipientType::ServiceProvider("Manager".to_string()),
            PaymentCalculation::PercentageOfCollateral {
                rate: 0.001, // 0.1% fee
                annualized: true,
                day_count: None,
                rounding: None,
            },
        ),
    ];

    Waterfall::standard_sequential(Currency::USD, tranches, fees)
}

fn create_market() -> MarketContext {
    let base = test_date();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([
            (0.0, 1.0),
            (1.0, 0.96),
            (3.0, 0.88),
            (5.0, 0.78),
            (10.0, 0.60),
        ])
        .build()
        .unwrap();

    MarketContext::new().insert_discount(disc)
}

fn create_deal(deal_type: DealType, num_assets: usize) -> StructuredCredit {
    let pool = create_pool(deal_type, num_assets);
    let tranches = create_tranches(10_000_000.0);
    let waterfall = create_waterfall(&tranches);

    StructuredCredit::apply_deal_defaults(
        format!("{:?}-BENCHMARK", deal_type),
        deal_type,
        pool,
        tranches,
        waterfall,
        closing_date(),
        maturity_date(),
        "USD-OIS",
    )
}

// ============================================================================
// NPV Benchmarks by Deal Type
// ============================================================================

fn bench_npv_by_deal_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_npv");
    let market = create_market();
    let as_of = test_date();

    for deal_type in [DealType::ABS, DealType::CLO, DealType::CMBS, DealType::RMBS].iter() {
        let deal = create_deal(*deal_type, 10);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", deal_type)),
            deal_type,
            |b, _| {
                b.iter(|| deal.value(black_box(&market), black_box(as_of)));
            },
        );
    }
    group.finish();
}

// ============================================================================
// Cashflow Generation Benchmarks
// ============================================================================

fn bench_cashflow_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_cashflows");
    let market = create_market();
    let as_of = test_date();

    for num_assets in [10, 25, 50, 100].iter() {
        let deal = create_deal(DealType::CLO, *num_assets);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}assets", num_assets)),
            num_assets,
            |b, _| {
                b.iter(|| deal.build_schedule(black_box(&market), black_box(as_of)));
            },
        );
    }
    group.finish();
}

// ============================================================================
// Risk Metrics Benchmarks
// ============================================================================

fn bench_wal_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_wal");
    let market = create_market();
    let as_of = test_date();

    for deal_type in [DealType::CLO, DealType::RMBS].iter() {
        let deal = create_deal(*deal_type, 20);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", deal_type)),
            deal_type,
            |b, _| {
                b.iter(|| {
                    deal.price_with_metrics(
                        black_box(&market),
                        black_box(as_of),
                        black_box(&[MetricId::WAL]),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_duration_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_duration");
    let market = create_market();
    let as_of = test_date();
    let deal = create_deal(DealType::CLO, 25);

    let metrics = vec![MetricId::DurationMod, MetricId::SpreadDuration];

    group.bench_function("duration_suite", |b| {
        b.iter(|| {
            deal.price_with_metrics(black_box(&market), black_box(as_of), black_box(&metrics))
        });
    });
    group.finish();
}

fn bench_cs01(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_cs01");
    let market = create_market();
    let as_of = test_date();

    for num_assets in [10, 25, 50].iter() {
        let deal = create_deal(DealType::CLO, *num_assets);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}assets", num_assets)),
            num_assets,
            |b, _| {
                b.iter(|| {
                    deal.price_with_metrics(
                        black_box(&market),
                        black_box(as_of),
                        black_box(&[MetricId::Cs01]),
                    )
                });
            },
        );
    }
    group.finish();
}

// ============================================================================
// Pool Metrics Benchmarks
// ============================================================================

fn bench_pool_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_pool_metrics");
    let market = create_market();
    let as_of = test_date();

    let pool_metrics = vec![
        MetricId::CloWac,
        MetricId::CloWas,
        MetricId::WAM,
        MetricId::CloDiversity,
    ];

    for num_assets in [10, 50, 100, 200].iter() {
        let deal = create_deal(DealType::CLO, *num_assets);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}assets", num_assets)),
            num_assets,
            |b, _| {
                b.iter(|| {
                    deal.price_with_metrics(
                        black_box(&market),
                        black_box(as_of),
                        black_box(&pool_metrics),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_warf_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_warf");
    let market = create_market();
    let as_of = test_date();

    for num_assets in [25, 50, 100, 200].iter() {
        let deal = create_deal(DealType::CLO, *num_assets);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}assets", num_assets)),
            num_assets,
            |b, _| {
                b.iter(|| {
                    deal.price_with_metrics(
                        black_box(&market),
                        black_box(as_of),
                        black_box(&[MetricId::CloWarf]),
                    )
                });
            },
        );
    }
    group.finish();
}

// ============================================================================
// Price Metrics Benchmarks
// ============================================================================

fn bench_price_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_prices");
    let market = create_market();
    let as_of = test_date();
    let deal = create_deal(DealType::ABS, 20);

    let price_metrics = vec![
        MetricId::DirtyPrice,
        MetricId::CleanPrice,
        MetricId::Accrued,
    ];

    group.bench_function("price_suite", |b| {
        b.iter(|| {
            deal.price_with_metrics(
                black_box(&market),
                black_box(as_of),
                black_box(&price_metrics),
            )
        });
    });
    group.finish();
}

// ============================================================================
// Full Metrics Suite Benchmark
// ============================================================================

fn bench_full_metrics_suite(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_full_metrics");
    let market = create_market();
    let as_of = test_date();
    let deal = create_deal(DealType::CLO, 50);

    let all_metrics = vec![
        MetricId::DirtyPrice,
        MetricId::CleanPrice,
        MetricId::Accrued,
        MetricId::WAL,
        MetricId::DurationMod,
        MetricId::SpreadDuration,
        MetricId::Cs01,
        MetricId::CloWac,
        MetricId::CloWas,
        MetricId::WAM,
        MetricId::CloWarf,
        MetricId::CloDiversity,
    ];

    group.bench_function("all_metrics", |b| {
        b.iter(|| {
            deal.price_with_metrics(
                black_box(&market),
                black_box(as_of),
                black_box(&all_metrics),
            )
        });
    });
    group.finish();
}

// ============================================================================
// Scaling Benchmarks
// ============================================================================

fn bench_scaling_with_pool_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("structured_credit_scaling");
    let market = create_market();
    let as_of = test_date();

    for num_assets in [10, 25, 50, 100, 200, 500].iter() {
        let deal = create_deal(DealType::CLO, *num_assets);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}assets", num_assets)),
            num_assets,
            |b, _| {
                b.iter(|| deal.value(black_box(&market), black_box(as_of)));
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_npv_by_deal_type,
    bench_cashflow_generation,
    bench_wal_calculation,
    bench_duration_metrics,
    bench_cs01,
    bench_pool_metrics,
    bench_warf_calculation,
    bench_price_metrics,
    bench_full_metrics_suite,
    bench_scaling_with_pool_size
);
criterion_main!(benches);
