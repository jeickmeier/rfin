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
    let total = 1.0 + super::returns::comp_total(returns);
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
    let total = 1.0 + super::returns::comp_total(returns);
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

/// Fisher-corrected sample skewness (G₁) of a return distribution.
///
/// Measures asymmetry: positive skewness indicates a longer right tail
/// (large gains more likely than large losses), negative skewness the
/// opposite. Uses the bias-corrected sample formula matching Bloomberg
/// and Excel `SKEW()`:
///
/// ```text
/// G₁ = [n / ((n-1)(n-2))] × Σ((r_i − x̄) / s)³
/// ```
///
/// where `s` is the sample standard deviation (n-1 denominator).
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
///
/// # Returns
///
/// The bias-corrected sample skewness. Returns `0.0` for fewer than 3
/// observations or zero-variance series.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::skewness;
///
/// // Symmetric distribution → skewness ≈ 0.
/// let r: Vec<f64> = (-50..=50).map(|i| i as f64 * 0.01).collect();
/// assert!(skewness(&r).abs() < 1e-10);
/// ```
///
/// # References
///
/// - Joanes & Gill (1998): see docs/REFERENCES.md#joanesGill1998
pub fn skewness(returns: &[f64]) -> f64 {
    let n = returns.len();
    if n < 3 {
        return 0.0;
    }
    let nf = n as f64;
    let m = mean(returns);
    let mut m2 = 0.0_f64;
    let mut m3 = 0.0_f64;
    for &r in returns {
        let d = r - m;
        let d2 = d * d;
        m2 += d2;
        m3 += d2 * d;
    }
    let sample_var = m2 / (nf - 1.0);
    if sample_var == 0.0 {
        return 0.0;
    }
    let s = sample_var.sqrt();
    let adj = nf / ((nf - 1.0) * (nf - 2.0));
    adj * (m3 / (s * s * s))
}

/// Fisher-corrected sample excess kurtosis (G₂) of a return distribution.
///
/// Measures tail heaviness relative to a normal distribution. A positive
/// value (leptokurtic) indicates fatter tails; negative (platykurtic)
/// indicates thinner tails. Normal returns have excess kurtosis = 0.
///
/// Uses the bias-corrected sample formula matching Bloomberg and
/// Excel `KURT()`:
///
/// ```text
/// G₂ = [n(n+1) / ((n-1)(n-2)(n-3))] × Σ((r_i − x̄) / s)⁴
///      − 3(n-1)² / ((n-2)(n-3))
/// ```
///
/// where `s` is the sample standard deviation (n-1 denominator).
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
///
/// # Returns
///
/// Bias-corrected excess kurtosis. Returns `0.0` for fewer than 4
/// observations or zero-variance series.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::kurtosis;
///
/// // Uniform distribution has negative excess kurtosis.
/// let r: Vec<f64> = (0..1000).map(|i| i as f64 / 1000.0).collect();
/// assert!(kurtosis(&r) < 0.0);
/// ```
///
/// # References
///
/// - Joanes & Gill (1998): see docs/REFERENCES.md#joanesGill1998
pub fn kurtosis(returns: &[f64]) -> f64 {
    let n = returns.len();
    if n < 4 {
        return 0.0;
    }
    let nf = n as f64;
    let m = mean(returns);
    let mut m2 = 0.0_f64;
    let mut m4 = 0.0_f64;
    for &r in returns {
        let d = r - m;
        let d2 = d * d;
        m2 += d2;
        m4 += d2 * d2;
    }
    let sample_var = m2 / (nf - 1.0);
    if sample_var == 0.0 {
        return 0.0;
    }
    let s2 = sample_var;
    let s4 = s2 * s2;
    let sum_z4 = m4 / s4;
    let a = (nf * (nf + 1.0)) / ((nf - 1.0) * (nf - 2.0) * (nf - 3.0));
    let b = (3.0 * (nf - 1.0) * (nf - 1.0)) / ((nf - 2.0) * (nf - 3.0));
    a * sum_z4 - b
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
    // Historical VaR is a non-parametric statistic: sqrt-time scaling is not
    // valid for empirical quantiles (only for parametric methods like
    // parametric_var / cornish_fisher_var). The ann_factor parameter exists
    // for API consistency across the VaR function family.
    let _ = ann_factor;
    var
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
    let n = returns.len();
    let p = 1.0 - confidence;
    let mut data: Vec<f64> = returns.to_vec();
    let var_threshold = quantile(&mut data, p);
    // After quantile(), select_nth_unstable guarantees data[0..=lo] contains
    // all elements ≤ var_threshold — compute ES directly from the partitioned
    // tail without a second pass over returns or a second Vec allocation.
    let lo = ((n - 1) as f64 * p).floor() as usize;
    let tail = &data[..=lo];
    let es = if tail.is_empty() {
        var_threshold
    } else {
        tail.iter().sum::<f64>() / tail.len() as f64
    };
    // Historical ES is a non-parametric statistic: sqrt-time scaling is not
    // valid for empirical tail means (only for parametric methods). The
    // ann_factor parameter exists for API consistency across the risk metrics.
    let _ = ann_factor;
    es
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
#[derive(Debug, Clone)]
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
    let w = window as f64;
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    for &r in &returns[..window] {
        sum += r;
        sum_sq += r * r;
    }
    let emit = |sum: f64, sum_sq: f64| -> f64 {
        let ann_mean = (sum / w) * ann_factor;
        let var = (sum_sq - sum * sum / w).max(0.0) / (w - 1.0);
        let ann_vol = var.sqrt() * ann_factor.sqrt();
        sharpe(ann_mean, ann_vol, risk_free_rate)
    };
    let mut values = Vec::with_capacity(n - window + 1);
    let mut out_dates = Vec::with_capacity(n - window + 1);
    values.push(emit(sum, sum_sq));
    out_dates.push(dates[window - 1]);
    for i in window..n {
        let add = returns[i];
        let rem = returns[i - window];
        sum += add - rem;
        sum_sq += add * add - rem * rem;
        values.push(emit(sum, sum_sq));
        out_dates.push(dates[i]);
    }
    RollingSharpe {
        values,
        dates: out_dates,
    }
}

