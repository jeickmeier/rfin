use criterion::{black_box, criterion_group, criterion_main, Criterion};
use finstack_core::math::solver::{BrentSolver, HybridSolver, Solver};

fn bench_brent(c: &mut Criterion) {
    let f = |x: f64| x * x - 2.0;
    let solver = BrentSolver::new()
        .with_tolerance(1e-12)
        .with_initial_bracket_size(Some(0.5));
    c.bench_function("brent_quadratic", |b| {
        b.iter(|| {
            let r = solver.solve(black_box(f), 1.5).unwrap();
            black_box(r)
        })
    });
}

fn bench_hybrid_solver(c: &mut Criterion) {
    let f = |x: f64| x * x * x - x;
    let solver = HybridSolver::new()
        .with_tolerance(1e-12)
        .with_max_iterations(100);
    c.bench_function("hybrid_solver_cubic", |b| {
        b.iter(|| {
            let r = solver.solve(black_box(f), 0.5).unwrap();
            black_box(r)
        })
    });
}

criterion_group!(benches, bench_brent, bench_hybrid_solver);
criterion_main!(benches);
