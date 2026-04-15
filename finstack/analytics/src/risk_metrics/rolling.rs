//! Rolling risk metrics: Sharpe, Sortino, and volatility over a sliding window.
//!
//! All rolling functions share O(n) sliding-window kernels and produce either
//! a dated struct (aligned to window-end dates) or a NaN-padded `Vec<f64>`.

use crate::dates::Date;
use finstack_core::math::neumaier_sum;

use super::return_based::sharpe;

/// Number of slide steps before the incremental rolling kernel fully
/// recomputes its running sums from the current window.
///
/// The rolling kernels update `sum` and `sum_sq` incrementally on each
/// slide, which accumulates floating-point drift over time. Every
/// `ROLLING_KERNEL_RECOMPUTE_INTERVAL` steps we recompute those values
/// over the full window with [`neumaier_sum`] to restore precision.
const ROLLING_KERNEL_RECOMPUTE_INTERVAL: usize = 1024;

#[inline]
fn recompute_sum_sum_sq(window: &[f64]) -> (f64, f64) {
    (
        neumaier_sum(window.iter().copied()),
        neumaier_sum(window.iter().map(|r| r * r)),
    )
}

#[inline]
fn recompute_sum_sum_ds(window: &[f64]) -> (f64, f64) {
    (
        neumaier_sum(window.iter().copied()),
        neumaier_sum(window.iter().filter(|&&r| r < 0.0).map(|&r| r * r)),
    )
}

/// Output of a rolling Sharpe ratio computation.
///
/// Contains parallel vectors of Sharpe values and their corresponding
/// window-end dates, suitable for time-series charting.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RollingSharpe {
    /// Rolling Sharpe ratio values.
    pub values: Vec<f64>,
    /// End dates for each rolling window.
    pub dates: Vec<Date>,
}

/// Rolling Sharpe ratio over a sliding window.
///
/// Computes the Sharpe ratio independently for each `window`-length sub-slice
/// of `returns`, advancing one period at a time. Produces `n - window + 1`
/// values where `n = min(returns.len(), dates.len())`.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
/// * `dates` - Date vector aligned with `returns`. Used only for labelling
///   output; must be at least as long as `returns`.
/// * `window` - Look-back window length in periods.
/// * `ann_factor` - Number of periods per year for annualization.
/// * `risk_free_rate` - Annualized risk-free rate to subtract.
///
/// # Returns
///
/// A [`RollingSharpe`] with `values` and `dates` of equal length. Returns
/// empty vectors if `window` is zero or larger than the series length.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::rolling_sharpe;
/// use finstack_core::dates::{Date, Duration, Month};
///
/// let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
/// let dates: Vec<Date> = (0..20)
///     .map(|i| Date::from_calendar_date(2025, Month::January, 1).unwrap()
///         + Duration::days(i))
///     .collect();
/// let rs = rolling_sharpe(&returns, &dates, 5, 252.0, 0.0);
/// assert_eq!(rs.values.len(), 16); // 20 − 5 + 1
/// ```
pub fn rolling_sharpe(
    returns: &[f64],
    dates: &[Date],
    window: usize,
    ann_factor: f64,
    risk_free_rate: f64,
) -> RollingSharpe {
    let n = returns.len().min(dates.len());
    if n < window || window == 0 {
        return RollingSharpe {
            values: vec![],
            dates: vec![],
        };
    }
    let w = window as f64;
    let mut values = Vec::with_capacity(n - window + 1);
    let mut out_dates = Vec::with_capacity(n - window + 1);
    let mut date_idx = window - 1;
    rolling_sum_sum_sq_kernel(returns, n, window, |sum, sum_sq| {
        let ann_mean = (sum / w) * ann_factor;
        let var = (sum_sq - sum * sum / w).max(0.0) / (w - 1.0);
        let ann_vol = var.sqrt() * ann_factor.sqrt();
        values.push(sharpe(ann_mean, ann_vol, risk_free_rate));
        out_dates.push(dates[date_idx]);
        date_idx += 1;
    });
    RollingSharpe {
        values,
        dates: out_dates,
    }
}

/// Output of a rolling volatility computation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RollingVolatility {
    /// Rolling annualized volatility values.
    pub values: Vec<f64>,
    /// End dates for each rolling window.
    pub dates: Vec<Date>,
}