/// Output of a rolling volatility computation.
#[derive(Debug, Clone)]
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
/// use finstack_core::analytics::risk_metrics::rolling_volatility;
/// use time::{Date, Month};
///
/// let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
/// let dates: Vec<Date> = (0..20)
///     .map(|i| Date::from_calendar_date(2025, Month::January, 1).unwrap()
///         + time::Duration::days(i))
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
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    for &r in &returns[..window] {
        sum += r;
        sum_sq += r * r;
    }
    let emit = |sum: f64, sum_sq: f64| -> f64 {
        let var = (sum_sq - sum * sum / w).max(0.0) / (w - 1.0);
        var.sqrt() * ann_factor.sqrt()
    };
    let mut values = Vec::with_capacity(n - window + 1);
    let mut out_dates = Vec::with_capacity(n - window + 1);
    values.push(emit(sum, sum_sq));
    out_dates.push(dates[window - 1]);
    for i in window..n {
        let add = returns[i];
        let rem = returns[i - window];
        sum += add - rem;
        sum_sq += add * add - rem * rem;
        values.push(emit(sum, sum_sq));
        out_dates.push(dates[i]);
    }
    RollingVolatility {
        values,
        dates: out_dates,
    }
}

// ── Batch 2: Standard ratios and VaR extensions ──

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

/// Treynor ratio: excess return per unit of systematic risk.
///
/// ```text
/// Treynor = (R_p − R_f) / β
/// ```
///
/// Complements the Sharpe ratio by using beta (systematic risk) rather
/// than total volatility as the risk denominator.
///
/// # Arguments
///
/// * `ann_return`     - Annualized portfolio return.
/// * `risk_free_rate` - Annualized risk-free rate.
/// * `beta`           - Portfolio beta vs benchmark.
///
/// # Returns
///
/// The Treynor ratio. Returns `0.0` if beta is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::treynor;
///
/// // 10% return, 2% risk-free, beta = 1.2 → Treynor ≈ 0.0667.
/// let t = treynor(0.10, 0.02, 1.2);
/// assert!((t - 0.0667).abs() < 0.001);
/// ```
///
/// # References
///
/// - Treynor (1965): see docs/REFERENCES.md#treynor1965
pub fn treynor(ann_return: f64, risk_free_rate: f64, beta: f64) -> f64 {
    if beta == 0.0 {
        return 0.0;
    }
    (ann_return - risk_free_rate) / beta
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

/// Martin ratio (Ulcer Performance Index): CAGR / Ulcer Index.
///
/// Measures return per unit of drawdown-based risk. Named after Peter
/// Martin who introduced the Ulcer Index.
///
/// # Arguments
///
/// * `cagr_val` - Compound annual growth rate.
/// * `ulcer`    - Ulcer Index (from [`ulcer_index`]).
///
/// # Returns
///
/// The Martin ratio. Returns `0.0` if the Ulcer Index is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::martin_ratio;
///
/// assert!((martin_ratio(0.10, 0.05) - 2.0).abs() < 1e-12);
/// assert_eq!(martin_ratio(0.10, 0.0), 0.0);
/// ```
///
/// # References
///
/// - Martin (1987): see docs/REFERENCES.md#martinUlcer1987
pub fn martin_ratio(cagr_val: f64, ulcer: f64) -> f64 {
    if ulcer == 0.0 {
        return 0.0;
    }
    cagr_val / ulcer
}

/// Parametric (Gaussian) Value-at-Risk.
///
/// Assumes normally distributed returns:
///
/// ```text
/// VaR = μ − z_α × σ
/// ```
///
/// where `z_α` is the standard normal quantile at `(1 - confidence)`.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
/// * `ann_factor` - If `Some(f)`, multiplies the result by `sqrt(f)`.
///
/// # Returns
///
/// The parametric VaR (typically negative). Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::parametric_var;
///
/// let r: Vec<f64> = (-100..=100).map(|i| i as f64 / 100.0).collect();
/// let pvar = parametric_var(&r, 0.95, None);
/// assert!(pvar < 0.0);
/// ```
///
/// # References
///
/// - J.P. Morgan RiskMetrics (1996): see docs/REFERENCES.md#jpmorgan1996RiskMetrics
pub fn parametric_var(returns: &[f64], confidence: f64, ann_factor: Option<f64>) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let m = mean(returns);
    let vol = variance(returns).sqrt();
    let z = crate::math::special_functions::standard_normal_inv_cdf(1.0 - confidence);
    match ann_factor {
        Some(af) => m * af + z * vol * af.sqrt(),
        None => m + z * vol,
    }
}

