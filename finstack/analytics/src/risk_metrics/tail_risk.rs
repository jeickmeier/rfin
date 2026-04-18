//! Tail-risk and distribution-shape metrics: VaR, ES, skewness, kurtosis.
//!
//! All functions operate on `&[f64]` return slices and return scalar `f64`.

use crate::math::stats::{mean, quantile, variance};

fn has_strict_confidence(confidence: f64) -> bool {
    confidence.is_finite() && confidence > 0.0 && confidence < 1.0
}

fn valid_horizon(ann_factor: Option<f64>) -> bool {
    ann_factor.is_none_or(|af| af.is_finite() && af > 0.0)
}

/// Historical Value-at-Risk at the given confidence level.
///
/// Computes the `(1 - confidence)` quantile of the empirical return
/// distribution. For example, at `confidence = 0.95`, the 5th percentile
/// is returned — losses exceeding this threshold occur with 5% probability
/// under the historical distribution.
///
/// Returns a **negative** number representing the loss threshold.
/// The `ann_factor` parameter is not applied (see parameter docs below).
///
/// # Quantile interpolation
///
/// Uses [`finstack_core::math::stats::quantile`]; see that function for
/// the exact interpolation method and cross-tool comparison notes.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
/// * `confidence` - Confidence level in `(0, 1)`, e.g. `0.95` for 95% VaR.
/// * `ann_factor` - Reserved; not applied for historical (empirical) VaR
///   because sqrt-T scaling is invalid for non-parametric quantiles. Pass
///   `None`.
///
/// # Returns
///
/// The VaR as a non-positive scalar. Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::value_at_risk;
///
/// let data: Vec<f64> = (-100..=100).map(|i| i as f64 / 100.0).collect();
/// let var95 = value_at_risk(&data, 0.95, None);
/// assert!(var95 < -0.8);
/// ```
///
/// # References
///
/// - J.P. Morgan RiskMetrics (1996): see docs/REFERENCES.md#jpmorgan1996RiskMetrics
#[must_use]
pub fn value_at_risk(returns: &[f64], confidence: f64, ann_factor: Option<f64>) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let mut data: Vec<f64> = returns.to_vec();
    value_at_risk_with_scratch(&mut data, confidence, ann_factor)
}

/// Historical VaR using a caller-provided scratch buffer (avoids allocation).
///
/// The contents of `scratch` will be partially reordered by `quantile`.
#[must_use]
pub(crate) fn value_at_risk_with_scratch(
    scratch: &mut [f64],
    confidence: f64,
    ann_factor: Option<f64>,
) -> f64 {
    if scratch.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let var = quantile(scratch, 1.0 - confidence);
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
/// * `ann_factor` - Reserved; not applied for historical (empirical) ES
///   because sqrt-T scaling is invalid for non-parametric tail means. Pass
///   `None`.
///
/// # Returns
///
/// The Expected Shortfall as a non-positive scalar. Returns `0.0` for
/// an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::{value_at_risk, expected_shortfall};
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
#[must_use]
pub fn expected_shortfall(returns: &[f64], confidence: f64, ann_factor: Option<f64>) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let mut data: Vec<f64> = returns.to_vec();
    expected_shortfall_with_scratch(&mut data, confidence, ann_factor)
}

