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
/// Computes:
///
/// ```text
/// CAGR = (Π(1 + r_i))^(1/years) - 1
/// ```
///
/// where `years = (end - start).days / 365` using an Act/365 Fixed
/// day-count convention.
///
/// # Arguments
///
/// * `returns` - Slice of simple period returns.
/// * `start`   - Start date of the series (inclusive).
/// * `end`     - End date of the series (inclusive).
///
/// # Returns
///
/// Annualized growth rate as a decimal. Returns `0.0` if `returns` is
/// empty or if the date range covers zero or negative days.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::cagr;
/// use time::{Date, Month};
///
/// let start = Date::from_calendar_date(2024, Month::January, 1).unwrap();
/// let end   = Date::from_calendar_date(2025, Month::January, 1).unwrap();
/// // Single 10% return over one year → CAGR ≈ 10%.
/// let c = cagr(&[0.10], start, end);
/// assert!((c - 0.10).abs() < 0.01);
/// ```
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
///
/// Computes the arithmetic mean of `returns`. When `annualize` is `true`
/// the result is scaled by `ann_factor` (e.g., 252 for daily data).
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `annualize`  - Whether to multiply the mean by `ann_factor`.
/// * `ann_factor` - Number of periods per year (e.g., 252 daily, 12 monthly).
///
/// # Returns
///
/// Arithmetic mean return, annualized if requested. Returns `0.0` for an
/// empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::mean_return;
///
/// let r = [0.01, 0.02, 0.03];
/// let m = mean_return(&r, false, 252.0);
/// assert!((m - 0.02).abs() < 1e-12);
///
/// let m_ann = mean_return(&r, true, 252.0);
/// assert!((m_ann - 0.02 * 252.0).abs() < 1e-10);
/// ```
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
/// Uses **population** standard deviation (divides by `n`, not `n-1`),
/// consistent with the Python reference implementation. Annualizes by
/// multiplying by `sqrt(ann_factor)` following the square-root-of-time rule.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `annualize`  - Whether to scale by `sqrt(ann_factor)`.
/// * `ann_factor` - Number of periods per year (e.g., 252 daily, 12 monthly).
///
/// # Returns
///
/// Population standard deviation of `returns`, annualized if requested.
/// Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::volatility;
///
/// let r = [0.01, -0.01, 0.02, -0.02];
/// let vol = volatility(&r, false, 252.0);
/// assert!(vol > 0.0);
///
/// let vol_ann = volatility(&r, true, 252.0);
/// assert!((vol_ann - vol * 252.0_f64.sqrt()).abs() < 1e-12);
/// ```
pub fn volatility(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    let v = variance(returns).sqrt();
    if annualize {
        v * ann_factor.sqrt()
    } else {
        v
    }
}

/// Sharpe ratio = (annualized return − risk-free rate) / annualized volatility.
///
/// Measures risk-adjusted return relative to total (upside + downside)
/// volatility. A higher value indicates better risk-adjusted performance.
///
/// # Arguments
///
/// * `ann_return`     - Annualized portfolio return.
/// * `ann_vol`        - Annualized portfolio volatility.
/// * `risk_free_rate` - Annualized risk-free rate (e.g., `0.02` for 2%).
///
/// # Returns
///
/// The Sharpe ratio. Returns `0.0` if `ann_vol` is zero (constant returns).
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::sharpe;
///
/// assert!((sharpe(0.10, 0.15, 0.0) - 0.6667).abs() < 0.001);
/// // Zero volatility → zero Sharpe.
/// assert_eq!(sharpe(0.10, 0.0, 0.0), 0.0);
/// ```
///
/// # References
///
/// - Sharpe (1966): see docs/REFERENCES.md#sharpe1966
pub fn sharpe(ann_return: f64, ann_vol: f64, risk_free_rate: f64) -> f64 {
    if ann_vol == 0.0 {
        return 0.0;
    }
    (ann_return - risk_free_rate) / ann_vol
}

