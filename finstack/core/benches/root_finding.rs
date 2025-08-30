use criterion::{black_box, criterion_group, criterion_main, Criterion};
use finstack_core::math::{brent, newton_bracketed};

fn bench_brent(c: &mut Criterion) {
    let f = |x: f64| x * x - 2.0;
    c.bench_function("brent_quadratic", |b| {
        b.iter(|| {
            let r = brent(black_box(f), 1.0, 2.0, 1e-12, 100).unwrap();
            black_box(r)
        })
    });
}

fn bench_newton_bracketed(c: &mut Criterion) {
    let f = |x: f64| x * x * x - x;
    let df = |x: f64| 3.0 * x * x - 1.0;
    c.bench_function("newton_bracketed_cubic", |b| {
        b.iter(|| {
            let r = newton_bracketed(black_box(f), black_box(df), 0.2, 1.5, 1e-12, 100).unwrap();
            black_box(r)
        })
    });
}

criterion_group!(benches, bench_brent, bench_newton_bracketed);
criterion_main!(benches);
