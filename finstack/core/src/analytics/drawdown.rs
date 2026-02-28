//! Drawdown computation: series, episode detection, averaging, and CDaR.
//!
//! Drawdown measures the peak-to-trough decline in cumulative wealth.
//! This module provides four levels of granularity:
//! - [`to_drawdown_series`]: per-period drawdown depth as a time series.
//! - [`drawdown_details`]: structured episodes (start, valley, recovery).
//! - [`avg_drawdown`]: scalar average of the worst N episodes.
//! - [`cdar`]: Conditional Drawdown at Risk at a given confidence level.

use crate::dates::Date;
use crate::math::stats::quantile;

/// Drawdown episode with start, valley, optional recovery, and max drawdown.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawdownEpisode {
    /// Date when the drawdown began (peak date).
    pub start: Date,
    /// Date of the maximum drawdown depth.
    pub valley: Date,
    /// Date when wealth recovered to the prior peak (None if still in drawdown).
    pub end: Option<Date>,
    /// Calendar days from start to end (or last observation).
    pub duration_days: i64,
    /// Maximum drawdown depth (negative fraction, e.g. −0.25 for a 25% loss).
    pub max_drawdown: f64,
    /// Near-recovery threshold: the drawdown level at which 99% of the
    /// peak-to-trough loss has been recovered (i.e. `max_drawdown * 0.01`,
    /// a value slightly below zero). Useful for identifying "almost recovered"
    /// drawdowns where the series is within 1% of the prior peak.
    pub near_recovery_threshold: f64,
}

/// Compute a drawdown series from a simple-return series.
///
/// At each time step `i`, the drawdown depth is:
///
/// ```text
/// dd[i] = wealth[i] / peak[i] - 1  (≤ 0)
/// ```
///
/// where `wealth[i] = Π(1 + r[j]) for j ≤ i` and `peak[i]` is the running
/// maximum of wealth up to and including `i`.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns (e.g., `0.01` = +1 %).
///
/// # Returns
///
/// A `Vec<f64>` of the same length as `returns`. Each value is ≤ 0;
/// a value of `0.0` means wealth is at or above its prior peak.
/// Returns an empty vector if `returns` is empty.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::drawdown::to_drawdown_series;
///
/// // No drawdown while returns are positive.
/// let dd = to_drawdown_series(&[0.10, 0.05]);
/// assert!(dd.iter().all(|&v| v.abs() < 1e-12));
///
/// // 20% loss after a 10% gain: wealth = 1.10 * 0.80 = 0.88 → dd ≈ −0.2
/// let dd = to_drawdown_series(&[0.10, -0.20]);
/// assert!(dd[1] < -0.18);
/// ```
pub fn to_drawdown_series(returns: &[f64]) -> Vec<f64> {
    if returns.is_empty() {
        return vec![];
    }
    let mut wealth = 1.0;
    let mut peak = 1.0;
    let mut dd = Vec::with_capacity(returns.len());
    for &r in returns {
        wealth *= 1.0 + r;
        if wealth > peak {
            peak = wealth;
        }
        dd.push(wealth / peak - 1.0);
    }
    dd
}