/// Cornish-Fisher Value-at-Risk: adjusts Gaussian VaR for skewness and kurtosis.
///
/// Uses the Cornish-Fisher expansion to produce a more accurate VaR
/// estimate when the return distribution departs from normality:
///
/// ```text
/// z_cf = z + (z² − 1)S/6 + (z³ − 3z)K/24 − (2z³ − 5z)S²/36
/// VaR_CF = μ − z_cf × σ
/// ```
///
/// where `S` is skewness and `K` is excess kurtosis.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95`.
/// * `ann_factor` - If `Some(f)`, multiplies the result by `sqrt(f)`.
///
/// # Returns
///
/// The Cornish-Fisher adjusted VaR (typically negative). Returns `0.0`
/// for an empty slice. Falls back to parametric VaR if skewness and
/// kurtosis are both zero.
///
/// # Caution
///
/// The Cornish-Fisher expansion is a polynomial approximation valid for
/// moderate departures from normality. For extreme skewness or kurtosis
/// (e.g., |S| > 2 or |K| > 6), the adjusted quantile `z_cf` can become
/// non-monotonic in confidence level, producing paradoxical results.
/// Always cross-check against historical VaR for heavily-tailed series.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::{parametric_var, cornish_fisher_var};
///
/// let r: Vec<f64> = (-100..=100).map(|i| i as f64 / 100.0).collect();
/// let pvar = parametric_var(&r, 0.95, None);
/// let cfvar = cornish_fisher_var(&r, 0.95, None);
/// // For a uniform distribution, CF-VaR differs from parametric.
/// assert!(cfvar < 0.0);
/// ```
///
/// # References
///
/// - Cornish & Fisher (1937): see docs/REFERENCES.md#cornishFisher1937
pub fn cornish_fisher_var(returns: &[f64], confidence: f64, ann_factor: Option<f64>) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let m = mean(returns);
    let vol = variance(returns).sqrt();
    let z = crate::math::special_functions::standard_normal_inv_cdf(1.0 - confidence);
    let s = skewness(returns);
    let k = kurtosis(returns);
    let z2 = z * z;
    let z3 = z2 * z;
    let z_cf =
        z + (z2 - 1.0) * s / 6.0 + (z3 - 3.0 * z) * k / 24.0 - (2.0 * z3 - 5.0 * z) * s * s / 36.0;
    match ann_factor {
        Some(af) => m * af + z_cf * vol * af.sqrt(),
        None => m + z_cf * vol,
    }
}

/// Output of a rolling Sortino ratio computation.
#[derive(Debug, Clone)]
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
/// use finstack_core::analytics::risk_metrics::rolling_sortino;
/// use time::{Date, Month};
///
/// let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
/// let dates: Vec<Date> = (0..20)
///     .map(|i| Date::from_calendar_date(2025, Month::January, 1).unwrap()
///         + time::Duration::days(i))
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
    let mut sum = 0.0_f64;
    let mut sum_ds = 0.0_f64; // Σ min(r, 0)²
    for &r in &returns[..window] {
        sum += r;
        if r < 0.0 {
            sum_ds += r * r;
        }
    }
    let emit = |sum: f64, sum_ds: f64| -> f64 {
        let m = sum / w;
        let dd = (sum_ds / w).sqrt(); // downside deviation (period)
        if dd == 0.0 {
            if m > 0.0 {
                f64::INFINITY
            } else if m < 0.0 {
                f64::NEG_INFINITY
            } else {
                0.0
            }
        } else {
            (m * ann_factor) / (dd * ann_factor.sqrt())
        }
    };
    let mut values = Vec::with_capacity(n - window + 1);
    let mut out_dates = Vec::with_capacity(n - window + 1);
    values.push(emit(sum, sum_ds));
    out_dates.push(dates[window - 1]);
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
        values.push(emit(sum, sum_ds));
        out_dates.push(dates[i]);
    }
    RollingSortino {
        values,
        dates: out_dates,
    }
}

// ── Batch 3: Drawdown-family ratios ──

/// Recovery factor: total return / |max drawdown|.
///
/// Measures how many times the portfolio has recovered its worst loss.
/// A higher value indicates greater resilience.
///
/// # Arguments
///
/// * `total_return` - Total compounded return over the period.
/// * `max_dd`       - Maximum drawdown (negative number, e.g. `−0.25`).
///
/// # Returns
///
/// The recovery factor. Returns `0.0` if `max_dd` is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::recovery_factor;
///
/// // 50% total return with 25% max drawdown → 2.0.
/// assert!((recovery_factor(0.50, -0.25) - 2.0).abs() < 1e-12);
/// ```
pub fn recovery_factor(total_return: f64, max_dd: f64) -> f64 {
    if max_dd == 0.0 {
        return 0.0;
    }
    total_return / max_dd.abs()
}

/// Sterling ratio: risk-adjusted return using average drawdown.
///
/// ```text
/// Sterling = (CAGR − R_f) / |avg_drawdown|
/// ```
///
/// # Arguments
///
/// * `cagr_val`       - Compound annual growth rate.
/// * `avg_dd`         - Average of the top-N worst drawdowns (negative number).
/// * `risk_free_rate` - Annualized risk-free rate.
///
/// # Returns
///
/// The Sterling ratio. Returns `0.0` if `avg_dd` is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::sterling_ratio;
///
/// // 12% CAGR, 2% risk-free, −10% avg drawdown → 1.0.
/// assert!((sterling_ratio(0.12, -0.10, 0.02) - 1.0).abs() < 1e-12);
/// ```
///
/// # References
///
/// - Kestner (1996): see docs/REFERENCES.md#kestner1996
pub fn sterling_ratio(cagr_val: f64, avg_dd: f64, risk_free_rate: f64) -> f64 {
    if avg_dd == 0.0 {
        return 0.0;
    }
    (cagr_val - risk_free_rate) / avg_dd.abs()
}

