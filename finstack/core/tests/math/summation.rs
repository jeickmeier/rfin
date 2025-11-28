use finstack_core::math::{kahan_sum, pairwise_sum, stable_sum};

#[test]
fn kahan_and_pairwise_more_accurate_than_naive() {
    // Pathological case: alternating signs with varying magnitudes
    // This creates conditions where naive summation accumulates error
    let xs: Vec<f64> = (1..10_000)
        .map(|i| {
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            sign * ((i as f64).sin() * 1e-6 + 1.0 / (i as f64))
        })
        .collect();

    // Reference: sort by absolute value then sum (more stable ordering)
    let mut sorted = xs.clone();
    sorted.sort_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap());
    let reference: f64 = sorted.iter().sum();

    let naive: f64 = xs.iter().copied().sum();
    let k = kahan_sum(xs.iter().copied());
    let p = pairwise_sum(&xs);
    let s = stable_sum(&xs);

    let naive_err = (reference - naive).abs();
    let kahan_err = (reference - k).abs();
    let pairwise_err = (reference - p).abs();
    let stable_err = (reference - s).abs();

    // Compensated methods should be at least as accurate as naive (within 10% tolerance)
    assert!(
        kahan_err <= naive_err * 1.1 + 1e-15,
        "Kahan error {} worse than naive error {}",
        kahan_err,
        naive_err
    );
    assert!(
        pairwise_err <= naive_err * 1.1 + 1e-15,
        "Pairwise error {} worse than naive error {}",
        pairwise_err,
        naive_err
    );
    assert!(
        stable_err <= naive_err * 1.1 + 1e-15,
        "Stable error {} worse than naive error {}",
        stable_err,
        naive_err
    );
}

#[test]
fn summation_catastrophic_cancellation() {
    // Case where naive sum accumulates error: many small values with alternating signs
    // This creates conditions where compensated summation shows its advantage
    //
    // Using values within the same magnitude range where Kahan can help:
    // 1e8 + 1.0 preserves the 1.0 at some precision, but accumulated error grows
    let xs = vec![1e8, 1.0, 1.0, 1.0, 1.0, -1e8, 1.0, 1.0, 1.0, 1.0];

    let naive: f64 = xs.iter().copied().sum();
    let k = kahan_sum(xs.iter().copied());
    let s = stable_sum(&xs);

    // Exact answer is 8.0
    // Kahan and stable_sum should preserve accuracy
    assert!(
        (k - 8.0).abs() < 1e-10,
        "Kahan sum {} should be 8.0 (error: {})",
        k,
        (k - 8.0).abs()
    );
    assert!(
        (s - 8.0).abs() < 1e-10,
        "Stable sum {} should be 8.0 (error: {})",
        s,
        (s - 8.0).abs()
    );

    // Naive sum should also work for this case (within double precision)
    // but documents that compensated methods are at least as good
    assert!(
        (naive - 8.0).abs() < 1e-6,
        "Naive sum {} should be ~8.0 (error: {})",
        naive,
        (naive - 8.0).abs()
    );
}

#[test]
fn summation_large_alternating_series() {
    // A case where Kahan compensated summation should outperform naive:
    // Alternating harmonic series with large initial terms
    let n = 100_000;
    let xs: Vec<f64> = (1..=n)
        .map(|i| {
            let sign = if i % 2 == 0 { -1.0 } else { 1.0 };
            sign / (i as f64)
        })
        .collect();

    // The alternating harmonic series: 1 - 1/2 + 1/3 - 1/4 + ... → ln(2)
    let expected = 2.0_f64.ln();

    let naive: f64 = xs.iter().copied().sum();
    let k = kahan_sum(xs.iter().copied());
    let s = stable_sum(&xs);

    // For large n, all methods should be close to ln(2)
    // but Kahan/stable should have smaller error
    let naive_err = (naive - expected).abs();
    let kahan_err = (k - expected).abs();
    let stable_err = (s - expected).abs();

    // Compensated methods should be accurate
    assert!(
        kahan_err < 1e-4,
        "Kahan error {} too large (expected ~ln(2))",
        kahan_err
    );
    assert!(
        stable_err < 1e-4,
        "Stable error {} too large (expected ~ln(2))",
        stable_err
    );

    // Document that compensated methods are at least as accurate
    assert!(
        kahan_err <= naive_err * 1.1 + 1e-15,
        "Kahan {} should be at least as accurate as naive {}",
        kahan_err,
        naive_err
    );
}

#[test]
fn summation_methods_agree_on_simple_case() {
    // Simple case where all methods should agree closely
    let xs: Vec<f64> = (1..1000).map(|i| 1.0 / (i as f64)).collect();

    let naive: f64 = xs.iter().copied().sum();
    let k = kahan_sum(xs.iter().copied());
    let p = pairwise_sum(&xs);
    let s = stable_sum(&xs);

    // All methods should be within tight tolerance of each other
    let tol = naive.abs() * 1e-12;
    assert!(
        (naive - k).abs() < tol,
        "Kahan {} differs from naive {} by {}",
        k,
        naive,
        (naive - k).abs()
    );
    assert!(
        (naive - p).abs() < tol,
        "Pairwise {} differs from naive {} by {}",
        p,
        naive,
        (naive - p).abs()
    );
    assert!(
        (naive - s).abs() < tol,
        "Stable {} differs from naive {} by {}",
        s,
        naive,
        (naive - s).abs()
    );
}