/// Expected Shortfall using a caller-provided scratch buffer (avoids allocation).
///
/// The contents of `scratch` will be partially reordered by `quantile`.
#[must_use]
pub(crate) fn expected_shortfall_with_scratch(
    scratch: &mut [f64],
    confidence: f64,
    ann_factor: Option<f64>,
) -> f64 {
    if scratch.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let p = 1.0 - confidence;
    let var_threshold = quantile(scratch, p);
    let mut tail_sum = 0.0;
    let mut tail_count = 0usize;
    for &value in scratch.iter() {
        if value <= var_threshold {
            tail_sum += value;
            tail_count += 1;
        }
    }
    let es = if tail_count == 0 {
        var_threshold
    } else {
        tail_sum / tail_count as f64
    };
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
/// use finstack_analytics::risk_metrics::tail_ratio;
///
/// // Symmetric distribution → tail ratio ≈ 1.
/// let r: Vec<f64> = (-50..=50).map(|i| i as f64 * 0.01).collect();
/// let tr = tail_ratio(&r, 0.95);
/// assert!((tr - 1.0).abs() < 0.1);
/// ```
#[must_use]
pub fn tail_ratio(returns: &[f64], confidence: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let mut data: Vec<f64> = returns.to_vec();
    tail_ratio_with_scratch(&mut data, confidence)
}

/// Tail ratio using a caller-provided scratch buffer (avoids allocation).
#[must_use]
pub(crate) fn tail_ratio_with_scratch(scratch: &mut [f64], confidence: f64) -> f64 {
    if scratch.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let upper = quantile(scratch, confidence).abs();
    let lower = quantile(scratch, 1.0 - confidence).abs();
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
/// use finstack_analytics::risk_metrics::outlier_win_ratio;
///
/// let r: Vec<f64> = (0..100).map(|i| i as f64 * 0.001).collect();
/// let ratio = outlier_win_ratio(&r, 0.95);
/// assert!(ratio < 0.06);
/// ```
#[must_use]
pub fn outlier_win_ratio(returns: &[f64], confidence: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let mut data: Vec<f64> = returns.to_vec();
    outlier_win_ratio_with_scratch(returns, &mut data, confidence)
}

/// Outlier win ratio using a caller-provided scratch buffer (avoids allocation).
///
/// `original` must contain the un-reordered return data for counting.
#[must_use]
pub(crate) fn outlier_win_ratio_with_scratch(
    original: &[f64],
    scratch: &mut [f64],
    confidence: f64,
) -> f64 {
    if scratch.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let threshold = quantile(scratch, confidence);
    let count = original.iter().filter(|&&r| r > threshold).count();
    count as f64 / original.len() as f64
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
/// use finstack_analytics::risk_metrics::outlier_loss_ratio;
///
/// let r: Vec<f64> = (0..100).map(|i| i as f64 * 0.001).collect();
/// let ratio = outlier_loss_ratio(&r, 0.95);
/// assert!(ratio < 0.06);
/// ```
#[must_use]
pub fn outlier_loss_ratio(returns: &[f64], confidence: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let mut data: Vec<f64> = returns.to_vec();
    outlier_loss_ratio_with_scratch(returns, &mut data, confidence)
}

/// Outlier loss ratio using a caller-provided scratch buffer (avoids allocation).
///
/// `original` must contain the un-reordered return data for counting.
#[must_use]
pub(crate) fn outlier_loss_ratio_with_scratch(
    original: &[f64],
    scratch: &mut [f64],
    confidence: f64,
) -> f64 {
    if scratch.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    let threshold = quantile(scratch, 1.0 - confidence);
    let count = original.iter().filter(|&&r| r < threshold).count();
    count as f64 / original.len() as f64
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
/// use finstack_analytics::risk_metrics::skewness;
///
/// // Symmetric distribution → skewness ≈ 0.
/// let r: Vec<f64> = (-50..=50).map(|i| i as f64 * 0.01).collect();
/// assert!(skewness(&r).abs() < 1e-10);
/// ```
///
/// # References
///
/// - Joanes & Gill (1998): see docs/REFERENCES.md#joanesGill1998
#[must_use]
pub fn skewness(returns: &[f64]) -> f64 {
    let (_, _, skew, _) = moments4(returns);
    skew
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
/// use finstack_analytics::risk_metrics::kurtosis;
///
/// // Uniform distribution has negative excess kurtosis.
/// let r: Vec<f64> = (0..1000).map(|i| i as f64 / 1000.0).collect();
/// assert!(kurtosis(&r) < 0.0);
/// ```
///
/// # References
///
/// - Joanes & Gill (1998): see docs/REFERENCES.md#joanesGill1998
#[must_use]
pub fn kurtosis(returns: &[f64]) -> f64 {
    let (_, _, _, kurt) = moments4(returns);
    kurt
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
/// * `ann_factor` - If `Some(f)`, scales the mean term by `f` and the
///   volatility term by `sqrt(f)`.
///
/// # Returns
///
/// The parametric VaR (typically negative). Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::risk_metrics::parametric_var;
///
/// let r: Vec<f64> = (-100..=100).map(|i| i as f64 / 100.0).collect();
/// let pvar = parametric_var(&r, 0.95, None);
/// assert!(pvar < 0.0);
/// ```
///
/// # References
///
/// - J.P. Morgan RiskMetrics (1996): see docs/REFERENCES.md#jpmorgan1996RiskMetrics
#[must_use]
pub fn parametric_var(returns: &[f64], confidence: f64, ann_factor: Option<f64>) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    if !valid_horizon(ann_factor) {
        return f64::NAN;
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
/// * `ann_factor` - If `Some(f)`, scales the mean term by `f` and the
///   volatility term by `sqrt(f)`.
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
/// use finstack_analytics::risk_metrics::{parametric_var, cornish_fisher_var};
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
#[must_use]
pub fn cornish_fisher_var(returns: &[f64], confidence: f64, ann_factor: Option<f64>) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if !has_strict_confidence(confidence) {
        return f64::NAN;
    }
    if !valid_horizon(ann_factor) {
        return f64::NAN;
    }
    let (m, vol, s, k) = moments4(returns);
    let z = crate::math::special_functions::standard_normal_inv_cdf(1.0 - confidence);
    let z2 = z * z;
    let z3 = z2 * z;
    let z_cf =
        z + (z2 - 1.0) * s / 6.0 + (z3 - 3.0 * z) * k / 24.0 - (2.0 * z3 - 5.0 * z) * s * s / 36.0;
    match ann_factor {
        Some(af) => m * af + z_cf * vol * af.sqrt(),
        None => m + z_cf * vol,
    }
}

/// Compute mean, standard deviation, skewness (G₁), and excess kurtosis (G₂) in a single pass.
///
/// Uses a one-pass algorithm accumulating central moments (Pebay 2008, Terriberry 2007).
/// Returns `(mean, std_dev, skewness, excess_kurtosis)` matching the bias-corrected
/// formulas used by `skewness()` and `kurtosis()`.
pub fn moments4(returns: &[f64]) -> (f64, f64, f64, f64) {
    let n = returns.len();
    if n == 0 {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let nf = n as f64;

    let mut m1 = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut m3 = 0.0_f64;
    let mut m4 = 0.0_f64;
    let mut k = 0.0_f64;

    for &r in returns {
        k += 1.0;
        let d = r - m1;
        let d_over_k = d / k;
        let d_over_k2 = d_over_k * d_over_k;
        let term1 = d * d_over_k * (k - 1.0);
        m4 += term1 * d_over_k2 * (k * k - 3.0 * k + 3.0) + 6.0 * d_over_k2 * m2
            - 4.0 * d_over_k * m3;
        m3 += term1 * d_over_k * (k - 2.0) - 3.0 * d_over_k * m2;
        m2 += term1;
        m1 += d_over_k;
    }

    let sample_var = if n < 2 { 0.0 } else { m2 / (nf - 1.0) };
    let vol = sample_var.sqrt();

    let skew = if n < 3 || sample_var == 0.0 {
        0.0
    } else {
        let s3 = vol * vol * vol;
        let adj = nf / ((nf - 1.0) * (nf - 2.0));
        adj * (m3 / s3)
    };

    let kurt = if n < 4 || sample_var == 0.0 {
        0.0
    } else {
        let s4 = sample_var * sample_var;
        let sum_z4 = m4 / s4;
        let a = (nf * (nf + 1.0)) / ((nf - 1.0) * (nf - 2.0) * (nf - 3.0));
        let b = (3.0 * (nf - 1.0) * (nf - 1.0)) / ((nf - 2.0) * (nf - 3.0));
        a * sum_z4 - b
    };

    (m1, vol, skew, kurt)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::math::stats::{mean, variance};

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
    fn expected_shortfall_includes_all_values_at_var_threshold() {
        let data = [-3.0, -1.0, -1.0, 10.0];
        let es = expected_shortfall(&data, 0.5, None);
        let expected = (-3.0 - 1.0 - 1.0) / 3.0;
        assert!((es - expected).abs() < 1e-12);
    }

    #[test]
    fn skewness_symmetric_zero() {
        let r: Vec<f64> = (-50..=50).map(|i| i as f64 * 0.01).collect();
        assert!(skewness(&r).abs() < 1e-10);
    }

    #[test]
    fn skewness_hand_calc() {
        let r = [0.0, 0.0, 0.0, 0.0, 5.0];
        assert!((skewness(&r) - 5.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn skewness_negative() {
        let r = [0.0, 0.0, 0.0, 0.0, -5.0];
        assert!((skewness(&r) - (-5.0_f64.sqrt())).abs() < 1e-12);
    }

    #[test]
    fn skewness_edge_cases() {
        assert_eq!(skewness(&[]), 0.0);
        assert_eq!(skewness(&[1.0, 2.0]), 0.0);
        assert_eq!(skewness(&[3.0, 3.0, 3.0]), 0.0);
    }

    #[test]
    fn kurtosis_hand_calc() {
        let r = [0.0, 0.0, 0.0, 0.0, 5.0];
        assert!((kurtosis(&r) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn kurtosis_uniform_analytical() {
        let r: Vec<f64> = (0..10000).map(|i| i as f64 / 10000.0).collect();
        let k = kurtosis(&r);
        assert!((k - (-1.2)).abs() < 0.005);
    }

    #[test]
    fn kurtosis_edge_cases() {
        assert_eq!(kurtosis(&[]), 0.0);
        assert_eq!(kurtosis(&[1.0, 2.0, 3.0]), 0.0);
        assert_eq!(kurtosis(&[1.0, 1.0, 1.0, 1.0]), 0.0);
    }

    #[test]
    fn parametric_var_formula_verification() {
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
        let r: Vec<f64> = (-500..=500).map(|i| i as f64 / 5000.0).collect();
        let pvar = parametric_var(&r, 0.95, None);
        let cfvar = cornish_fisher_var(&r, 0.95, None);
        assert!((cfvar - pvar).abs() < 0.02);
    }

    #[test]
    fn cornish_fisher_var_differs_from_parametric_for_skewed() {
        let mut r = vec![-0.01_f64; 80];
        r.extend(vec![0.05; 20]);
        let pvar = parametric_var(&r, 0.95, None);
        let cfvar = cornish_fisher_var(&r, 0.95, None);
        assert!(pvar < 0.0);
        assert!(cfvar < 0.0);
        assert!((cfvar - pvar).abs() > 1e-6);
    }

    #[test]
    fn cornish_fisher_var_empty() {
        assert_eq!(cornish_fisher_var(&[], 0.95, None), 0.0);
    }

    #[test]
    fn es_is_worse_than_var_always() {
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
    fn historical_var_does_not_apply_sqrt_time_scaling() {
        let returns = [-0.03, -0.02, -0.01, 0.01, 0.02];
        let period_var = value_at_risk(&returns, 0.95, None);
        let annualized_var = value_at_risk(&returns, 0.95, Some(252.0));
        assert!((annualized_var - period_var).abs() < 1e-14);
    }

    #[test]
    fn historical_es_does_not_apply_sqrt_time_scaling() {
        let returns = [-0.03, -0.02, -0.01, 0.01, 0.02];
        let period_es = expected_shortfall(&returns, 0.95, None);
        let annualized_es = expected_shortfall(&returns, 0.95, Some(252.0));
        assert!((annualized_es - period_es).abs() < 1e-14);
    }

    #[test]
    fn cornish_fisher_formula_verification() {
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
