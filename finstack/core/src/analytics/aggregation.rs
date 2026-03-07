//! Period aggregation: group returns by period and compute period-level stats.
//!
//! Uses `dates::periods::PeriodId` as grouping keys and `DateExt` for
//! date-to-period mapping.

use crate::dates::{Date, DateExt, FiscalConfig, PeriodId, PeriodKind};

use super::returns::comp_total;

/// Period-level aggregate statistics.
#[derive(Debug, Clone)]
pub struct PeriodStats {
    /// Best single-period return.
    pub best: f64,
    /// Worst single-period return.
    pub worst: f64,
    /// Longest streak of positive-return periods.
    pub consecutive_wins: usize,
    /// Longest streak of negative-return periods.
    pub consecutive_losses: usize,
    /// Fraction of periods with positive returns.
    pub win_rate: f64,
    /// Mean return across all periods.
    pub avg_return: f64,
    /// Mean of positive-return periods.
    pub avg_win: f64,
    /// Mean of negative-return periods.
    pub avg_loss: f64,
    /// avg_win / |avg_loss|.
    pub payoff_ratio: f64,
    /// Sum of wins / sum of |losses|.
    pub profit_ratio: f64,
    /// Same as `profit_ratio` (alias).
    pub profit_factor: f64,
    /// CPC index (Common Sense Ratio): profit_factor × win_rate × payoff_ratio.
    pub cpc_ratio: f64,
    /// Kelly criterion: win_rate − loss_rate / payoff_ratio.
    pub kelly_criterion: f64,
}

/// Map a date to its `PeriodId` for the requested frequency.
fn date_to_period_id(
    date: Date,
    freq: PeriodKind,
    fiscal_config: Option<FiscalConfig>,
) -> PeriodId {
    let (year, month, _day) = date.to_calendar_date();
    match freq {
        PeriodKind::Daily => PeriodId::day(year, date.ordinal()),
        PeriodKind::Weekly => {
            let ordinal = date.ordinal();
            let week = ((ordinal - 1) / 7 + 1).min(53);
            PeriodId::week(year, week as u8)
        }
        PeriodKind::Monthly => PeriodId::month(year, month as u8),
        PeriodKind::Quarterly => {
            let q = date.quarter();
            PeriodId::quarter(year, q)
        }
        PeriodKind::SemiAnnual => {
            let h = if (month as u8) <= 6 { 1 } else { 2 };
            PeriodId::half(year, h)
        }
        PeriodKind::Annual => match fiscal_config {
            Some(config) => {
                let fy = date.fiscal_year(config);
                PeriodId::annual(fy)
            }
            None => PeriodId::annual(year),
        },
    }
}

/// Group daily returns by period, compounding within each period.
///
/// Assigns each observation to a [`PeriodId`] bucket determined by `freq`
/// and `fiscal_config`, then compounds the intra-period returns using
/// [`comp_total`]. The result is a time-ordered sequence of
/// `(period_id, compounded_return)` pairs suitable for period-level
/// statistics.
///
/// # Arguments
///
/// * `dates` - Sorted slice of observation dates.
/// * `returns` - Return series aligned with `dates`. If longer, excess
///   elements are ignored.
/// * `freq` - Aggregation frequency (e.g., `Monthly`, `Annual`).
/// * `fiscal_config` - Fiscal year configuration, required when `freq` is
///   `Annual` and a non-calendar fiscal year is desired.
///
/// # Returns
///
/// A `Vec<(PeriodId, f64)>` in chronological order. Returns an empty
/// vector if either `dates` or `returns` is empty.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::aggregation::group_by_period;
/// use finstack_core::dates::{PeriodId, PeriodKind};
/// use time::{Date, Month};
///
/// let dates = vec![
///     Date::from_calendar_date(2025, Month::January, 2).unwrap(),
///     Date::from_calendar_date(2025, Month::January, 3).unwrap(),
///     Date::from_calendar_date(2025, Month::February, 3).unwrap(),
/// ];
/// let returns = vec![0.01, 0.02, -0.01];
/// let grouped = group_by_period(&dates, &returns, PeriodKind::Monthly, None);
/// assert_eq!(grouped.len(), 2);
/// assert_eq!(grouped[0].0, PeriodId::month(2025, 1));
/// ```
pub fn group_by_period(
    dates: &[Date],
    returns: &[f64],
    freq: PeriodKind,
    fiscal_config: Option<FiscalConfig>,
) -> Vec<(PeriodId, f64)> {
    let n = dates.len().min(returns.len());
    if n == 0 {
        return vec![];
    }

    let mut result: Vec<(PeriodId, f64)> = Vec::new();
    let mut current_pid = date_to_period_id(dates[0], freq, fiscal_config);
    let mut period_returns: Vec<f64> = Vec::new();

    for i in 0..n {
        let pid = date_to_period_id(dates[i], freq, fiscal_config);
        if pid != current_pid {
            let comp = comp_total(&period_returns);
            result.push((current_pid, comp));
            period_returns.clear();
            current_pid = pid;
        }
        period_returns.push(returns[i]);
    }
    if !period_returns.is_empty() {
        let comp = comp_total(&period_returns);
        result.push((current_pid, comp));
    }

    result
}

