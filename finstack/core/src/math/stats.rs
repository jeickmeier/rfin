//! Small statistics helpers (mean/variance/covariance/correlation).

use super::summation::{kahan_sum, pairwise_sum};

/// Arithmetic mean.
pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let s = if cfg!(feature = "deterministic") {
        pairwise_sum(xs)
    } else {
        kahan_sum(xs.iter().copied())
    };
    s / xs.len() as f64
}

/// Variance (population) using a single-pass Welford algorithm (deterministic order).
pub fn variance(xs: &[f64]) -> f64 {
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    let mut mean = 0.0;
    let mut m2 = 0.0;
    let mut k = 0.0;
    for &x in xs {
        k += 1.0;
        let delta = x - mean;
        mean += delta / k;
        let delta2 = x - mean;
        m2 += delta * delta2;
    }
    m2 / n as f64
}

/// Return (mean, variance) pair.
pub fn mean_var(xs: &[f64]) -> (f64, f64) {
    (mean(xs), variance(xs))
}

/// Covariance (population) between two equal-length slices.
pub fn covariance(x: &[f64], y: &[f64]) -> f64 {
    assert_eq!(x.len(), y.len());
    let n = x.len();
    if n == 0 {
        return 0.0;
    }
    let mx = mean(x);
    let my = mean(y);
    let mut acc = 0.0;
    for i in 0..n {
        acc += (x[i] - mx) * (y[i] - my);
    }
    acc / n as f64
}

/// Pearson correlation.
pub fn correlation(x: &[f64], y: &[f64]) -> f64 {
    let cov = covariance(x, y);
    let vx = variance(x);
    let vy = variance(y);
    if vx == 0.0 || vy == 0.0 {
        return 0.0;
    }
    cov / (vx.sqrt() * vy.sqrt())
}
