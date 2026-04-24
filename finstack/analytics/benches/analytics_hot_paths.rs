//! Criterion benchmarks for `finstack-analytics` hot paths.
//!
//! Covers the metrics most likely to sit on hot request or batch paths in
//! production:
//!
//! - tail-risk quantiles (`value_at_risk`, `expected_shortfall`)
//! - return-based ratios (`sharpe`, `volatility`)
//! - drawdown series construction
//! - GARCH(1,1) fit
//! - `Performance::new` construction + summary stats

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use finstack_analytics::backtesting::{rolling_var_forecasts, VarMethod};
use finstack_analytics::drawdown::to_drawdown_series;
use finstack_analytics::risk_metrics::{expected_shortfall, sharpe, value_at_risk, volatility};
use finstack_analytics::timeseries::{Garch11, GarchModel, InnovationDist};
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
            // Center around zero, small scale.
            (u - 0.5) * 0.04
        })
        .collect()
}

fn bench_tail_risk(c: &mut Criterion) {
    let r = synthetic_returns(2_500, 42);
    c.bench_function("value_at_risk 2.5k", |b| {
        b.iter(|| black_box(value_at_risk(black_box(&r), 0.95)));
    });
    c.bench_function("expected_shortfall 2.5k", |b| {
        b.iter(|| black_box(expected_shortfall(black_box(&r), 0.95)));
    });
}

fn bench_return_based(c: &mut Criterion) {
    let r = synthetic_returns(2_500, 7);
    c.bench_function("volatility 2.5k", |b| {
        b.iter(|| black_box(volatility(black_box(&r), true, 252.0)));
    });
    c.bench_function("sharpe 2.5k", |b| {
        b.iter(|| black_box(sharpe(0.08, 0.12, 0.02)));
    });
}

fn bench_drawdown(c: &mut Criterion) {
    let r = synthetic_returns(10_000, 11);
    c.bench_function("to_drawdown_series 10k", |b| {
        b.iter(|| black_box(to_drawdown_series(black_box(&r))));
    });
}

fn bench_rolling_var(c: &mut Criterion) {
    let r = synthetic_returns(1_000, 17);
    c.bench_function("rolling_var_forecasts hist 1k/250", |b| {
        b.iter(|| {
            black_box(rolling_var_forecasts(
                black_box(&r),
                250,
                0.99,
                VarMethod::Historical,
            ))
        });
    });
}

fn bench_garch11(c: &mut Criterion) {
    let r = synthetic_returns(500, 23);
    c.bench_function("fit Garch11 500 gaussian", |b| {
        b.iter(|| black_box(Garch11.fit(black_box(&r), InnovationDist::Gaussian, None)));
    });
}

fn bench_performance(c: &mut Criterion) {
    let n = 750;
    let start = Date::from_calendar_date(2020, Month::January, 1).expect("valid");
    let mut dates = Vec::with_capacity(n);
    let mut d = start;
    for _ in 0..n {
        dates.push(d);
        d = d.next_day().expect("next day");
    }
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
    bench_rolling_var,
    bench_garch11,
    bench_performance,
);
criterion_main!(benches);