/// Rolling annualized volatility over a sliding window.
///
/// Computes annualized volatility independently for each `window`-length
/// sub-slice, advancing one period at a time.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `dates`      - Date vector aligned with `returns`.
/// * `window`     - Look-back window length in periods.
/// * `ann_factor` - Number of periods per year for annualization.
///
/// # Returns
///
/// A [`RollingVolatility`] with `n - window + 1` values. Returns empty
/// vectors if `window` is zero or larger than the series length.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::rolling_volatility;
/// use finstack_core::dates::{Date, Duration, Month};
///
/// let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
/// let dates: Vec<Date> = (0..20)
///     .map(|i| Date::from_calendar_date(2025, Month::January, 1).unwrap()
///         + Duration::days(i))
///     .collect();
/// let rv = rolling_volatility(&returns, &dates, 5, 252.0);
/// assert_eq!(rv.values.len(), 16);
/// ```
pub fn rolling_volatility(
    returns: &[f64],
    dates: &[Date],
    window: usize,
    ann_factor: f64,
) -> RollingVolatility {
    let n = returns.len().min(dates.len());
    if n < window || window == 0 {
        return RollingVolatility {
            values: vec![],
            dates: vec![],
        };
    }
    let w = window as f64;
    let mut values = Vec::with_capacity(n - window + 1);
    let mut out_dates = Vec::with_capacity(n - window + 1);
    let mut date_idx = window - 1;
    rolling_sum_sum_sq_kernel(returns, n, window, |sum, sum_sq| {
        let var = (sum_sq - sum * sum / w).max(0.0) / (w - 1.0);
        values.push(var.sqrt() * ann_factor.sqrt());
        out_dates.push(dates[date_idx]);
        date_idx += 1;
    });
    RollingVolatility {
        values,
        dates: out_dates,
    }
}

/// Output of a rolling Sortino ratio computation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RollingSortino {
    /// Rolling Sortino ratio values.
    pub values: Vec<f64>,
    /// End dates for each rolling window.
    pub dates: Vec<Date>,
}

/// Rolling Sortino ratio over a sliding window.
///
/// Computes the Sortino ratio independently for each `window`-length
/// sub-slice, advancing one period at a time.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `dates`      - Date vector aligned with `returns`.
/// * `window`     - Look-back window length in periods.
/// * `ann_factor` - Number of periods per year for annualization.
///
/// # Returns
///
/// A [`RollingSortino`] with `n - window + 1` values. Returns empty
/// vectors if `window` is zero or larger than the series length.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::rolling_sortino;
/// use finstack_core::dates::{Date, Duration, Month};
///
/// let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
/// let dates: Vec<Date> = (0..20)
///     .map(|i| Date::from_calendar_date(2025, Month::January, 1).unwrap()
///         + Duration::days(i))
///     .collect();
/// let rs = rolling_sortino(&returns, &dates, 5, 252.0);
/// assert_eq!(rs.values.len(), 16);
/// ```
pub fn rolling_sortino(
    returns: &[f64],
    dates: &[Date],
    window: usize,
    ann_factor: f64,
) -> RollingSortino {
    let n = returns.len().min(dates.len());
    if n < window || window == 0 {
        return RollingSortino {
            values: vec![],
            dates: vec![],
        };
    }
    let w = window as f64;
    let mut values = Vec::with_capacity(n - window + 1);
    let mut out_dates = Vec::with_capacity(n - window + 1);
    let mut date_idx = window - 1;
    rolling_sortino_kernel(returns, n, window, |sum, sum_ds| {
        let m = sum / w;
        let dd = (sum_ds / w).sqrt();
        values.push(if dd == 0.0 {
            if m > 0.0 {
                f64::INFINITY
            } else if m < 0.0 {
                f64::NEG_INFINITY
            } else {
                0.0
            }
        } else {
            (m * ann_factor) / (dd * ann_factor.sqrt())
        });
        out_dates.push(dates[date_idx]);
        date_idx += 1;
    });
    RollingSortino {
        values,
        dates: out_dates,
    }
}

// ── Internal rolling kernels ──
//
// Each kernel advances a sliding window over `returns[..n]` and calls
// `emit` once per completed window.  Both the dated (struct) and undated
// (NaN-padded Vec) public functions delegate here; they share *identical*
// arithmetic and differ only in how the output is assembled.

