//! Portfolio cashflow aggregation benchmarks.
//!
//! Measures the full cashflow ladder pipeline for realistic institutional portfolios.
//! Cashflow aggregation touches per-position schedule generation, O(E log E) event
//! sort, nested IndexMap accumulation, and (optionally) FX conversion of every
//! distinct payment date.
//!
//! Benchmark groups:
//! - `portfolio_cashflows_simple`     — `aggregate_cashflows` (date × currency ladder)
//! - `portfolio_cashflows_full`       — `aggregate_full_cashflows` (+ CFKind breakdown)
//! - `portfolio_cashflows_by_period`  — pre-built ladder → `cashflows_to_base_by_period`

#[path = "bench_common.rs"]
mod bench_common;

use bench_common::{create_institutional_portfolio, create_market_context};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Period, PeriodId};
use finstack_portfolio::cashflows::{
    aggregate_cashflows, aggregate_full_cashflows, cashflows_to_base_by_period,
};
use std::hint::black_box;
use time::Month;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a set of annual reporting periods spanning the benchmark portfolio's
/// cashflow horizon (2025–2035).
fn annual_periods() -> Vec<Period> {
    (0..10i32)
        .map(|i| {
            let year = 2025 + i;
            Period {
                id: PeriodId::annual(year),
                start: Date::from_calendar_date(year, Month::January, 1).unwrap(),
                end: Date::from_calendar_date(year + 1, Month::January, 1).unwrap(),
                is_actual: i == 0,
            }
        })
        .collect()
}

// ============================================================================
// aggregate_cashflows — date × currency ladder
// ============================================================================

fn bench_aggregate_cashflows(c: &mut Criterion) {
    let mut group = c.benchmark_group("portfolio_cashflows_simple");
    let market = create_market_context();

    for num_positions in [50usize, 100, 250, 500, 2000] {
        let portfolio = create_institutional_portfolio(num_positions);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}pos", num_positions)),
            &num_positions,
            |b, _| {
                b.iter(|| aggregate_cashflows(black_box(&portfolio), black_box(&market)).unwrap());
            },
        );
    }
    group.finish();
}

// ============================================================================
// aggregate_full_cashflows — date × currency × CFKind ladder
// ============================================================================

fn bench_aggregate_full_cashflows(c: &mut Criterion) {
    let mut group = c.benchmark_group("portfolio_cashflows_full");
    let market = create_market_context();

    for num_positions in [50usize, 100, 250] {
        let portfolio = create_institutional_portfolio(num_positions);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}pos", num_positions)),
            &num_positions,
            |b, _| {
                b.iter(|| {
                    aggregate_full_cashflows(black_box(&portfolio), black_box(&market)).unwrap()
                });
            },
        );
    }
    group.finish();
}

// ============================================================================
// cashflows_to_base_by_period
//
// The ladder is pre-built outside the loop so this group isolates the cost of
// FX conversion across all payment dates and bucketing into reporting periods.
// ============================================================================

fn bench_cashflows_to_base_by_period(c: &mut Criterion) {
    let mut group = c.benchmark_group("portfolio_cashflows_by_period");
    let market = create_market_context();
    let periods = annual_periods();

    for num_positions in [50usize, 100, 250] {
        let portfolio = create_institutional_portfolio(num_positions);
        let ladder = aggregate_cashflows(&portfolio, &market).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}pos", num_positions)),
            &num_positions,
            |b, _| {
                b.iter(|| {
                    cashflows_to_base_by_period(
                        black_box(&ladder),
                        black_box(&market),
                        black_box(Currency::USD),
                        black_box(&periods),
                    )
                    .unwrap()
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_aggregate_cashflows,
    bench_aggregate_full_cashflows,
    bench_cashflows_to_base_by_period,
);
criterion_main!(benches);
