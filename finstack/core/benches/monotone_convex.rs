use criterion::{black_box, criterion_group, criterion_main, Criterion};
use finstack_core::market_data::interp::{InterpFn, MonotoneConvex};

fn bench_monotone_convex(c: &mut Criterion) {
    // Dense test curve: 101 knots from 0 to 10 years.
    let knots: Vec<f64> = (0..=100).map(|i| i as f64 * 0.1).collect();
    let dfs: Vec<f64> = knots.iter().map(|&t| (-0.03 * t).exp()).collect();
    let interp = MonotoneConvex::new(
        knots.clone().into_boxed_slice(),
        dfs.into_boxed_slice(),
        finstack_core::market_data::interp::ExtrapolationPolicy::default(),
    )
    .unwrap();

    // Evaluation grid – 10_000 samples.
    let evals: Vec<f64> = (0..10_000).map(|i| i as f64 * 0.001).collect();

    c.bench_function("monotone_convex_df", |b| {
        b.iter(|| {
            let mut acc = 0.0;
            for &x in &evals {
                acc += interp.interp(black_box(x));
            }
            black_box(acc)
        })
    });
}

criterion_group!(benches, bench_monotone_convex);
criterion_main!(benches);
