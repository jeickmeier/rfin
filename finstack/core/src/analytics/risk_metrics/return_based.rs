//! Return-based risk metrics: mean, volatility, Sharpe, Sortino, CAGR, and more.
//!
//! All functions operate on `&[f64]` return slices and return scalar `f64`.
//! Annualization uses the caller-supplied factor (typically from
//! `PeriodKind::annualization_factor()`).

use crate::math::stats::{mean, variance};
use crate::math::summation::kahan_sum;

use super::tail_risk::cornish_fisher_var;

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
pub fn cagr(returns: &[f64], start: crate::dates::Date, end: crate::dates::Date) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let total = 1.0 + crate::analytics::returns::comp_total(returns);
    let days = (end - start).whole_days() as f64;
    if days <= 0.0 {
        return 0.0;
    }
    let years = days / 365.0;
    total.powf(1.0 / years) - 1.0
}

/// Compound annual growth rate from a return series using a period-based
/// annualization factor (e.g., 252 for daily, 12 for monthly).
///
/// Unlike [`cagr`], which requires start/end dates, this variant derives
/// the holding period from `returns.len() / ann_factor`.
///
/// Returns `f64::NAN` when `returns` has fewer than 2 elements.
pub fn cagr_from_periods(returns: &[f64], ann_factor: f64) -> f64 {
    let n = returns.len();
    if n < 2 {
        return f64::NAN;
    }
    let total = 1.0 + crate::analytics::returns::comp_total(returns);
    let years = n as f64 / ann_factor;
    if years > 0.0 {
        total.powf(1.0 / years) - 1.0
    } else {
        f64::NAN
    }
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
/// Uses **sample** standard deviation (n-1 denominator), consistent with
/// Bloomberg, QuantLib, and the `OnlineStats::variance()` convention.
/// Annualizes by multiplying by `sqrt(ann_factor)` following the
/// square-root-of-time rule.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `annualize`  - Whether to scale by `sqrt(ann_factor)`.
/// * `ann_factor` - Number of periods per year (e.g., 252 daily, 12 monthly).
///
/// # Returns
///
/// Sample standard deviation of `returns` (n-1 denominator), annualized if requested.
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

/// Downside deviation: semi-standard deviation below a minimum acceptable return.
///
/// Computes the root-mean-square of returns falling below `mar`, using
/// the full series length as the denominator (population convention),
/// consistent with Sortino & van der Meer (1991):
///
/// ```text
/// DD = sqrt( (1/n) × Σ min(r_i − MAR, 0)² )
/// ```
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `mar`        - Minimum acceptable return (threshold). Use `0.0` for
///   the standard Sortino definition.
/// * `annualize`  - Whether to scale by `sqrt(ann_factor)`.
/// * `ann_factor` - Number of periods per year.
///
/// # Returns
///
/// The downside deviation (non-negative). Returns `0.0` for an empty
/// slice or when no returns fall below `mar`.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::downside_deviation;
///
/// let r = [0.01, -0.02, 0.03, -0.01, 0.005];
/// let dd = downside_deviation(&r, 0.0, false, 252.0);
/// assert!(dd > 0.0);
///
/// // All returns above MAR → zero downside deviation.
/// let dd_pos = downside_deviation(&[0.01, 0.02, 0.03], 0.0, false, 252.0);
/// assert_eq!(dd_pos, 0.0);
/// ```
///
/// # References
///
/// - Sortino & van der Meer (1991): see docs/REFERENCES.md#sortinoVanDerMeer1991
pub fn downside_deviation(returns: &[f64], mar: f64, annualize: bool, ann_factor: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let downside_sq = kahan_sum(returns.iter().filter(|&&r| r < mar).map(|&r| {
        let d = r - mar;
        d * d
    }));
    let dd = (downside_sq / returns.len() as f64).sqrt();
    if annualize {
        dd * ann_factor.sqrt()
    } else {
        dd
    }
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
    let dd = downside_deviation(returns, 0.0, false, ann_factor);
    if dd == 0.0 {
        return if m > 0.0 {
            f64::INFINITY
        } else if m < 0.0 {
            f64::NEG_INFINITY
        } else {
            0.0
        };
    }
    if annualize {
        (m * ann_factor) / (dd * ann_factor.sqrt())
    } else {
        m / dd
    }
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
/// Probability of ruin in `[0, 1]`. When `vol` is zero (deterministic
/// returns), returns `0.0` if `mean_ret > 0.0` (guaranteed no ruin) or
/// `1.0` if `mean_ret <= 0.0` (ruin is certain). Clamped to `1.0` from above.
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
        return if mean_ret > 0.0 { 0.0 } else { 1.0 };
    }
    let var = vol * vol;
    (-2.0 * mean_ret / var).exp().min(1.0)
}