/// Detect individual drawdown episodes from a drawdown series.
///
/// An episode begins when the drawdown series first drops below zero
/// (i.e., wealth falls from its peak) and ends when it returns to zero
/// (recovery to a new peak). The function collects all such episodes,
/// sorts them by severity (most negative `max_drawdown` first), and
/// returns the worst `n`.
///
/// # Arguments
///
/// * `drawdown` - Pre-computed drawdown series, as produced by [`to_drawdown_series`].
/// * `dates`    - Date vector aligned with `drawdown`. Must be the same length or longer.
/// * `n`        - Maximum number of episodes to return.
///
/// # Returns
///
/// Up to `n` [`DrawdownEpisode`] structs sorted by `max_drawdown` ascending
/// (most severe first). Returns an empty vector if `drawdown` or `dates` is empty.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::drawdown::{to_drawdown_series, drawdown_details};
/// use time::{Date, Month};
///
/// let returns = [0.10, -0.20, 0.05, 0.10];
/// let dd = to_drawdown_series(&returns);
/// let dates: Vec<Date> = (1..=4)
///     .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
///     .collect();
/// let episodes = drawdown_details(&dd, &dates, 5);
/// assert!(!episodes.is_empty());
/// assert!(episodes[0].max_drawdown < 0.0);
/// ```
pub fn drawdown_details(drawdown: &[f64], dates: &[Date], n: usize) -> Vec<DrawdownEpisode> {
    if drawdown.is_empty() || dates.is_empty() {
        return vec![];
    }
    let len = drawdown.len().min(dates.len());

    let mut episodes: Vec<DrawdownEpisode> = Vec::new();
    let mut in_dd = false;
    let mut start_idx = 0usize;
    let mut valley_idx = 0usize;
    let mut valley_val = 0.0_f64;

    for (i, &d) in drawdown.iter().enumerate().take(len) {
        if d < -1e-15 {
            if !in_dd {
                in_dd = true;
                start_idx = if i > 0 { i - 1 } else { 0 };
                valley_idx = i;
                valley_val = d;
            } else if d < valley_val {
                valley_idx = i;
                valley_val = d;
            }
        } else if in_dd {
            let ep = make_episode(dates, start_idx, valley_idx, Some(i), valley_val);
            episodes.push(ep);
            in_dd = false;
        }
    }
    if in_dd {
        let ep = make_episode(dates, start_idx, valley_idx, None, valley_val);
        episodes.push(ep);
    }

    episodes.sort_by(|a, b| {
        a.max_drawdown
            .partial_cmp(&b.max_drawdown)
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    episodes.truncate(n);
    episodes
}

fn make_episode(
    dates: &[Date],
    start_idx: usize,
    valley_idx: usize,
    end_idx: Option<usize>,
    valley_val: f64,
) -> DrawdownEpisode {
    let start = dates[start_idx];
    let valley = dates[valley_idx];
    let end = end_idx.map(|i| dates[i]);
    let end_date = end.unwrap_or(dates[dates.len() - 1]);
    let duration_days = (end_date - start).whole_days();
    DrawdownEpisode {
        start,
        valley,
        end,
        duration_days,
        max_drawdown: valley_val,
        near_recovery_threshold: valley_val * 0.01,
    }
}

/// Average of the top-N worst drawdowns.
///
/// Identifies the `n` largest drawdown episodes via [`drawdown_details`]
/// and returns the arithmetic mean of their `max_drawdown` values.
///
/// # Arguments
///
/// * `drawdown` - Pre-computed drawdown series (values ≤ 0).
/// * `dates`    - Date vector aligned with `drawdown`.
/// * `n`        - Number of worst episodes to average.
///
/// # Returns
///
/// Mean of the `n` worst `max_drawdown` values (a negative number), or
/// `0.0` if `drawdown` is empty or no episodes are found.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::drawdown::{to_drawdown_series, avg_drawdown};
/// use time::{Date, Month};
///
/// let returns = [0.05, -0.15, 0.10, -0.08, 0.03];
/// let dd = to_drawdown_series(&returns);
/// let dates: Vec<Date> = (1..=5)
///     .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
///     .collect();
/// let avg = avg_drawdown(&dd, &dates, 3);
/// assert!(avg < 0.0);
/// ```
pub fn avg_drawdown(drawdown: &[f64], dates: &[Date], n: usize) -> f64 {
    let episodes = drawdown_details(drawdown, dates, n);
    if episodes.is_empty() {
        return 0.0;
    }
    let sum: f64 = episodes.iter().map(|e| e.max_drawdown).sum();
    sum / episodes.len() as f64
}

/// Maximum drawdown duration in calendar days across all episodes.
///
/// Identifies all drawdown episodes and returns the longest `duration_days`.
///
/// # Arguments
///
/// * `drawdown` - Pre-computed drawdown series (values ≤ 0).
/// * `dates`    - Date vector aligned with `drawdown`.
///
/// # Returns
///
/// Duration in calendar days of the longest drawdown episode. Returns `0`
/// if no episodes are found.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::drawdown::{to_drawdown_series, max_drawdown_duration};
/// use time::{Date, Month};
///
/// let returns = [0.10, -0.20, 0.05, 0.10, -0.05, -0.03];
/// let dd = to_drawdown_series(&returns);
/// let dates: Vec<Date> = (1..=6)
///     .map(|d| Date::from_calendar_date(2025, Month::January, d).unwrap())
///     .collect();
/// let max_dur = max_drawdown_duration(&dd, &dates);
/// assert!(max_dur > 0);
/// ```
pub fn max_drawdown_duration(drawdown: &[f64], dates: &[Date]) -> i64 {
    let episodes = drawdown_details(drawdown, dates, usize::MAX);
    episodes.iter().map(|e| e.duration_days).max().unwrap_or(0)
}

/// Conditional Drawdown at Risk (CDaR) at the given confidence level.
///
/// The expected drawdown depth in the tail beyond the `(1 − α)` quantile
/// of the drawdown distribution:
///
/// ```text
/// CDaR_α = E[ |dd| | |dd| ≥ q_{1−α}(|dd|) ]
/// ```
///
/// CDaR is the drawdown analogue of Expected Shortfall (CVaR).
///
/// # Arguments
///
/// * `drawdown`   - Pre-computed drawdown series (values ≤ 0), as produced
///   by [`to_drawdown_series`].
/// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
///
/// # Returns
///
/// The CDaR as a non-negative scalar (expressed as an absolute drawdown
/// depth). Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::drawdown::cdar;
///
/// let dd = [-0.01, -0.05, -0.10, -0.15, -0.20, 0.0, -0.03, -0.08, -0.12, -0.18];
/// let c = cdar(&dd, 0.80);
/// assert!(c > 0.10);
/// ```
///
/// # References
///
/// - Chekhlov, Uryasev & Zabarankin (2005): see docs/REFERENCES.md#chekhlov2005
pub fn cdar(drawdown: &[f64], confidence: f64) -> f64 {
    if drawdown.is_empty() {
        return 0.0;
    }
    let mut abs_dd: Vec<f64> = drawdown.iter().map(|&d| d.abs()).collect();
    let threshold = quantile(&mut abs_dd, confidence);
    let tail: Vec<f64> = abs_dd
        .iter()
        .filter(|&&d| d >= threshold)
        .copied()
        .collect();
    if tail.is_empty() {
        return threshold;
    }
    tail.iter().sum::<f64>() / tail.len() as f64
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use time::Month;

    fn make_dates(n: usize) -> Vec<Date> {
        (0..n)
            .map(|i| {
                Date::from_calendar_date(2025, Month::January, 1).expect("valid date")
                    + time::Duration::days(i as i64)
            })
            .collect()
    }

    #[test]
    fn drawdown_series_no_loss() {
        let r = [0.01, 0.02, 0.03];
        let dd = to_drawdown_series(&r);
        assert!(dd.iter().all(|&v| v.abs() < 1e-12));
    }

    #[test]
    fn drawdown_series_basic() {
        let r = [0.10, -0.20, 0.05, 0.10];
        let dd = to_drawdown_series(&r);
        assert_eq!(dd.len(), 4);
        assert!(dd[0].abs() < 1e-12); // no DD after gain
        assert!(dd[1] < -0.1); // DD after big loss
    }

    #[test]
    fn drawdown_details_basic() {
        let r = [0.10, -0.20, 0.05, 0.10, -0.05, -0.03];
        let dd = to_drawdown_series(&r);
        let dates = make_dates(r.len());
        let episodes = drawdown_details(&dd, &dates, 5);
        assert!(!episodes.is_empty());
        assert!(episodes[0].max_drawdown < 0.0);
    }

    #[test]
    fn avg_drawdown_empty() {
        assert_eq!(avg_drawdown(&[], &[], 5), 0.0);
    }

    #[test]
    fn cdar_hand_calc() {
        // dd = [−0.10, −0.20, −0.05, −0.15, −0.25, −0.30, −0.02, −0.08, −0.12, −0.18]
        // abs = [0.10, 0.20, 0.05, 0.15, 0.25, 0.30, 0.02, 0.08, 0.12, 0.18]
        // sorted: [0.02, 0.05, 0.08, 0.10, 0.12, 0.15, 0.18, 0.20, 0.25, 0.30]
        // quantile(0.80): h = 9*0.8 = 7.2, lo=7 (0.20), hi=8 (0.25), frac=0.2
        // threshold = 0.20 + 0.2*(0.25−0.20) = 0.21
        // Tail (abs_dd ≥ 0.21): [0.25, 0.30]
        // CDaR = (0.25 + 0.30) / 2 = 0.275
        let dd = [
            -0.10, -0.20, -0.05, -0.15, -0.25, -0.30, -0.02, -0.08, -0.12, -0.18,
        ];
        let c = cdar(&dd, 0.80);
        assert!((c - 0.275).abs() < 1e-10);
    }

    #[test]
    fn cdar_worse_than_max_drawdown_var() {
        // CDaR at any confidence ≥ the quantile threshold (it's a tail average)
        let dd = [
            -0.01, -0.05, -0.10, -0.15, -0.20, 0.0, -0.03, -0.08, -0.12, -0.18,
        ];
        let c95 = cdar(&dd, 0.95);
        let c80 = cdar(&dd, 0.80);
        // Higher confidence → fewer, more extreme tail observations → larger CDaR
        assert!(c95 >= c80);
    }

    #[test]
    fn cdar_empty() {
        assert_eq!(cdar(&[], 0.95), 0.0);
    }

    #[test]
    fn cdar_no_drawdown() {
        let dd = [0.0, 0.0, 0.0];
        assert_eq!(cdar(&dd, 0.95), 0.0);
    }

    #[test]
    fn cdar_uniform_drawdown() {
        // All drawdowns identical at −5% → CDaR = 5% regardless of confidence
        let dd = [-0.05; 20];
        let c = cdar(&dd, 0.95);
        assert!((c - 0.05).abs() < 1e-12);
    }
}
