//! Risk and return metrics: Sharpe, Sortino, Calmar, VaR, ES, and more.
//!
//! All functions operate on `&[f64]` return slices and return scalar `f64`.
//! Annualization uses the caller-supplied factor (typically from
//! `PeriodKind::annualization_factor()`).

use crate::dates::Date;
use crate::math::stats::{mean, quantile, variance};
use crate::math::summation::kahan_sum;

/// Compound annual growth rate from a return series over a date range.
///
/// Uses `Act/365 Fixed` day-count for the year fraction.
pub fn cagr(returns: &[f64], start: Date, end: Date) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let total = returns.iter().fold(1.0_f64, |acc, &r| acc * (1.0 + r));
    let days = (end - start).whole_days() as f64;
    if days <= 0.0 {
        return 0.0;
    }
    let years = days / 365.0;
    total.powf(1.0 / years) - 1.0
}

/// Mean return, optionally annualized.
pub fn mean_return(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    let m = mean(returns);
    if annualize {
        m * ann_factor
    } else {
        m
    }
}

/// Volatility (standard deviation of returns), optionally annualized.
///
/// Uses population variance (consistent with the Python source).
pub fn volatility(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    let v = variance(returns).sqrt();
    if annualize {
        v * ann_factor.sqrt()
    } else {
        v
    }
}

/// Sharpe ratio = annualized_return / annualized_volatility.
pub fn sharpe(ann_return: f64, ann_vol: f64) -> f64 {
    if ann_vol == 0.0 {
        return 0.0;
    }
    ann_return / ann_vol
}

/// Sortino ratio: penalises only downside volatility.
pub fn sortino(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    let m = mean(returns);
    let downside: Vec<f64> = returns
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|&r| r * r)
        .collect();
    if downside.is_empty() {
        return 0.0;
    }
    let downside_dev = (kahan_sum(downside.iter().copied()) / returns.len() as f64).sqrt();
    if downside_dev == 0.0 {
        return 0.0;
    }
    if annualize {
        (m * ann_factor) / (downside_dev * ann_factor.sqrt())
    } else {
        m / downside_dev
    }
}

/// Calmar ratio = CAGR / |max drawdown|.
pub fn calmar(cagr_val: f64, max_dd: f64) -> f64 {
    if max_dd == 0.0 {
        return 0.0;
    }
    cagr_val / max_dd.abs()
}

/// Ulcer index: RMS of the drawdown series.
pub fn ulcer_index(drawdown: &[f64]) -> f64 {
    if drawdown.is_empty() {
        return 0.0;
    }
    let ss: f64 = drawdown.iter().map(|&d| d * d).sum();
    (ss / drawdown.len() as f64).sqrt()
}

/// Risk of ruin (simplified closed-form approximation).
///
/// `exp(-2 * mean / var)` where var is the variance.
pub fn risk_of_ruin(mean_ret: f64, vol: f64) -> f64 {
    if vol == 0.0 {
        return 0.0;
    }
    let var = vol * vol;
    (-2.0 * mean_ret / var).exp().min(1.0)
}

/// Historical Value-at-Risk at the given confidence level (e.g., 0.95).
///
/// Returns a **negative** number representing the loss threshold.
/// Optionally annualized by multiplying by `sqrt(ann_factor)`.
pub fn value_at_risk(returns: &[f64], confidence: f64, ann_factor: Option<f64>) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mut data: Vec<f64> = returns.to_vec();
    let var = quantile(&mut data, 1.0 - confidence);
    match ann_factor {
        Some(af) => var * af.sqrt(),
        None => var,
    }
}

/// Expected shortfall (CVaR) at the given confidence level.
///
/// Mean of all returns below the VaR threshold.
pub fn expected_shortfall(returns: &[f64], confidence: f64, ann_factor: Option<f64>) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mut data: Vec<f64> = returns.to_vec();
    let var_threshold = quantile(&mut data, 1.0 - confidence);
    let tail: Vec<f64> = returns
        .iter()
        .filter(|&&r| r <= var_threshold)
        .copied()
        .collect();
    if tail.is_empty() {
        return var_threshold;
    }
    let es = mean(&tail);
    match ann_factor {
        Some(af) => es * af.sqrt(),
        None => es,
    }
}