/// Risk of ruin computed directly from a returns series.
pub fn risk_of_ruin_from_returns(returns: &[f64]) -> f64 {
    let mean_ret = mean(returns);
    let vol = variance(returns).sqrt();
    risk_of_ruin(mean_ret, vol)
}

/// Geometric mean return per period.
///
/// The compound-average return: the constant per-period return that
/// would produce the same terminal wealth as the actual series.
///
/// ```text
/// geo_mean = (Π(1 + r_i))^(1/n) − 1
/// ```
///
/// Computed in log-space with Kahan summation for numerical stability.
/// Growth factors are clamped to `1e-18` for returns ≤ −100%.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
///
/// # Returns
///
/// The geometric mean return. Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::geometric_mean;
///
/// // +10% then −10%: geometric mean < 0 (volatility drag).
/// let gm = geometric_mean(&[0.10, -0.10]);
/// assert!(gm < 0.0);
///
/// // Constant 5% → geometric mean = 5%.
/// let gm5 = geometric_mean(&[0.05, 0.05, 0.05]);
/// assert!((gm5 - 0.05).abs() < 1e-12);
/// ```
pub fn geometric_mean(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let n = returns.len() as f64;
    let log_sum = kahan_sum(returns.iter().map(|&r| (1.0 + r).max(1e-18).ln()));
    (log_sum / n).exp() - 1.0
}

/// Omega ratio: probability-weighted gain-to-loss ratio above a threshold.
///
/// ```text
/// Ω(L) = Σ max(r_i − L, 0) / Σ max(L − r_i, 0)
/// ```
///
/// Unlike the Sharpe ratio (which uses only mean and variance), the Omega
/// ratio incorporates the full return distribution.
///
/// # Arguments
///
/// * `returns`   - Slice of period simple returns.
/// * `threshold` - Return threshold (typically `0.0`).
///
/// # Returns
///
/// The Omega ratio. Returns `f64::INFINITY` if no returns fall below the
/// threshold, and `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::omega_ratio;
///
/// let r = [0.05, -0.02, 0.03, -0.01, 0.04];
/// let omega = omega_ratio(&r, 0.0);
/// assert!(omega > 1.0);
/// ```
///
/// # References
///
/// - Keating & Shadwick (2002): see docs/REFERENCES.md#keatingShadwick2002
pub fn omega_ratio(returns: &[f64], threshold: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let mut gains = 0.0_f64;
    let mut losses = 0.0_f64;
    for &r in returns {
        if r > threshold {
            gains += r - threshold;
        } else {
            losses += threshold - r;
        }
    }
    if losses == 0.0 {
        return if gains > 0.0 { f64::INFINITY } else { 0.0 };
    }
    gains / losses
}

