use criterion::{BenchmarkGroup, BenchmarkId, Criterion, measurement::WallTime};

/// Simple helper to reduce repetitive `bench_function` + `iter` boilerplate.
pub fn bench_iter<G, F>(group: &mut BenchmarkGroup<WallTime>, id: G, mut f: F)
where
    G: Into<BenchmarkId>,
    F: FnMut(),
{
    group.bench_function(id, |b| b.iter(|| f()));
}

/// Convenience wrapper for single-function benches without an explicit group.
pub fn bench_with_criterion<F>(c: &mut Criterion, name: &str, mut f: F)
where
    F: FnMut(),
{
    c.bench_function(name, |b| b.iter(|| f()));
}