/// Burke ratio: return per unit of drawdown-based risk (RMS of drawdowns).
///
/// ```text
/// Burke = (CAGR − R_f) / sqrt( (1/n) Σ dd_i² )
/// ```
///
/// where `dd_i` are the max-drawdown depths of the top-N episodes.
///
/// # Arguments
///
/// * `cagr_val`       - Compound annual growth rate.
/// * `dd_episodes`    - Slice of max-drawdown values from each episode (negative).
/// * `risk_free_rate` - Annualized risk-free rate.
///
/// # Returns
///
/// The Burke ratio. Returns `0.0` if `dd_episodes` is empty or all zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::burke_ratio;
///
/// let dds = [-0.10, -0.05, -0.03];
/// let b = burke_ratio(0.15, &dds, 0.02);
/// assert!(b > 0.0);
/// ```
///
/// # References
///
/// - Burke (1994): see docs/REFERENCES.md#burke1994
pub fn burke_ratio(cagr_val: f64, dd_episodes: &[f64], risk_free_rate: f64) -> f64 {
    if dd_episodes.is_empty() {
        return 0.0;
    }
    let n = dd_episodes.len() as f64;
    let ss: f64 = dd_episodes.iter().map(|&d| d * d).sum();
    let rms = (ss / n).sqrt();
    if rms == 0.0 {
        return 0.0;
    }
    (cagr_val - risk_free_rate) / rms
}

/// Pain index: mean absolute drawdown over the full series.
///
/// ```text
/// Pain = (1/n) Σ |dd_i|
/// ```
///
/// Less sensitive to outlier drawdowns than max drawdown.
///
/// # Arguments
///
/// * `drawdown` - Pre-computed drawdown series (values ≤ 0).
///
/// # Returns
///
/// The pain index (non-negative). Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::pain_index;
///
/// let dd = [-0.05, -0.10, 0.0, -0.03];
/// let pi = pain_index(&dd);
/// assert!((pi - 0.045).abs() < 1e-12);
/// ```
pub fn pain_index(drawdown: &[f64]) -> f64 {
    if drawdown.is_empty() {
        return 0.0;
    }
    let sum: f64 = drawdown.iter().map(|&d| d.abs()).sum();
    sum / drawdown.len() as f64
}

/// Pain ratio: return per unit of average drawdown pain.
///
/// ```text
/// Pain Ratio = (CAGR − R_f) / Pain Index
/// ```
///
/// # Arguments
///
/// * `cagr_val`       - Compound annual growth rate.
/// * `pain`           - Pain index (from [`pain_index`]).
/// * `risk_free_rate` - Annualized risk-free rate.
///
/// # Returns
///
/// The pain ratio. Returns `0.0` if the pain index is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::pain_ratio;
///
/// assert!((pain_ratio(0.10, 0.05, 0.02) - 1.6).abs() < 1e-12);
/// ```
pub fn pain_ratio(cagr_val: f64, pain: f64, risk_free_rate: f64) -> f64 {
    if pain == 0.0 {
        return 0.0;
    }
    (cagr_val - risk_free_rate) / pain
}