/// Gain-to-pain ratio: total return divided by total losses.
///
/// ```text
/// GtP = Σ r_i / Σ |r_i|   for r_i < 0
/// ```
///
/// Popular among CTA and systematic macro managers as a simple
/// measure of return efficiency relative to the pain of drawdowns.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
///
/// # Returns
///
/// The gain-to-pain ratio. Returns `f64::INFINITY` when total return is
/// positive but there are no losses, `0.0` for an empty slice or zero
/// net return with no losses.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::gain_to_pain;
///
/// let r = [0.05, -0.02, 0.03, -0.01, 0.04];
/// let gtp = gain_to_pain(&r);
/// assert!(gtp > 0.0);
/// ```
///
/// # References
///
/// - Schwager (2012): see docs/REFERENCES.md#schwager2012
pub fn gain_to_pain(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let total: f64 = kahan_sum(returns.iter().copied());
    let abs_losses: f64 = kahan_sum(returns.iter().filter(|&&r| r < 0.0).map(|&r| r.abs()));
    if abs_losses == 0.0 {
        return if total > 0.0 { f64::INFINITY } else { 0.0 };
    }
    total / abs_losses
}

/// Modified Sharpe ratio: excess return divided by Cornish-Fisher VaR.
///
/// Replaces the standard deviation in the Sharpe denominator with the
/// Cornish-Fisher adjusted VaR, accounting for skewness and kurtosis:
///
/// ```text
/// Modified Sharpe = (R_p − R_f) / |CF-VaR|
/// ```
///
/// # Arguments
///
/// * `returns`        - Slice of period simple returns.
/// * `risk_free_rate` - Annualized risk-free rate.
/// * `confidence`     - VaR confidence level (e.g., `0.95`).
/// * `ann_factor`     - Number of periods per year.
///
/// # Returns
///
/// The Modified Sharpe ratio. Returns `0.0` for empty slices or when
/// CF-VaR is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::modified_sharpe;
///
/// let r = [0.01, -0.02, 0.03, -0.01, 0.02, 0.005, -0.005, 0.015];
/// let ms = modified_sharpe(&r, 0.02, 0.95, 252.0);
/// assert!(ms.is_finite());
/// ```
///
/// # References
///
/// - Gregoriou & Gueyie (2003): see docs/REFERENCES.md#gregoriou2003
pub fn modified_sharpe(
    returns: &[f64],
    risk_free_rate: f64,
    confidence: f64,
    ann_factor: f64,
) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let ann_ret = mean_return(returns, true, ann_factor);
    let cf_var = cornish_fisher_var(returns, confidence, Some(ann_factor));
    if cf_var == 0.0 {
        return 0.0;
    }
    (ann_ret - risk_free_rate) / cf_var.abs()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::math::stats::{mean, variance};
    use time::Month;

    fn jan1(year: i32) -> crate::dates::Date {
        crate::dates::Date::from_calendar_date(year, Month::January, 1).expect("valid date")
    }

    #[test]
    fn cagr_basic() {
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
        assert!((sharpe(0.10, 0.15, 0.02) - 0.5333).abs() < 0.01);
    }

    #[test]
    fn sortino_positive_returns() {
        let r = [0.01, 0.02, 0.03, -0.005, 0.01];
        let s = sortino(&r, false, 252.0);
        assert!(s > 0.0);
    }

    #[test]
    fn downside_deviation_hand_calc() {
        let r = [0.01, -0.02, 0.03, -0.01, 0.005];
        let dd = downside_deviation(&r, 0.0, false, 252.0);
        assert!((dd - 0.01).abs() < 1e-14);
    }

    #[test]
    fn downside_deviation_annualized() {
        let r = [0.01, -0.02, 0.03, -0.01, 0.005];
        let dd_raw = downside_deviation(&r, 0.0, false, 252.0);
        let dd_ann = downside_deviation(&r, 0.0, true, 252.0);
        assert!((dd_ann - dd_raw * 252.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn downside_deviation_all_positive() {
        let dd = downside_deviation(&[0.01, 0.02, 0.03], 0.0, false, 252.0);
        assert_eq!(dd, 0.0);
    }

    #[test]
    fn downside_deviation_empty() {
        assert_eq!(downside_deviation(&[], 0.0, false, 252.0), 0.0);
    }

    #[test]
    fn downside_deviation_with_mar() {
        let r = [0.01, 0.02, 0.03, 0.005];
        let dd = downside_deviation(&r, 0.02, false, 252.0);
        let expected = (0.000325_f64 / 4.0).sqrt();
        assert!((dd - expected).abs() < 1e-14);
    }

    #[test]
    fn sortino_consistent_with_downside_deviation() {
        let r = [0.01, 0.02, 0.03, -0.005, 0.01];
        let m = mean(&r);
        let dd = downside_deviation(&r, 0.0, false, 252.0);
        let s = sortino(&r, false, 252.0);
        assert!((s - m / dd).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_constant() {
        let gm = geometric_mean(&[0.05, 0.05, 0.05]);
        assert!((gm - 0.05).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_volatility_drag_exact() {
        let gm = geometric_mean(&[0.10, -0.10]);
        let expected = 0.99_f64.sqrt() - 1.0;
        assert!((gm - expected).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_empty() {
        assert_eq!(geometric_mean(&[]), 0.0);
    }

    #[test]
    fn geometric_mean_less_than_arithmetic() {
        let r = [0.05, 0.10, -0.03, 0.08];
        let gm = geometric_mean(&r);
        let am = mean(&r);
        assert!(gm < am);
    }

    #[test]
    fn omega_ratio_hand_calc() {
        let r = [0.05, -0.02, 0.03, -0.01, 0.04];
        let omega = omega_ratio(&r, 0.0);
        assert!((omega - 4.0).abs() < 1e-12);
    }

    #[test]
    fn omega_ratio_no_losses() {
        assert_eq!(omega_ratio(&[0.01, 0.02, 0.03], 0.0), f64::INFINITY);
    }

    #[test]
    fn omega_ratio_empty() {
        assert_eq!(omega_ratio(&[], 0.0), 0.0);
    }

    #[test]
    fn gain_to_pain_hand_calc() {
        let r = [0.05, -0.02, 0.03, -0.01, 0.04];
        let gtp = gain_to_pain(&r);
        assert!((gtp - 3.0).abs() < 1e-12);
    }

    #[test]
    fn gain_to_pain_no_losses() {
        assert_eq!(gain_to_pain(&[0.01, 0.02]), f64::INFINITY);
    }

    #[test]
    fn gain_to_pain_empty() {
        assert_eq!(gain_to_pain(&[]), 0.0);
    }

    #[test]
    fn risk_of_ruin_from_returns_matches_composed_formula() {
        let returns = [0.01, -0.02, 0.015, -0.005, 0.012, 0.008];
        let ann = 252.0;
        let mean_ret = mean_return(&returns, false, ann);
        let vol = volatility(&returns, false, ann);
        let expected = risk_of_ruin(mean_ret, vol);
        let actual = risk_of_ruin_from_returns(&returns);
        assert!((actual - expected).abs() < 1e-12);
    }

    #[test]
    fn modified_sharpe_finite() {
        let r = [0.01, -0.02, 0.03, -0.01, 0.02, 0.005, -0.005, 0.015];
        let ms = modified_sharpe(&r, 0.02, 0.95, 252.0);
        assert!(ms.is_finite());
    }

    #[test]
    fn modified_sharpe_empty() {
        assert_eq!(modified_sharpe(&[], 0.02, 0.95, 252.0), 0.0);
    }

    #[test]
    fn parametric_var_scales_mean_and_vol_by_horizon() {
        let returns = [0.01, -0.02, 0.03, -0.01, 0.02, -0.005];
        let ann_factor = 12.0;
        let m = mean(&returns);
        let vol = variance(&returns).sqrt();
        let z = crate::math::special_functions::standard_normal_inv_cdf(0.05);
        let expected = m * ann_factor + z * vol * ann_factor.sqrt();
        let actual =
            crate::analytics::risk_metrics::parametric_var(&returns, 0.95, Some(ann_factor));
        assert!((actual - expected).abs() < 1e-14, "{actual} vs {expected}");
    }
}
