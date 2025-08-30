//! Small statistics helpers (mean/variance/covariance/correlation).
//! 
//! We implement these ourselves rather than using external crates to ensure:
//! - Deterministic results using our custom summation algorithm
//! - Feature-flag controlled behaviour (deterministic vs. fast)
//! - No dependencies on external crates for basic operations
//! - Consistent numerical behaviour across platforms
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::{mean, variance, covariance, correlation};
//! let xs = [1.0, 2.0, 3.0, 4.0];
//! let ys = [2.0, 4.0, 6.0, 8.0];
//! assert!((mean(&xs) - 2.5).abs() < 1e-12);
//! assert!(variance(&xs) > 0.0);
//! assert!(covariance(&xs, &ys) > 0.0);
//! assert!((correlation(&xs, &ys) - 1.0).abs() < 1e-12);
//! ```

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
    // One-pass Chan/Welford style covariance for deterministic, stable accumulation
    let mut mean_x = 0.0;
    let mut mean_y = 0.0;
    let mut co_moment = 0.0;
    let mut k = 0.0;
    for i in 0..n {
        let xi = x[i];
        let yi = y[i];
        k += 1.0;
        let dx = xi - mean_x;
        mean_x += dx / k;
        let dy = yi - mean_y;
        mean_y += dy / k;
        // Use updated mean_y for the second factor per Chan's formulation
        co_moment += dx * (yi - mean_y);
    }
    co_moment / n as f64
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