/// M-squared (Modigliani-Modigliani): risk-adjusted return on the benchmark's scale.
///
/// Leverages or deleverages the portfolio to match the benchmark's volatility,
/// then reports the resulting return. The difference `M² − R_bench` is a
/// direct measure of value added at the same risk level.
///
/// ```text
/// M² = R_f + (R_p − R_f) × (σ_bench / σ_portfolio)
/// ```
///
/// # Arguments
///
/// * `ann_return`     - Annualized portfolio return.
/// * `ann_vol`        - Annualized portfolio volatility.
/// * `bench_vol`      - Annualized benchmark volatility.
/// * `risk_free_rate` - Annualized risk-free rate.
///
/// # Returns
///
/// The M-squared return. Returns the risk-free rate if portfolio volatility is zero.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::risk_metrics::m_squared;
///
/// // Portfolio: 12% return, 20% vol; Benchmark: 15% vol; Rf: 2%
/// // M² = 0.02 + (0.12 − 0.02) × (0.15 / 0.20) = 0.02 + 0.075 = 0.095
/// let m2 = m_squared(0.12, 0.20, 0.15, 0.02);
/// assert!((m2 - 0.095).abs() < 1e-12);
/// ```
///
/// # References
///
/// - Modigliani & Modigliani (1997): see docs/REFERENCES.md#modigliani1997
pub fn m_squared(ann_return: f64, ann_vol: f64, bench_vol: f64, risk_free_rate: f64) -> f64 {
    if ann_vol == 0.0 {
        return risk_free_rate;
    }
    risk_free_rate + (ann_return - risk_free_rate) * (bench_vol / ann_vol)
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

    // ── Batch 1: Higher moments, downside deviation, geometric mean ──

    #[test]
    fn downside_deviation_hand_calc() {
        // r = [0.01, -0.02, 0.03, -0.01, 0.005], MAR = 0.0
        // Below MAR: -0.02, -0.01
        // Sum of squared: 0.0004 + 0.0001 = 0.0005
        // DD = sqrt(0.0005 / 5) = sqrt(0.0001) = 0.01
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
        // r = [0.01, 0.02, 0.03, 0.005], MAR = 0.02
        // Below MAR: 0.01, 0.005 → deviations: -0.01, -0.015
        // Sum sq: 0.0001 + 0.000225 = 0.000325
        // DD = sqrt(0.000325 / 4) = sqrt(0.00008125) ≈ 0.009013878
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
    fn skewness_symmetric_zero() {
        let r: Vec<f64> = (-50..=50).map(|i| i as f64 * 0.01).collect();
        assert!(skewness(&r).abs() < 1e-10);
    }

    #[test]
    fn skewness_hand_calc() {
        // r = [0, 0, 0, 0, 5], n = 5, mean = 1.0
        // d = [-1, -1, -1, -1, 4], m2 = 20, sample_var = 5, s = sqrt(5)
        // m3 = 60, adj = 5/(4*3) = 5/12
        // G₁ = (5/12) * 60 / (sqrt(5))³ = sqrt(5)
        let r = [0.0, 0.0, 0.0, 0.0, 5.0];
        assert!((skewness(&r) - 5.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn skewness_negative() {
        // Mirror of the above → −sqrt(5)
        let r = [0.0, 0.0, 0.0, 0.0, -5.0];
        assert!((skewness(&r) - (-5.0_f64.sqrt())).abs() < 1e-12);
    }

    #[test]
    fn skewness_edge_cases() {
        assert_eq!(skewness(&[]), 0.0);
        assert_eq!(skewness(&[1.0, 2.0]), 0.0); // n < 3
        assert_eq!(skewness(&[3.0, 3.0, 3.0]), 0.0); // zero variance
    }

    #[test]
    fn kurtosis_hand_calc() {
        // r = [0, 0, 0, 0, 5], n = 5, mean = 1.0
        // d = [-1, -1, -1, -1, 4], m2 = 20, sample_var = 5, s4 = 25
        // m4 = 260, sum_z4 = 10.4
        // a = (5*6)/(4*3*2) = 1.25, b = 3*16/(3*2) = 8.0
        // G₂ = 1.25 * 10.4 − 8.0 = 5.0
        let r = [0.0, 0.0, 0.0, 0.0, 5.0];
        assert!((kurtosis(&r) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn kurtosis_uniform_analytical() {
        // Continuous uniform on [a,b]: excess kurtosis = −6/5 = −1.2
        // Fisher-corrected for large n converges to population value.
        // Discrete uniform on {0,...,n-1}: population k = −6(n²+1)/(5(n²−1))
        // Fisher correction inflates slightly for finite n.
        let r: Vec<f64> = (0..10000).map(|i| i as f64 / 10000.0).collect();
        let k = kurtosis(&r);
        assert!((k - (-1.2)).abs() < 0.005);
    }

    #[test]
    fn kurtosis_edge_cases() {
        assert_eq!(kurtosis(&[]), 0.0);
        assert_eq!(kurtosis(&[1.0, 2.0, 3.0]), 0.0); // n < 4
        assert_eq!(kurtosis(&[1.0, 1.0, 1.0, 1.0]), 0.0); // zero variance
    }

    #[test]
    fn geometric_mean_constant() {
        let gm = geometric_mean(&[0.05, 0.05, 0.05]);
        assert!((gm - 0.05).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_volatility_drag_exact() {
        // (1.10)(0.90) = 0.99 → geo = sqrt(0.99) − 1 ≈ −0.005013
        let gm = geometric_mean(&[0.10, -0.10]);
        let expected = 0.99_f64.sqrt() - 1.0;
        assert!((gm - expected).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_known_product() {
        // (1.20)(1.10)(0.95) = 1.254 → geo = 1.254^(1/3) − 1
        let gm = geometric_mean(&[0.20, 0.10, -0.05]);
        let expected = 1.254_f64.powf(1.0 / 3.0) - 1.0;
        assert!((gm - expected).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_empty() {
        assert_eq!(geometric_mean(&[]), 0.0);
    }

    #[test]
    fn geometric_mean_less_than_arithmetic() {
        // AM-GM inequality: geometric mean <= arithmetic mean (strict for non-constant)
        let r = [0.05, 0.10, -0.03, 0.08];
        let gm = geometric_mean(&r);
        let am = mean(&r);
        assert!(gm < am);
    }

    #[test]
    fn rolling_volatility_window_count() {
        let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
        let dates: Vec<Date> = (0..20)
            .map(|i| jan1(2025) + time::Duration::days(i))
            .collect();
        let rv = rolling_volatility(&returns, &dates, 5, 252.0);
        assert_eq!(rv.values.len(), 16); // 20 − 5 + 1
        assert_eq!(rv.dates.len(), 16);
        assert!(rv.values.iter().all(|&v| v > 0.0));
    }

    #[test]
    fn rolling_volatility_matches_pointwise() {
        // First window [0..5] should match standalone volatility
        let returns: Vec<f64> = (0..10).map(|i| (i as f64 - 5.0) * 0.01).collect();
        let dates: Vec<Date> = (0..10)
            .map(|i| jan1(2025) + time::Duration::days(i))
            .collect();
        let rv = rolling_volatility(&returns, &dates, 5, 252.0);
        let first_window = volatility(&returns[0..5], true, 252.0);
        assert!((rv.values[0] - first_window).abs() < 1e-12);
    }

    #[test]
    fn rolling_volatility_empty_window() {
        let rv = rolling_volatility(&[0.01], &[jan1(2025)], 5, 252.0);
        assert!(rv.values.is_empty());
    }

    // ── Batch 2: Standard ratios and VaR extensions ──

    #[test]
    fn omega_ratio_hand_calc() {
        // r = [0.05, −0.02, 0.03, −0.01, 0.04], threshold = 0
        // Gains above 0: 0.05 + 0.03 + 0.04 = 0.12
        // Losses below 0: 0.02 + 0.01 = 0.03
        // Omega = 0.12 / 0.03 = 4.0
        let r = [0.05, -0.02, 0.03, -0.01, 0.04];
        let omega = omega_ratio(&r, 0.0);
        assert!((omega - 4.0).abs() < 1e-12);
    }

    #[test]
    fn omega_ratio_with_threshold() {
        // Same returns, threshold = 0.02
        // Gains above 0.02: (0.05−0.02) + (0.03−0.02) + (0.04−0.02) = 0.03+0.01+0.02 = 0.06
        // Losses below 0.02: (0.02−(−0.02)) + (0.02−(−0.01)) = 0.04 + 0.03 = 0.07
        // Omega = 0.06 / 0.07 = 6/7
        let r = [0.05, -0.02, 0.03, -0.01, 0.04];
        let omega = omega_ratio(&r, 0.02);
        assert!((omega - 6.0 / 7.0).abs() < 1e-12);
    }

    #[test]
    fn omega_ratio_no_losses() {
        assert_eq!(omega_ratio(&[0.01, 0.02, 0.03], 0.0), f64::INFINITY);
    }

    #[test]
    fn omega_ratio_no_gains() {
        // All below threshold → gains=0, losses>0 → omega = 0
        let omega = omega_ratio(&[-0.01, -0.02, -0.03], 0.0);
        assert!((omega - 0.0).abs() < 1e-14);
    }

    #[test]
    fn omega_ratio_empty() {
        assert_eq!(omega_ratio(&[], 0.0), 0.0);
    }

    #[test]
    fn treynor_hand_calc() {
        // (0.10 − 0.02) / 1.2 = 0.08/1.2 = 1/15 ≈ 0.06667
        let t = treynor(0.10, 0.02, 1.2);
        assert!((t - 0.08 / 1.2).abs() < 1e-14);
    }

    #[test]
    fn treynor_zero_beta() {
        assert_eq!(treynor(0.10, 0.02, 0.0), 0.0);
    }

    #[test]
    fn treynor_negative_beta() {
        // Negative beta inverts the ratio → negative Treynor for positive excess return
        let t = treynor(0.10, 0.02, -0.5);
        assert!((t - (0.08 / -0.5)).abs() < 1e-14);
    }

    #[test]
    fn gain_to_pain_hand_calc() {
        // r = [0.05, −0.02, 0.03, −0.01, 0.04]
        // Total = 0.09, |losses| = 0.03
        // GtP = 0.09 / 0.03 = 3.0
        let r = [0.05, -0.02, 0.03, -0.01, 0.04];
        let gtp = gain_to_pain(&r);
        assert!((gtp - 3.0).abs() < 1e-12);
    }

    #[test]
    fn gain_to_pain_negative_total() {
        // Total return negative: GtP < 0
        let gtp = gain_to_pain(&[-0.05, 0.01, -0.04]);
        // Total = -0.08, |losses| = 0.09, GtP = -0.08/0.09 ≈ -0.8889
        assert!((gtp - (-0.08 / 0.09)).abs() < 1e-12);
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
    fn martin_ratio_hand_calc() {
        assert!((martin_ratio(0.10, 0.05) - 2.0).abs() < 1e-12);
        assert_eq!(martin_ratio(0.10, 0.0), 0.0);
    }

    #[test]
    fn parametric_var_formula_verification() {
        // Verify VaR = μ + Φ⁻¹(1−α) × σ directly
        let r = [-0.02, -0.01, 0.0, 0.01, 0.02];
        let pvar = parametric_var(&r, 0.95, None);
        let m = mean(&r);
        let vol = variance(&r).sqrt();
        let z = crate::math::special_functions::standard_normal_inv_cdf(0.05);
        let expected = m + z * vol;
        assert!((pvar - expected).abs() < 1e-14);
    }

    #[test]
    fn parametric_var_annualized() {
        let r = [-0.02, -0.01, 0.0, 0.01, 0.02];
        let pvar_raw = parametric_var(&r, 0.95, None);
        let pvar_ann = parametric_var(&r, 0.95, Some(252.0));
        assert!((pvar_ann - pvar_raw * 252.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn parametric_var_empty() {
        assert_eq!(parametric_var(&[], 0.95, None), 0.0);
    }

    #[test]
    fn cornish_fisher_var_converges_to_parametric_for_normal_like() {
        // For a symmetric distribution (skew≈0, kurtosis≈0), CF-VaR ≈ parametric
        let r: Vec<f64> = (-500..=500).map(|i| i as f64 / 5000.0).collect();
        let pvar = parametric_var(&r, 0.95, None);
        let cfvar = cornish_fisher_var(&r, 0.95, None);
        // Uniform has kurtosis ≈ −1.2, so there will be some difference,
        // but for a wide uniform the difference is bounded
        assert!((cfvar - pvar).abs() < 0.02);
    }

    #[test]
    fn cornish_fisher_var_differs_from_parametric_for_skewed() {
        // Right-skewed distribution: CF-VaR should differ meaningfully
        let mut r = vec![-0.01_f64; 80];
        r.extend(vec![0.05; 20]);
        let pvar = parametric_var(&r, 0.95, None);
        let cfvar = cornish_fisher_var(&r, 0.95, None);
        // Both should be negative (losses)
        assert!(pvar < 0.0);
        assert!(cfvar < 0.0);
        // Should differ for skewed data
        assert!((cfvar - pvar).abs() > 1e-6);
    }

    #[test]
    fn cornish_fisher_var_empty() {
        assert_eq!(cornish_fisher_var(&[], 0.95, None), 0.0);
    }

    #[test]
    fn rolling_sortino_window_count() {
        let returns: Vec<f64> = (0..20).map(|i| (i as f64 - 10.0) * 0.001).collect();
        let dates: Vec<Date> = (0..20)
            .map(|i| jan1(2025) + time::Duration::days(i))
            .collect();
        let rs = rolling_sortino(&returns, &dates, 5, 252.0);
        assert_eq!(rs.values.len(), 16); // 20 − 5 + 1
        assert_eq!(rs.dates.len(), 16);
    }

    #[test]
    fn rolling_sortino_matches_pointwise() {
        let returns: Vec<f64> = (0..10).map(|i| (i as f64 - 5.0) * 0.01).collect();
        let dates: Vec<Date> = (0..10)
            .map(|i| jan1(2025) + time::Duration::days(i))
            .collect();
        let rs = rolling_sortino(&returns, &dates, 5, 252.0);
        let first_window = sortino(&returns[0..5], true, 252.0);
        assert!((rs.values[0] - first_window).abs() < 1e-12);
    }

    // ── Batch 3: Drawdown-family ratios ──

    #[test]
    fn recovery_factor_hand_calc() {
        // 50% total return, −25% max DD → 0.50 / 0.25 = 2.0
        assert!((recovery_factor(0.50, -0.25) - 2.0).abs() < 1e-12);
        // Negative total return: −0.10 / |−0.30| = −0.10 / 0.30 = −1/3
        assert!((recovery_factor(-0.10, -0.30) - (-1.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn recovery_factor_zero_dd() {
        assert_eq!(recovery_factor(0.50, 0.0), 0.0);
    }

    #[test]
    fn sterling_ratio_hand_calc() {
        // CAGR=0.12, Rf=0.02, avg_dd=−0.10 → (0.12−0.02)/0.10 = 1.0
        assert!((sterling_ratio(0.12, -0.10, 0.02) - 1.0).abs() < 1e-12);
        // CAGR=0.15, Rf=0.03, avg_dd=−0.06 → (0.15−0.03)/0.06 = 2.0
        assert!((sterling_ratio(0.15, -0.06, 0.03) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn sterling_ratio_zero_dd() {
        assert_eq!(sterling_ratio(0.12, 0.0, 0.02), 0.0);
    }

    #[test]
    fn burke_ratio_hand_calc() {
        // CAGR=0.12, Rf=0.02, dds=[−0.10, −0.10]
        // ss = 0.01 + 0.01 = 0.02, rms = sqrt(0.02/2) = sqrt(0.01) = 0.1
        // Burke = (0.12−0.02) / 0.1 = 1.0
        let dds = [-0.10, -0.10];
        let b = burke_ratio(0.12, &dds, 0.02);
        assert!((b - 1.0).abs() < 1e-12);
    }

    #[test]
    fn burke_ratio_single_drawdown() {
        // CAGR=0.20, Rf=0.0, dds=[−0.05]
        // rms = 0.05, Burke = 0.20/0.05 = 4.0
        assert!((burke_ratio(0.20, &[-0.05], 0.0) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn burke_ratio_empty() {
        assert_eq!(burke_ratio(0.15, &[], 0.02), 0.0);
    }

    #[test]
    fn pain_index_hand_calc() {
        // dd = [−0.05, −0.10, 0.0, −0.03]
        // mean(|dd|) = (0.05 + 0.10 + 0.0 + 0.03) / 4 = 0.18/4 = 0.045
        let dd = [-0.05, -0.10, 0.0, -0.03];
        let pi = pain_index(&dd);
        assert!((pi - 0.045).abs() < 1e-14);
    }

    #[test]
    fn pain_index_constant_drawdown() {
        // All at −5% → pain = 0.05
        let dd = [-0.05, -0.05, -0.05];
        assert!((pain_index(&dd) - 0.05).abs() < 1e-14);
    }

    #[test]
    fn pain_index_empty() {
        assert_eq!(pain_index(&[]), 0.0);
    }

    #[test]
    fn pain_ratio_hand_calc() {
        // CAGR=0.10, pain=0.05, Rf=0.02 → (0.10−0.02)/0.05 = 1.6
        assert!((pain_ratio(0.10, 0.05, 0.02) - 1.6).abs() < 1e-12);
        // CAGR=0.08, pain=0.04, Rf=0.0 → 0.08/0.04 = 2.0
        assert!((pain_ratio(0.08, 0.04, 0.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn pain_ratio_zero_pain() {
        assert_eq!(pain_ratio(0.10, 0.0, 0.0), 0.0);
    }

    // ── Cross-validation and invariant tests ──

    #[test]
    fn omega_monotonic_in_threshold() {
        // Lower threshold → higher omega (more gains count, fewer losses)
        let r = [0.05, -0.02, 0.03, -0.01, 0.04, -0.03, 0.02];
        let o_low = omega_ratio(&r, -0.01);
        let o_mid = omega_ratio(&r, 0.0);
        let o_high = omega_ratio(&r, 0.02);
        assert!(o_low > o_mid);
        assert!(o_mid > o_high);
    }

    #[test]
    fn calmar_consistent_with_components() {
        // Calmar = CAGR / |max_dd|
        let cagr_val = 0.15;
        let max_dd = -0.30;
        let c = calmar(cagr_val, max_dd);
        assert!((c - cagr_val / max_dd.abs()).abs() < 1e-14);
    }

    #[test]
    fn pain_ratio_equals_cagr_over_pain_when_rf_zero() {
        let cagr_val = 0.12;
        let pain_val = 0.04;
        let pr = pain_ratio(cagr_val, pain_val, 0.0);
        assert!((pr - cagr_val / pain_val).abs() < 1e-14);
    }

    #[test]
    fn sterling_consistent_with_components() {
        let cagr_val = 0.15;
        let avg_dd = -0.08;
        let rf = 0.03;
        let sr = sterling_ratio(cagr_val, avg_dd, rf);
        assert!((sr - (cagr_val - rf) / avg_dd.abs()).abs() < 1e-14);
    }

    #[test]
    fn es_is_worse_than_var_always() {
        // ES ≤ VaR for any return distribution (coherent risk axiom)
        let datasets: Vec<Vec<f64>> = vec![
            (-100..=100).map(|i| i as f64 / 100.0).collect(),
            vec![0.05, -0.10, 0.03, -0.15, 0.07, -0.02, 0.01, -0.08],
            vec![-0.01; 50],
        ];
        for data in &datasets {
            let var = value_at_risk(data, 0.95, None);
            let es = expected_shortfall(data, 0.95, None);
            assert!(es <= var + 1e-14, "ES must be ≤ VaR: es={es}, var={var}");
        }
    }

    #[test]
    fn var_es_parametric_ordering() {
        // For the same data, ES ≤ VaR ≤ 0 for loss-generating portfolios
        let r = [
            0.05, -0.10, 0.03, -0.15, 0.07, -0.02, 0.01, -0.08, -0.05, 0.02,
        ];
        let var = value_at_risk(&r, 0.95, None);
        let es = expected_shortfall(&r, 0.95, None);
        let pvar = parametric_var(&r, 0.95, None);
        assert!(var < 0.0);
        assert!(es <= var);
        assert!(pvar < 0.0);
    }

    #[test]
    fn cornish_fisher_formula_verification() {
        // Verify z_cf = z + (z²−1)S/6 + (z³−3z)K/24 − (2z³−5z)S²/36
        let r = [
            0.05, -0.10, 0.03, -0.15, 0.07, -0.02, 0.01, -0.08, -0.05, 0.02,
        ];
        let cf = cornish_fisher_var(&r, 0.95, None);
        let m = mean(&r);
        let vol = variance(&r).sqrt();
        let s = skewness(&r);
        let k = kurtosis(&r);
        let z = crate::math::special_functions::standard_normal_inv_cdf(0.05);
        let z2 = z * z;
        let z3 = z2 * z;
        let z_cf = z + (z2 - 1.0) * s / 6.0 + (z3 - 3.0 * z) * k / 24.0
            - (2.0 * z3 - 5.0 * z) * s * s / 36.0;
        let expected = m + z_cf * vol;
        assert!((cf - expected).abs() < 1e-14);
    }

    // ── M-squared and Modified Sharpe ──

    #[test]
    fn m_squared_hand_calc() {
        // M² = Rf + (Rp − Rf) × (σ_bench / σ_port)
        // = 0.02 + (0.12 − 0.02) × (0.15 / 0.20) = 0.02 + 0.075 = 0.095
        let m2 = m_squared(0.12, 0.20, 0.15, 0.02);
        assert!((m2 - 0.095).abs() < 1e-12);
    }

    #[test]
    fn m_squared_zero_vol() {
        assert_eq!(m_squared(0.10, 0.0, 0.15, 0.02), 0.02);
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
    fn historical_var_does_not_apply_sqrt_time_scaling() {
        let returns = [-0.03, -0.02, -0.01, 0.01, 0.02];
        let period_var = value_at_risk(&returns, 0.95, None);
        let annualized_var = value_at_risk(&returns, 0.95, Some(252.0));

        assert!(
            (annualized_var - period_var).abs() < 1e-14,
            "historical VaR should remain a period statistic: {annualized_var} vs {period_var}"
        );
    }

    #[test]
    fn historical_es_does_not_apply_sqrt_time_scaling() {
        let returns = [-0.03, -0.02, -0.01, 0.01, 0.02];
        let period_es = expected_shortfall(&returns, 0.95, None);
        let annualized_es = expected_shortfall(&returns, 0.95, Some(252.0));

        assert!(
            (annualized_es - period_es).abs() < 1e-14,
            "historical ES should remain a period statistic: {annualized_es} vs {period_es}"
        );
    }

    #[test]
    fn parametric_var_scales_mean_and_vol_by_horizon() {
        let returns = [0.01, -0.02, 0.03, -0.01, 0.02, -0.005];
        let ann_factor = 12.0;
        let m = mean(&returns);
        let vol = variance(&returns).sqrt();
        let z = crate::math::special_functions::standard_normal_inv_cdf(0.05);
        let expected = m * ann_factor + z * vol * ann_factor.sqrt();

        let actual = parametric_var(&returns, 0.95, Some(ann_factor));
        assert!((actual - expected).abs() < 1e-14, "{actual} vs {expected}");
    }

    #[test]
    fn cornish_fisher_var_scales_mean_and_vol_by_horizon() {
        let returns = [
            0.05, -0.10, 0.03, -0.15, 0.07, -0.02, 0.01, -0.08, -0.05, 0.02,
        ];
        let ann_factor = 12.0;
        let m = mean(&returns);
        let vol = variance(&returns).sqrt();
        let s = skewness(&returns);
        let k = kurtosis(&returns);
        let z = crate::math::special_functions::standard_normal_inv_cdf(0.05);
        let z2 = z * z;
        let z3 = z2 * z;
        let z_cf = z + (z2 - 1.0) * s / 6.0 + (z3 - 3.0 * z) * k / 24.0
            - (2.0 * z3 - 5.0 * z) * s * s / 36.0;
        let expected = m * ann_factor + z_cf * vol * ann_factor.sqrt();

        let actual = cornish_fisher_var(&returns, 0.95, Some(ann_factor));
        assert!((actual - expected).abs() < 1e-14, "{actual} vs {expected}");
    }
}