/// Sortino ratio: penalises only downside volatility.
///
/// Unlike the Sharpe ratio, the Sortino ratio uses the **downside deviation**
/// (semi-standard deviation of negative returns) as the risk denominator,
/// leaving upside volatility unrewarded:
///
/// ```text
/// Sortino = (annualized mean return) / (annualized downside deviation)
/// ```
///
/// Downside deviation is computed over the full return series (denominator
/// is `n`, not the number of negative observations), consistent with the
/// Sortino & van der Meer (1991) definition.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns. Minimum acceptable return
///   is implicitly `0.0`.
/// * `annualize` - Whether to annualize both numerator and denominator.
/// * `ann_factor` - Number of periods per year.
///
/// # Returns
///
/// The Sortino ratio. Returns `±∞` when the mean is nonzero but there
/// are no negative returns (zero downside risk), and `0.0` when the
/// mean is zero or the downside deviation is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::sortino;
///
/// let r = [0.01, 0.02, 0.03, -0.005, 0.01];
/// let s = sortino(&r, false, 252.0);
/// assert!(s > 0.0);
/// ```
///
/// # References
///
/// - Sortino & van der Meer (1991): see docs/REFERENCES.md#sortinoVanDerMeer1991
pub fn sortino(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    let m = mean(returns);
    let downside: Vec<f64> = returns
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|&r| r * r)
        .collect();
    if downside.is_empty() {
        return if m > 0.0 {
            f64::INFINITY
        } else if m < 0.0 {
            f64::NEG_INFINITY
        } else {
            0.0
        };
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
///
/// Compares annualized growth against the worst peak-to-trough loss,
/// making it particularly useful for evaluating trend-following strategies.
///
/// # Arguments
///
/// * `cagr_val` - Compound annual growth rate (already computed).
/// * `max_dd` - Maximum drawdown depth (a negative number, e.g., `-0.25`
///   for a 25% drawdown).
///
/// # Returns
///
/// The Calmar ratio (positive when CAGR and max drawdown have the same sign).
/// Returns `0.0` if `max_dd` is zero (no drawdown observed).
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::calmar;
///
/// // 15% CAGR with 30% max drawdown → Calmar ≈ 0.5
/// assert!((calmar(0.15, -0.30) - 0.5).abs() < 1e-12);
/// ```
///
/// # References
///
/// - Young (1991): see docs/REFERENCES.md#youngCalmar1991
pub fn calmar(cagr_val: f64, max_dd: f64) -> f64 {
    if max_dd == 0.0 {
        return 0.0;
    }
    cagr_val / max_dd.abs()
}

/// Ulcer index: root-mean-square of the drawdown series.
///
/// Measures the depth and duration of drawdowns from a pre-computed
/// drawdown series. A higher Ulcer Index indicates more persistent or
/// deeper losses ("investor distress").
///
/// ```text
/// UI = sqrt(mean(dd_i^2))
/// ```
///
/// # Arguments
///
/// * `drawdown` - Pre-computed drawdown series (values ≤ 0), as produced
///   by [`crate::analytics::drawdown::to_drawdown_series`].
///
/// # Returns
///
/// The Ulcer Index (a non-negative scalar). Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::ulcer_index;
///
/// // Flat drawdown of −10% throughout → UI = 0.10.
/// let dd = [-0.10, -0.10, -0.10];
/// assert!((ulcer_index(&dd) - 0.10).abs() < 1e-12);
/// ```
///
/// # References
///
/// - Martin (1987): see docs/REFERENCES.md#martinUlcer1987
pub fn ulcer_index(drawdown: &[f64]) -> f64 {
    if drawdown.is_empty() {
        return 0.0;
    }
    let ss: f64 = drawdown.iter().map(|&d| d * d).sum();
    (ss / drawdown.len() as f64).sqrt()
}

/// Risk of ruin: probability of total loss under a simplified normal model.
///
/// Uses the closed-form approximation:
///
/// ```text
/// P(ruin) = exp(-2 × μ / σ²)
/// ```
///
/// where `μ` is the mean return and `σ²` is the return variance. This is a
/// rough heuristic; it assumes normally distributed returns and a fixed
/// ruin threshold of zero.
///
/// # Arguments
///
/// * `mean_ret` - Period mean return.
/// * `vol`      - Period standard deviation of returns (σ).
///
/// # Returns
///
/// Probability of ruin in `[0, 1]`. Returns `0.0` if `vol` is zero
/// (deterministic returns cannot ruin). Clamped to `1.0` from above.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::risk_of_ruin;
///
/// // Highly positive mean relative to vol → very low ruin probability.
/// let p = risk_of_ruin(0.10, 0.05);
/// assert!(p < 0.02);
///
/// // Mean = 0 → ruin probability = 1.
/// let p_zero = risk_of_ruin(0.0, 0.10);
/// assert!((p_zero - 1.0).abs() < 1e-12);
/// ```
pub fn risk_of_ruin(mean_ret: f64, vol: f64) -> f64 {
    if vol == 0.0 {
        return 0.0;
    }
    let var = vol * vol;
    (-2.0 * mean_ret / var).exp().min(1.0)
}

/// Historical Value-at-Risk at the given confidence level.
///
/// Computes the `(1 - confidence)` quantile of the empirical return
/// distribution. For example, at `confidence = 0.95`, the 5th percentile
/// is returned — losses exceeding this threshold occur with 5% probability
/// under the historical distribution.
///
/// Returns a **negative** number representing the loss threshold.
/// Optionally scaled by `sqrt(ann_factor)` for horizon adjustment.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
/// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95` for 95% VaR.
/// * `ann_factor` - If `Some(f)`, multiplies the result by `sqrt(f)` to
///   scale to an annual horizon.
///
/// # Returns
///
/// The VaR as a non-positive scalar. Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::value_at_risk;
///
/// let data: Vec<f64> = (-100..=100).map(|i| i as f64 / 100.0).collect();
/// let var95 = value_at_risk(&data, 0.95, None);
/// assert!(var95 < -0.8);
/// ```
///
/// # References
///
/// - J.P. Morgan RiskMetrics (1996): see docs/REFERENCES.md#jpmorgan1996RiskMetrics
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

/// Expected Shortfall (CVaR / ES) at the given confidence level.
///
/// The mean of all returns that fall at or below the VaR threshold,
/// providing a coherent measure of tail risk:
///
/// ```text
/// ES_α = E[r | r ≤ VaR_α]
/// ```
///
/// ES is always at least as bad (negative) as VaR at the same confidence
/// level, and satisfies the sub-additivity axiom of coherent risk measures.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
/// * `ann_factor` - If `Some(f)`, multiplies the result by `sqrt(f)`.
///
/// # Returns
///
/// The Expected Shortfall as a non-positive scalar. Returns `0.0` for
/// an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::{value_at_risk, expected_shortfall};
///
/// let data: Vec<f64> = (-100..=100).map(|i| i as f64 / 100.0).collect();
/// let var = value_at_risk(&data, 0.95, None);
/// let es  = expected_shortfall(&data, 0.95, None);
/// // ES must be at least as bad as VaR.
/// assert!(es <= var);
/// ```
///
/// # References
///
/// - Artzner et al. (1999): see docs/REFERENCES.md#artzner1999CoherentRisk
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

/// Tail ratio = |upper tail| / |lower tail|.
///
/// Computes the ratio of the absolute upper quantile to the absolute lower
/// quantile:
///
/// ```text
/// tail_ratio = |quantile(confidence)| / |quantile(1 - confidence)|
/// ```
///
/// A value greater than 1.0 indicates that the right tail (gains) is larger
/// than the left tail (losses) at the symmetric confidence level.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
/// * `confidence` - Quantile level for the upper tail (e.g., `0.95`).
///   The lower tail uses `1 - confidence`.
///
/// # Returns
///
/// The tail ratio (non-negative). Returns `0.0` if `returns` is empty
/// or if the lower tail quantile is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::tail_ratio;
///
/// // Symmetric distribution → tail ratio ≈ 1.
/// let r: Vec<f64> = (-50..=50).map(|i| i as f64 * 0.01).collect();
/// let tr = tail_ratio(&r, 0.95);
/// assert!((tr - 1.0).abs() < 0.1);
/// ```
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
///
/// Counts how many returns exceed the `confidence` quantile of the
/// distribution and expresses that as a fraction of the total. Identifies
/// how often a strategy generates outsized positive returns.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `confidence` - Upper quantile threshold (e.g., `0.95`).
///
/// # Returns
///
/// Fraction of returns strictly above the threshold, in `[0, 1)`.
/// Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::outlier_win_ratio;
///
/// let r: Vec<f64> = (0..100).map(|i| i as f64 * 0.001).collect();
/// let ratio = outlier_win_ratio(&r, 0.95);
/// assert!(ratio < 0.06);
/// ```
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
///
/// Counts how many returns fall strictly below the `(1 - confidence)`
/// quantile and expresses that as a fraction of the total. Identifies how
/// often a strategy suffers outsized negative returns.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
/// * `confidence` - Upper confidence level; the lower tail is `1 - confidence`
///   (e.g., `0.95` checks below the 5th percentile).
///
/// # Returns
///
/// Fraction of returns strictly below the lower threshold, in `[0, 1)`.
/// Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::outlier_loss_ratio;
///
/// let r: Vec<f64> = (0..100).map(|i| i as f64 * 0.001).collect();
/// let ratio = outlier_loss_ratio(&r, 0.95);
/// assert!(ratio < 0.06);
/// ```
pub fn outlier_loss_ratio(returns: &[f64], confidence: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mut data: Vec<f64> = returns.to_vec();
    let threshold = quantile(&mut data, 1.0 - confidence);
    let count = returns.iter().filter(|&&r| r < threshold).count();
    count as f64 / returns.len() as f64
}

/// Output of a rolling Sharpe ratio computation.
///
/// Contains parallel vectors of Sharpe values and their corresponding
/// window-end dates, suitable for time-series charting.
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
/// use finstack_core::analytics::risk_metrics::rolling_sharpe;
/// use time::{Date, Month};
///
/// let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
/// let dates: Vec<Date> = (0..20)
///     .map(|i| Date::from_calendar_date(2025, Month::January, 1).unwrap()
///         + time::Duration::days(i))
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
    let mut values = Vec::with_capacity(n - window + 1);
    let mut out_dates = Vec::with_capacity(n - window + 1);
    for i in window..=n {
        let slice = &returns[i - window..i];
        let m = mean_return(slice, true, ann_factor);
        let v = volatility(slice, true, ann_factor);
        values.push(sharpe(m, v, risk_free_rate));
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
        assert!((sharpe(0.10, 0.15, 0.0) - 0.6666).abs() < 0.01);
        assert_eq!(sharpe(0.10, 0.0, 0.0), 0.0);
    }

    #[test]
    fn sharpe_with_risk_free_rate() {
        // (0.10 - 0.02) / 0.15 ≈ 0.5333
        assert!((sharpe(0.10, 0.15, 0.02) - 0.5333).abs() < 0.01);
    }

    #[test]
    fn var_basic() {
        let data: Vec<f64> = (-100..=100).map(|i| i as f64 / 100.0).collect();
        let var = value_at_risk(&data, 0.95, None);
        assert!(var < -0.8);
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
        let rs = rolling_sharpe(&returns, &dates, 5, 252.0, 0.0);
        assert_eq!(rs.values.len(), 16);
    }
}