/// Compute period-level statistics from grouped returns.
///
/// Derives a comprehensive set of trading statistics from a sequence of
/// per-period compounded returns, including win rate, payoff ratio, Kelly
/// criterion, and consecutive streak lengths.
///
/// # Arguments
///
/// * `grouped` - Slice of `(PeriodId, compounded_return)` pairs, typically
///   produced by [`group_by_period`]. The `PeriodId` values are not used
///   in the computation; only the returns matter.
///
/// # Returns
///
/// A [`PeriodStats`] struct. If `grouped` is empty, all fields are `0.0` / `0`.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::aggregation::period_stats;
/// use finstack_core::dates::PeriodId;
///
/// let grouped = vec![
///     (PeriodId::month(2025, 1),  0.05),
///     (PeriodId::month(2025, 2), -0.02),
///     (PeriodId::month(2025, 3),  0.03),
///     (PeriodId::month(2025, 4),  0.01),
/// ];
/// let stats = period_stats(&grouped);
/// assert!((stats.best  - 0.05).abs() < 1e-12);
/// assert!((stats.worst - (-0.02)).abs() < 1e-12);
/// assert!((stats.win_rate - 0.75).abs() < 1e-12);
/// ```
pub fn period_stats(grouped: &[(PeriodId, f64)]) -> PeriodStats {
    if grouped.is_empty() {
        return PeriodStats {
            best: 0.0,
            worst: 0.0,
            consecutive_wins: 0,
            consecutive_losses: 0,
            win_rate: 0.0,
            avg_return: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            payoff_ratio: 0.0,
            profit_ratio: 0.0,
            profit_factor: 0.0,
            cpc_ratio: 0.0,
            kelly_criterion: 0.0,
        };
    }

    // Single pass: compute all stats without intermediate allocations.
    let mut best = f64::NEG_INFINITY;
    let mut worst = f64::INFINITY;
    let mut total_sum = 0.0_f64;
    let mut win_sum = 0.0_f64;
    let mut loss_sum = 0.0_f64;
    let mut win_count = 0usize;
    let mut loss_count = 0usize;
    // Consecutive streak tracking — computed inline to avoid a second pass.
    let mut cur_win_streak = 0usize;
    let mut cur_loss_streak = 0usize;
    let mut consecutive_wins = 0usize;
    let mut consecutive_losses = 0usize;

    for &(_, r) in grouped {
        if r > best {
            best = r;
        }
        if r < worst {
            worst = r;
        }
        total_sum += r;
        if r > 0.0 {
            win_sum += r;
            win_count += 1;
            cur_win_streak += 1;
            if cur_win_streak > consecutive_wins {
                consecutive_wins = cur_win_streak;
            }
            cur_loss_streak = 0;
        } else if r < 0.0 {
            loss_sum += r;
            loss_count += 1;
            cur_loss_streak += 1;
            if cur_loss_streak > consecutive_losses {
                consecutive_losses = cur_loss_streak;
            }
            cur_win_streak = 0;
        } else {
            cur_win_streak = 0;
            cur_loss_streak = 0;
        }
    }

    let total = grouped.len();
    let win_rate = win_count as f64 / total as f64;
    let avg_return = total_sum / total as f64;
    let avg_win = if win_count == 0 {
        0.0
    } else {
        win_sum / win_count as f64
    };
    let avg_loss = if loss_count == 0 {
        0.0
    } else {
        loss_sum / loss_count as f64
    };

    let payoff_ratio = if avg_loss == 0.0 {
        0.0
    } else {
        avg_win / avg_loss.abs()
    };

    let total_profit = win_sum;
    let total_loss = loss_sum.abs();

    let profit_ratio = if total_loss == 0.0 {
        if total_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        total_profit / total_loss
    };

    let profit_factor = profit_ratio;

    let cpc_ratio = profit_factor * win_rate * payoff_ratio;

    let loss_rate = 1.0 - win_rate;
    let kelly_criterion = if payoff_ratio == 0.0 {
        0.0
    } else {
        win_rate - loss_rate / payoff_ratio
    };

    PeriodStats {
        best,
        worst,
        consecutive_wins,
        consecutive_losses,
        win_rate,
        avg_return,
        avg_win,
        avg_loss,
        payoff_ratio,
        profit_ratio,
        profit_factor,
        cpc_ratio,
        kelly_criterion,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use time::Month;

    fn d(y: i32, m: u8, day: u8) -> Date {
        crate::dates::create_date(y, Month::try_from(m).expect("valid month"), day)
            .expect("valid date")
    }

    #[test]
    fn group_by_monthly() {
        let dates = vec![d(2025, 1, 2), d(2025, 1, 3), d(2025, 2, 3), d(2025, 2, 4)];
        let returns = vec![0.01, 0.02, -0.01, 0.03];
        let grouped = group_by_period(&dates, &returns, PeriodKind::Monthly, None);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].0, PeriodId::month(2025, 1));
        assert_eq!(grouped[1].0, PeriodId::month(2025, 2));
    }

    #[test]
    fn period_stats_basic() {
        let grouped = vec![
            (PeriodId::month(2025, 1), 0.05),
            (PeriodId::month(2025, 2), -0.02),
            (PeriodId::month(2025, 3), 0.03),
            (PeriodId::month(2025, 4), 0.01),
        ];
        let stats = period_stats(&grouped);
        assert!((stats.best - 0.05).abs() < 1e-12);
        assert!((stats.worst - (-0.02)).abs() < 1e-12);
        assert!((stats.win_rate - 0.75).abs() < 1e-12);
    }

    #[test]
    fn period_stats_empty() {
        let stats = period_stats(&[]);
        assert_eq!(stats.win_rate, 0.0);
    }
}
