//! Monte Carlo pricing benchmarks.
//!
//! Benchmarks for different MC features:
//! - European options (GBM)
//! - Asian options
//! - Barrier options
//! - Heston stochastic vol
//! - LSMC American options
//! - Parallel scaling

#![cfg(feature = "mc")]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::mc::discretization::qe_heston::QeHeston;
use finstack_valuations::instruments::common::mc::process::gbm::{GbmParams, GbmProcess};
use finstack_valuations::instruments::common::mc::process::heston::HestonProcess;
use finstack_valuations::instruments::common::models::monte_carlo::payoff::asian::{
    AsianCall, AveragingMethod,
};
use finstack_valuations::instruments::common::models::monte_carlo::payoff::barrier::{
    BarrierCall, BarrierType,
};
use finstack_valuations::instruments::OptionType;
use finstack_valuations::instruments::common::models::monte_carlo::payoff::vanilla::EuropeanCall;
use finstack_valuations::instruments::common::models::monte_carlo::pricer::european::{
    EuropeanPricer, EuropeanPricerConfig,
};
use finstack_valuations::instruments::common::models::monte_carlo::pricer::lsmc::{
    AmericanPut, LsmcConfig, LsmcPricer, PolynomialBasis,
};
use finstack_valuations::instruments::common::models::monte_carlo::pricer::path_dependent::{
    PathDependentPricer, PathDependentPricerConfig,
};

fn bench_european_gbm(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_european_gbm");

    for num_paths in [10_000, 50_000, 100_000] {
        group.throughput(Throughput::Elements(num_paths as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_paths),
            &num_paths,
            |b, &n| {
                let config = EuropeanPricerConfig::new(n)
                    .with_seed(42)
                    .with_parallel(false);
                let pricer = EuropeanPricer::new(config);
                let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
                let call = EuropeanCall::new(100.0, 1.0, 252);

                b.iter(|| {
                    let result = pricer
                        .price(
                            black_box(&gbm),
                            100.0,
                            1.0,
                            252,
                            black_box(&call),
                            Currency::USD,
                            0.95,
                        )
                        .unwrap();
                    black_box(result)
                });
            },
        );
    }
    group.finish();
}

fn bench_asian_options(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_asian");

    let fixing_steps: Vec<usize> = (0..=12).map(|i| i * 21).collect();
    let asian = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps);
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    group.bench_function("asian_arithmetic", |b| {
        let config = PathDependentPricerConfig::new(50_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = PathDependentPricer::new(config);

        b.iter(|| {
            let result = pricer
                .price(
                    black_box(&gbm),
                    100.0,
                    1.0,
                    252,
                    black_box(&asian),
                    Currency::USD,
                    1.0,
                )
                .unwrap();
            black_box(result)
        });
    });

    group.finish();
}

fn bench_barrier_options(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_barrier");

    let barrier = BarrierCall::new(
        100.0,
        120.0,
        BarrierType::UpAndOut,
        OptionType::Call,
        None,
        1.0,
        252,
        0.2,
        1.0,
        true,
    );
    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

    group.bench_function("barrier_up_and_out", |b| {
        let config = PathDependentPricerConfig::new(50_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = PathDependentPricer::new(config);

        b.iter(|| {
            let result = pricer
                .price(
                    black_box(&gbm),
                    100.0,
                    1.0,
                    252,
                    black_box(&barrier),
                    Currency::USD,
                    1.0,
                )
                .unwrap();
            black_box(result)
        });
    });

    group.finish();
}

fn bench_heston(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_heston");

    use finstack_valuations::instruments::common::mc::rng::philox::PhiloxRng;
    use finstack_valuations::instruments::common::mc::time_grid::TimeGrid;
    use finstack_valuations::instruments::common::models::monte_carlo::engine::{
        McEngine, McEngineConfig,
    };

    group.bench_function("heston_european", |b| {
        let time_grid = TimeGrid::uniform(1.0, 252).unwrap();
        let engine = McEngine::new(McEngineConfig {
            num_paths: 50_000,
            seed: 42,
            time_grid,
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
            path_capture: finstack_valuations::instruments::common::models::monte_carlo::engine::
                PathCaptureConfig::default(),
            antithetic: false,
        });

        let heston = HestonProcess::with_params(0.05, 0.02, 2.0, 0.04, 0.3, -0.7, 0.04);
        let disc = QeHeston::new();
        let call = EuropeanCall::new(100.0, 1.0, 252);
        let rng = PhiloxRng::new(42);

        b.iter(|| {
            let result = engine
                .price(
                    black_box(&rng),
                    black_box(&heston),
                    black_box(&disc),
                    &[100.0, 0.04],
                    black_box(&call),
                    Currency::USD,
                    0.95,
                )
                .unwrap();
            black_box(result)
        });
    });

    group.finish();
}

fn bench_lsmc_american(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_lsmc");

    group.bench_function("american_put", |b| {
        let exercise_dates: Vec<usize> = (25..=100).step_by(25).collect();
        let config = LsmcConfig::new(10_000, exercise_dates).with_seed(42);
        let pricer = LsmcPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.3));
        let put = AmericanPut { strike: 100.0 };
        let basis = PolynomialBasis::new(2);

        b.iter(|| {
            let result = pricer
                .price(
                    black_box(&gbm),
                    100.0,
                    1.0,
                    100,
                    black_box(&put),
                    black_box(&basis),
                    Currency::USD,
                    0.05,
                )
                .unwrap();
            black_box(result)
        });
    });

    group.finish();
}

#[cfg(feature = "parallel")]
fn bench_parallel_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("mc_parallel_scaling");

    let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
    let call = EuropeanCall::new(100.0, 1.0, 252);

    // Serial
    group.bench_function("serial_100k", |b| {
        let config = EuropeanPricerConfig::new(100_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = EuropeanPricer::new(config);

        b.iter(|| {
            let result = pricer
                .price(
                    black_box(&gbm),
                    100.0,
                    1.0,
                    252,
                    black_box(&call),
                    Currency::USD,
                    0.95,
                )
                .unwrap();
            black_box(result)
        });
    });

    // Parallel
    group.bench_function("parallel_100k", |b| {
        let config = EuropeanPricerConfig::new(100_000)
            .with_seed(42)
            .with_parallel(true);
        let pricer = EuropeanPricer::new(config);

        b.iter(|| {
            let result = pricer
                .price(
                    black_box(&gbm),
                    100.0,
                    1.0,
                    252,
                    black_box(&call),
                    Currency::USD,
                    0.95,
                )
                .unwrap();
            black_box(result)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_european_gbm,
    bench_asian_options,
    bench_barrier_options,
    bench_heston,
    bench_lsmc_american,
);

#[cfg(feature = "parallel")]
criterion_group!(parallel_benches, bench_parallel_scaling);

#[cfg(feature = "parallel")]
criterion_main!(benches, parallel_benches);

#[cfg(not(feature = "parallel"))]
criterion_main!(benches);