/// Shared kernel for rolling metrics that only need `sum` and `sum_sq`.
///
/// Maintains `(sum, sum_sq)` accumulators and calls `emit(sum, sum_sq)` for
/// every completed window starting at index `window-1`.
fn rolling_sum_sum_sq_kernel<F>(returns: &[f64], n: usize, window: usize, mut emit: F)
where
    F: FnMut(f64, f64),
{
    let (mut sum, mut sum_sq) = recompute_sum_sum_sq(&returns[..window]);
    emit(sum, sum_sq);
    let mut steps_since_recompute = 0_usize;
    for i in window..n {
        let add = returns[i];
        let rem = returns[i - window];
        sum += add - rem;
        sum_sq += add * add - rem * rem;
        steps_since_recompute += 1;
        if steps_since_recompute >= ROLLING_KERNEL_RECOMPUTE_INTERVAL {
            let start = i + 1 - window;
            (sum, sum_sq) = recompute_sum_sum_sq(&returns[start..=i]);
            steps_since_recompute = 0;
        }
        emit(sum, sum_sq);
    }
}

/// Kernel for the Sortino sliding window.
///
/// Maintains `(sum, sum_ds)` where `sum_ds = Σ min(r,0)²` and calls
/// `emit(sum, sum_ds)` for every completed window.
fn rolling_sortino_kernel<F>(returns: &[f64], n: usize, window: usize, mut emit: F)
where
    F: FnMut(f64, f64),
{
    let (mut sum, mut sum_ds) = recompute_sum_sum_ds(&returns[..window]);
    emit(sum, sum_ds);
    let mut steps_since_recompute = 0_usize;
    for i in window..n {
        let add = returns[i];
        let rem = returns[i - window];
        sum += add - rem;
        if add < 0.0 {
            sum_ds += add * add;
        }
        if rem < 0.0 {
            sum_ds -= rem * rem;
        }
        sum_ds = sum_ds.max(0.0); // guard against floating-point underflow
        steps_since_recompute += 1;
        if steps_since_recompute >= ROLLING_KERNEL_RECOMPUTE_INTERVAL {
            let start = i + 1 - window;
            (sum, sum_ds) = recompute_sum_sum_ds(&returns[start..=i]);
            steps_since_recompute = 0;
        }
        emit(sum, sum_ds);
    }
}

/// Rolling Sharpe ratio without dates, returning NaN-padded output.
///
/// The first `window - 1` values are `NaN`, followed by the Sharpe ratio
/// for each completed window. Uses the same O(n) sliding-window approach
/// as [`rolling_sharpe`].
///
/// # Arguments
///
/// * `returns`        - Slice of period simple returns.
/// * `window`         - Look-back window length in periods.
/// * `ann_factor`     - Number of periods per year for annualization.
/// * `risk_free_rate` - Annualized risk-free rate to subtract from return.
pub fn rolling_sharpe_values(
    returns: &[f64],
    window: usize,
    ann_factor: f64,
    risk_free_rate: f64,
) -> Vec<f64> {
    let n = returns.len();
    if n < window || window == 0 {
        return vec![];
    }
    let w = window as f64;
    let mut out = Vec::with_capacity(n);
    out.resize(window - 1, f64::NAN);
    rolling_sum_sum_sq_kernel(returns, n, window, |sum, sum_sq| {
        let ann_mean = (sum / w) * ann_factor;
        let var = (sum_sq - sum * sum / w).max(0.0) / (w - 1.0);
        let ann_vol = var.sqrt() * ann_factor.sqrt();
        out.push(sharpe(ann_mean, ann_vol, risk_free_rate));
    });
    out
}

/// Rolling annualized volatility without dates, returning NaN-padded output.
///
/// The first `window - 1` values are `NaN`, followed by the annualized
/// volatility for each completed window.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `window`     - Look-back window length in periods.
/// * `ann_factor` - Number of periods per year for annualization.
pub fn rolling_volatility_values(returns: &[f64], window: usize, ann_factor: f64) -> Vec<f64> {
    let n = returns.len();
    if n < window || window == 0 {
        return vec![];
    }
    let w = window as f64;
    let mut out = Vec::with_capacity(n);
    out.resize(window - 1, f64::NAN);
    rolling_sum_sum_sq_kernel(returns, n, window, |sum, sum_sq| {
        let var = (sum_sq - sum * sum / w).max(0.0) / (w - 1.0);
        out.push(var.sqrt() * ann_factor.sqrt());
    });
    out
}

