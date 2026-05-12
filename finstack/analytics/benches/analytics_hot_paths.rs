//! Criterion benchmarks for `finstack-analytics` hot paths.
//!
//! Drives every benchmark through [`Performance`], which is the canonical
//! public entry point. Building-block functions are `pub(crate)` and not
//! intended for direct measurement.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use finstack_analytics::Performance;
use finstack_core::dates::{Date, Month, PeriodKind};

fn synthetic_returns(n: usize, seed: u64) -> Vec<f64> {
    // Deterministic pseudo-random sequence via a splitmix64-ish iteration;
    // avoids bench-time jitter from a real RNG crate.
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    (0..n)
        .map(|_| {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            let u = ((z ^ (z >> 31)) as f64) / (u64::MAX as f64);
            (u - 0.5) * 0.04
        })
        .collect()
}

fn synthetic_dates(n: usize) -> Vec<Date> {
    let start = Date::from_calendar_date(2020, Month::January, 1).expect("valid");
    let mut dates = Vec::with_capacity(n);
    let mut d = start;
    for _ in 0..n {
        dates.push(d);
        d = d.next_day().expect("next day");
    }
    dates
}

fn perf_from_returns(n: usize, seed: u64) -> Performance {
    let returns = synthetic_returns(n, seed);
    let dates = synthetic_dates(n);
    Performance::from_returns(
        dates,
        vec![returns],
        vec!["X".to_string()],
        None,
        PeriodKind::Daily,
    )
    .expect("performance from returns")
}

fn bench_tail_risk(c: &mut Criterion) {
    let perf_small = perf_from_returns(2_500, 42);
    c.bench_function("Performance::value_at_risk 2.5k", |b| {
        b.iter(|| black_box(perf_small.value_at_risk(0.95)));
    });
    c.bench_function("Performance::expected_shortfall 2.5k", |b| {
        b.iter(|| black_box(perf_small.expected_shortfall(0.95)));
    });

    let perf_large = perf_from_returns(100_000, 43);
    c.bench_function("Performance::value_at_risk 100k", |b| {
        b.iter(|| black_box(perf_large.value_at_risk(0.95)));
    });
    c.bench_function("Performance::expected_shortfall 100k", |b| {
        b.iter(|| black_box(perf_large.expected_shortfall(0.95)));
    });
}

fn bench_return_based(c: &mut Criterion) {
    let perf = perf_from_returns(2_500, 7);
    c.bench_function("Performance::volatility 2.5k", |b| {
        b.iter(|| black_box(perf.volatility(true)));
    });
    c.bench_function("Performance::sharpe 2.5k", |b| {
        b.iter(|| black_box(perf.sharpe(0.02)));
    });
}

fn bench_drawdown(c: &mut Criterion) {
    let perf = perf_from_returns(10_000, 11);
    c.bench_function("Performance::drawdown_series 10k", |b| {
        b.iter(|| black_box(perf.drawdown_series()));
    });
}

fn bench_performance(c: &mut Criterion) {
    let n = 750;
    let dates = synthetic_dates(n);
    let prices_a: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.02).collect();
    let prices_b: Vec<f64> = (0..n).map(|i| 50.0 - i as f64 * 0.005).collect();

    c.bench_function("Performance::new 750x2 daily", |b| {
        b.iter(|| {
            black_box(
                Performance::new(
                    dates.clone(),
                    vec![prices_a.clone(), prices_b.clone()],
                    vec!["A".to_string(), "B".to_string()],
                    Some("B"),
                    PeriodKind::Daily,
                )
                .expect("perf"),
            )
        });
    });

    let perf = Performance::new(
        dates,
        vec![prices_a, prices_b],
        vec!["A".to_string(), "B".to_string()],
        Some("B"),
        PeriodKind::Daily,
    )
    .expect("perf");
    c.bench_function("Performance::sharpe 750x2", |b| {
        b.iter(|| black_box(perf.sharpe(0.02)));
    });
    c.bench_function("Performance::value_at_risk 750x2", |b| {
        b.iter(|| black_box(perf.value_at_risk(0.95)));
    });
}

criterion_group!(
    benches,
    bench_tail_risk,
    bench_return_based,
    bench_drawdown,
    bench_performance,
);
criterion_main!(benches);
