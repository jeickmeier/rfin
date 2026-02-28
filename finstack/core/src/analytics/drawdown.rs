//! Drawdown computation: series, episode detection, and averaging.

use crate::dates::Date;

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
    /// Maximum drawdown depth (negative fraction).
    pub max_drawdown: f64,
    /// 99% of the max drawdown depth.
    pub max_drawdown_99: f64,
}

/// Compute a drawdown series from a simple-return series.
///
/// `dd[i]` = current drawdown depth at time `i`, expressed as a non-positive
/// fraction (e.g., -0.10 = 10% drawdown).
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
/// Returns the top `n` worst episodes sorted by max drawdown (most negative first).
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
        max_drawdown_99: valley_val * 0.99,
    }
}

/// Average of the top-N worst drawdowns.
pub fn avg_drawdown(drawdown: &[f64], dates: &[Date], n: usize) -> f64 {
    let episodes = drawdown_details(drawdown, dates, n);
    if episodes.is_empty() {
        return 0.0;
    }
    let sum: f64 = episodes.iter().map(|e| e.max_drawdown).sum();
    sum / episodes.len() as f64
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
}