/// Rolling Sortino ratio without dates, returning NaN-padded output.
///
/// The first `window - 1` values are `NaN`, followed by the Sortino ratio
/// for each completed window.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `window`     - Look-back window length in periods.
/// * `ann_factor` - Number of periods per year for annualization.
pub fn rolling_sortino_values(returns: &[f64], window: usize, ann_factor: f64) -> Vec<f64> {
    let n = returns.len();
    if n < window || window == 0 {
        return vec![];
    }
    let w = window as f64;
    let mut out = Vec::with_capacity(n);
    out.resize(window - 1, f64::NAN);
    rolling_sortino_kernel(returns, n, window, |sum, sum_ds| {
        let m = sum / w;
        let dd = (sum_ds / w).sqrt();
        out.push(if dd == 0.0 {
            if m > 0.0 {
                f64::INFINITY
            } else if m < 0.0 {
                f64::NEG_INFINITY
            } else {
                0.0
            }
        } else {
            (m * ann_factor) / (dd * ann_factor.sqrt())
        });
    });
    out
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::dates::{Duration, Month};
    use crate::risk_metrics::return_based::{sortino, volatility};
    use finstack_core::math::neumaier_sum;

    fn jan1(year: i32) -> Date {
        Date::from_calendar_date(year, Month::January, 1).expect("valid date")
    }

    #[test]
    fn rolling_sharpe_window() {
        let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
        let dates: Vec<Date> = (0..20).map(|i| jan1(2025) + Duration::days(i)).collect();
        let rs = rolling_sharpe(&returns, &dates, 5, 252.0, 0.0);
        assert_eq!(rs.values.len(), 16);
    }

    #[test]
    fn rolling_volatility_window_count() {
        let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
        let dates: Vec<Date> = (0..20).map(|i| jan1(2025) + Duration::days(i)).collect();
        let rv = rolling_volatility(&returns, &dates, 5, 252.0);
        assert_eq!(rv.values.len(), 16);
        assert_eq!(rv.dates.len(), 16);
        assert!(rv.values.iter().all(|&v| v > 0.0));
    }

    #[test]
    fn rolling_volatility_matches_pointwise() {
        let returns: Vec<f64> = (0..10).map(|i| (i as f64 - 5.0) * 0.01).collect();
        let dates: Vec<Date> = (0..10).map(|i| jan1(2025) + Duration::days(i)).collect();
        let rv = rolling_volatility(&returns, &dates, 5, 252.0);
        let first_window = volatility(&returns[0..5], true, 252.0);
        assert!((rv.values[0] - first_window).abs() < 1e-12);
    }

    #[test]
    fn rolling_volatility_empty_window() {
        let rv = rolling_volatility(&[0.01], &[jan1(2025)], 5, 252.0);
        assert!(rv.values.is_empty());
    }

    #[test]
    fn rolling_sortino_window_count() {
        let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
        let dates: Vec<Date> = (0..20).map(|i| jan1(2025) + Duration::days(i)).collect();
        let rs = rolling_sortino(&returns, &dates, 5, 252.0);
        assert_eq!(rs.values.len(), 16);
        assert_eq!(rs.dates.len(), 16);
    }

    #[test]
    fn rolling_sortino_matches_pointwise() {
        let returns: Vec<f64> = (0..10).map(|i| (i as f64 - 5.0) * 0.01).collect();
        let dates: Vec<Date> = (0..10).map(|i| jan1(2025) + Duration::days(i)).collect();
        let rs = rolling_sortino(&returns, &dates, 5, 252.0);
        let first_window = sortino(&returns[0..5], true, 252.0);
        assert!((rs.values[0] - first_window).abs() < 1e-12);
    }

    #[test]
    fn rolling_sum_sq_kernel_limits_long_run_drift() {
        fn next_u64(state: &mut u64) -> u64 {
            *state ^= *state << 13;
            *state ^= *state >> 7;
            *state ^= *state << 17;
            *state
        }

        let mut state = 0x1234_5678_9abc_def0_u64;
        let returns: Vec<f64> = (0..20_000)
            .map(|_| {
                let u = next_u64(&mut state) as f64 / u64::MAX as f64;
                let v = next_u64(&mut state) as f64 / u64::MAX as f64;
                (u - 0.5) * 1.0e10 + (v - 0.5) * 1.0e-3
            })
            .collect();
        let window = 257;
        let n = returns.len();
        let mut max_sum_sq_error = 0.0_f64;
        let mut start = 0_usize;

        rolling_sum_sum_sq_kernel(&returns, n, window, |_, sum_sq| {
            let end = start + window;
            let exact_sum_sq = neumaier_sum(returns[start..end].iter().map(|r| r * r));
            max_sum_sq_error = max_sum_sq_error.max((sum_sq - exact_sum_sq).abs());
            start += 1;
        });

        assert!(
            max_sum_sq_error < 10_000_000.0,
            "rolling sum_sq drift should stay bounded, got {}",
            max_sum_sq_error
        );
    }
}
