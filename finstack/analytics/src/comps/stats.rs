//! Cross-sectional statistical analysis for peer sets.
//!
//! Pure functions on `&[f64]` slices consistent with the `analytics` crate
//! style. Provides descriptive statistics, percentile ranking, z-scores,
//! and single-factor OLS regression for fair-value estimation.

use crate::math::stats::{mean, OnlineCovariance, OnlineStats};
use serde::{Deserialize, Serialize};

/// Descriptive statistics for a peer set metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStats {
    /// Number of observations.
    pub count: usize,
    /// Arithmetic mean.
    pub mean: f64,
    /// Median (50th percentile).
    pub median: f64,
    /// Sample standard deviation.
    pub std_dev: f64,
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
    /// 25th percentile.
    pub q1: f64,
    /// 75th percentile.
    pub q3: f64,
}

/// Compute descriptive statistics on a slice of values.
///
/// Returns `None` if the slice is empty. The slice is not modified;
/// an internal sorted copy is used for percentile computations.
pub fn peer_stats(values: &[f64]) -> Option<PeerStats> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    let m = mean(values);
    let median = percentile_sorted(&sorted, 0.50);
    let q1 = percentile_sorted(&sorted, 0.25);
    let q3 = percentile_sorted(&sorted, 0.75);

    let mut os = OnlineStats::new();
    for &v in values {
        os.update(v);
    }

    Some(PeerStats {
        count: n,
        mean: m,
        median,
        std_dev: os.std_dev(),
        min: sorted[0],
        max: sorted[n - 1],
        q1,
        q3,
    })
}

/// Percentile rank of `value` within a peer set (0.0 = cheapest, 1.0 = richest).
///
/// Uses the "percentage of values less than or equal" convention.
/// The input `values` slice need not be sorted.
///
/// Returns `None` if `values` is empty.
pub fn percentile_rank(values: &[f64], value: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let count_le = values.iter().filter(|&&v| v <= value).count();
    Some(count_le as f64 / values.len() as f64)
}

/// Z-score of `value` relative to the peer distribution.
///
/// Returns `None` if fewer than 2 values or standard deviation is zero.
pub fn z_score(values: &[f64], value: f64) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mut os = OnlineStats::new();
    for &v in values {
        os.update(v);
    }
    let sd = os.std_dev();
    if sd < 1e-15 {
        return None;
    }
    Some((value - os.mean()) / sd)
}

/// OLS regression result for fair-value estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionResult {
    /// Intercept (alpha).
    pub intercept: f64,
    /// Slope coefficient (beta).
    pub slope: f64,
    /// R-squared goodness of fit.
    pub r_squared: f64,
    /// Fitted (fair) value for the subject.
    pub fitted_value: f64,
    /// Residual: actual - fitted. Positive = cheap, negative = rich.
    pub residual: f64,
    /// Number of observations used.
    pub n: usize,
}

/// Single-factor OLS regression of `y` on `x`, evaluated at `subject_x`.
///
/// Typical usage: regress OAS spread (y) against leverage (x) across
/// peers, then evaluate the fitted spread for the subject's leverage to
/// determine if it trades rich or cheap to the regression line.
///
/// Uses `OnlineCovariance` for numerically stable single-pass computation.
///
/// Requires at least 3 data points. Returns `None` if the regression
/// cannot be computed (e.g., zero variance in x).
pub fn regression_fair_value(
    x: &[f64],
    y: &[f64],
    subject_x: f64,
    subject_y: f64,
) -> Option<RegressionResult> {
    let n = x.len().min(y.len());
    if n < 3 {
        return None;
    }

    // We regress y on x: y = intercept + slope * x
    // OnlineCovariance.optimal_beta() returns Cov(X,Y)/Var(Y)
    // so we pass (y_i, x_i) to get slope = Cov(Y,X)/Var(X)
    let mut oc = OnlineCovariance::new();
    for i in 0..n {
        oc.update(y[i], x[i]);
    }

    // slope = Cov(Y, X) / Var(X) = optimal_beta when X is passed as second arg
    let slope = oc.optimal_beta();
    let intercept = oc.mean_x() - slope * oc.mean_y();
    let corr = oc.correlation();
    let r_squared = corr * corr;
    let fitted_value = intercept + slope * subject_x;
    let residual = subject_y - fitted_value;

    Some(RegressionResult {
        intercept,
        slope,
        r_squared,
        fitted_value,
        residual,
        n,
    })
}

/// Interpolated percentile from a pre-sorted slice.
///
/// Uses linear interpolation between adjacent ranks.
fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    if n == 1 {
        return sorted[0];
    }
    let rank = p * (n - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = rank - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}