/// Tail ratio = |95th percentile| / |5th percentile|.
pub fn tail_ratio(returns: &[f64], confidence: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mut data: Vec<f64> = returns.to_vec();
    let upper = quantile(&mut data, confidence).abs();
    let lower = quantile(&mut data, 1.0 - confidence).abs();
    if lower == 0.0 {
        return 0.0;
    }
    upper / lower
}

/// Fraction of returns above the upper quantile threshold (outlier wins).
pub fn outlier_win_ratio(returns: &[f64], confidence: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mut data: Vec<f64> = returns.to_vec();
    let threshold = quantile(&mut data, confidence);
    let count = returns.iter().filter(|&&r| r > threshold).count();
    count as f64 / returns.len() as f64
}

/// Fraction of returns below the lower quantile threshold (outlier losses).
pub fn outlier_loss_ratio(returns: &[f64], confidence: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mut data: Vec<f64> = returns.to_vec();
    let threshold = quantile(&mut data, 1.0 - confidence);
    let count = returns.iter().filter(|&&r| r < threshold).count();
    count as f64 / returns.len() as f64
}

/// Rolling Sharpe output.
pub struct RollingSharpe {
    /// Rolling Sharpe ratio values.
    pub values: Vec<f64>,
    /// End dates for each rolling window.
    pub dates: Vec<Date>,
}

/// Rolling Sharpe ratio over a sliding window.
pub fn rolling_sharpe(
    returns: &[f64],
    dates: &[Date],
    window: usize,
    ann_factor: f64,
) -> RollingSharpe {
    let n = returns.len().min(dates.len());
    if n < window || window == 0 {
        return RollingSharpe {
            values: vec![],
            dates: vec![],
        };
    }
    let mut values = Vec::with_capacity(n - window + 1);
    let mut out_dates = Vec::with_capacity(n - window + 1);
    for i in window..=n {
        let slice = &returns[i - window..i];
        let m = mean_return(slice, true, ann_factor);
        let v = volatility(slice, true, ann_factor);
        values.push(sharpe(m, v));
        out_dates.push(dates[i - 1]);
    }
    RollingSharpe {
        values,
        dates: out_dates,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use time::Month;

    fn jan1(year: i32) -> Date {
        Date::from_calendar_date(year, Month::January, 1).expect("valid date")
    }

    #[test]
    fn cagr_basic() {
        // 10% total return over 1 year
        let r = [0.10];
        let c = cagr(&r, jan1(2024), jan1(2025));
        assert!((c - 0.10).abs() < 0.01);
    }

    #[test]
    fn sharpe_basic() {
        assert!((sharpe(0.10, 0.15) - 0.6666).abs() < 0.01);
        assert_eq!(sharpe(0.10, 0.0), 0.0);
    }

    #[test]
    fn var_basic() {
        let mut data: Vec<f64> = (-100..=100).map(|i| i as f64 / 100.0).collect();
        let var = value_at_risk(&data, 0.95, None);
        assert!(var < -0.8);
        data.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in test data"));
    }

    #[test]
    fn es_is_worse_than_var() {
        let data: Vec<f64> = (-100..=100).map(|i| i as f64 / 100.0).collect();
        let var = value_at_risk(&data, 0.95, None);
        let es = expected_shortfall(&data, 0.95, None);
        assert!(es <= var);
    }

    #[test]
    fn sortino_positive_returns() {
        let r = [0.01, 0.02, 0.03, -0.005, 0.01];
        let s = sortino(&r, false, 252.0);
        assert!(s > 0.0);
    }

    #[test]
    fn rolling_sharpe_window() {
        let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
        let dates: Vec<Date> = (0..20)
            .map(|i| jan1(2025) + time::Duration::days(i))
            .collect();
        let rs = rolling_sharpe(&returns, &dates, 5, 252.0);
        assert_eq!(rs.values.len(), 16);
    }
}
