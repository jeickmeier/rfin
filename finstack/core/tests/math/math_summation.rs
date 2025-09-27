use finstack_core::math::{kahan_sum, pairwise_sum, stable_sum};

#[test]
fn kahan_and_pairwise_close_to_naive() {
    let xs: Vec<f64> = (1..10_000)
        .map(|i| ((i as f64).sin() * 1e-6) + (1.0 / (i as f64)))
        .collect();
    let naive: f64 = xs.iter().copied().sum();
    let k = kahan_sum(xs.iter().copied());
    let p = pairwise_sum(&xs);
    let s = stable_sum(&xs);
    let tol = 1e-9_f64.max(naive.abs() * 1e-12);
    assert!((naive - k).abs() < tol * 100.0);
    assert!((naive - p).abs() < tol * 100.0);
    assert!((naive - s).abs() < tol * 100.0);
}
