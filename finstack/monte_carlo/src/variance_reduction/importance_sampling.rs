//! Importance sampling variance reduction.
//!
//! Uses exponential tilting to shift the sampling distribution toward
//! rare events (deep OTM, barriers, tail risks).
//!
//! The tilted estimator is: E[X] ≈ E*[X * L(ω)]
//! where L is the likelihood ratio and E* is under the tilted measure.
//!
//! # ESS Threshold Warning
//!
//! The Effective Sample Size (ESS) measures the efficiency of importance sampling.
//! When ESS/N drops below 10% (typically 0.1), the variance of the estimator
//! becomes unreliable and results should be treated with caution.
//!
//! Reference: Kong et al. (1994) "Sequential Imputations and Bayesian Missing Data Problems"

/// Default ESS ratio threshold below which a warning is triggered.
/// An ESS ratio of 0.1 (10%) is a common threshold in practice.
pub const DEFAULT_ESS_THRESHOLD: f64 = 0.1;

/// Compute exponential tilting for barrier options.
///
/// Shifts the drift toward the barrier to increase hit probability,
/// then re-weights samples using likelihood ratios.
///
/// # Arguments
///
/// * `theta` - Tilting parameter (drift shift)
/// * `z` - Standard normal sample
///
/// # Returns
///
/// (tilted_z, likelihood_ratio)
///
/// The likelihood ratio is: L(Z) = exp(θ'Z - ½θ'θ)
pub fn exponential_tilt(theta: f64, z: f64) -> (f64, f64) {
    let tilted_z = z + theta;
    let log_likelihood = theta * z - 0.5 * theta * theta;
    let likelihood_ratio = log_likelihood.exp();

    (tilted_z, likelihood_ratio)
}

/// Result of importance sampling estimation with ESS diagnostics.
#[derive(Debug, Clone)]
pub struct ImportanceSamplingResult {
    /// Weighted mean estimate
    pub mean: f64,
    /// Standard error of the estimate
    pub stderr: f64,
    /// Effective sample size
    pub ess: f64,
    /// ESS ratio (ESS / N), ranges from 0 to 1
    pub ess_ratio: f64,
    /// True if ESS ratio is below threshold (estimate may be unreliable)
    pub low_ess_warning: bool,
}

/// Compute weighted estimate with importance sampling.
///
/// # Arguments
///
/// * `values` - Sampled values under tilted measure
/// * `weights` - Likelihood ratios
///
/// # Returns
///
/// (weighted_mean, weighted_stderr)
pub fn weighted_estimate(values: &[f64], weights: &[f64]) -> (f64, f64) {
    let result = weighted_estimate_with_diagnostics(values, weights, DEFAULT_ESS_THRESHOLD);
    (result.mean, result.stderr)
}

/// Compute weighted estimate with importance sampling and full diagnostics.
///
/// Returns detailed results including ESS and warning flag when ESS is low.
///
/// # Arguments
///
/// * `values` - Sampled values under tilted measure
/// * `weights` - Likelihood ratios
/// * `ess_threshold` - ESS ratio threshold for warning (typically 0.1)
///
/// # Returns
///
/// `ImportanceSamplingResult` with mean, stderr, ESS, and warning flag
///
/// # Example
///
/// ```rust,ignore
/// use finstack_monte_carlo::variance_reduction::importance_sampling::{
///     weighted_estimate_with_diagnostics, DEFAULT_ESS_THRESHOLD
/// };
///
/// let values = vec![1.0, 2.0, 3.0, 4.0];
/// let weights = vec![1.0, 1.0, 1.0, 1.0]; // Uniform weights
///
/// let result = weighted_estimate_with_diagnostics(&values, &weights, DEFAULT_ESS_THRESHOLD);
/// assert!(!result.low_ess_warning); // Uniform weights = good ESS
/// ```
pub fn weighted_estimate_with_diagnostics(
    values: &[f64],
    weights: &[f64],
    ess_threshold: f64,
) -> ImportanceSamplingResult {
    assert_eq!(values.len(), weights.len());

    if values.is_empty() {
        return ImportanceSamplingResult {
            mean: 0.0,
            stderr: 0.0,
            ess: 0.0,
            ess_ratio: 0.0,
            low_ess_warning: true,
        };
    }

    let n = values.len() as f64;

    // Compute ESS
    let ess = effective_sample_size(weights);
    let ess_ratio = ess / n;
    let low_ess_warning = ess_ratio < ess_threshold;

    // Compute weighted mean
    let sum_weights: f64 = weights.iter().sum();
    let weighted_sum: f64 = values.iter().zip(weights).map(|(v, w)| v * w).sum();
    let mean = if sum_weights > 1e-10 {
        weighted_sum / sum_weights
    } else {
        0.0
    };

    // Compute weighted variance
    let weighted_var: f64 = values
        .iter()
        .zip(weights)
        .map(|(v, w)| w * (v - mean).powi(2))
        .sum();

    let variance = if sum_weights > 1e-10 {
        weighted_var / sum_weights
    } else {
        0.0
    };

    let stderr = (variance / n).sqrt();

    ImportanceSamplingResult {
        mean,
        stderr,
        ess,
        ess_ratio,
        low_ess_warning,
    }
}

/// Effective sample size for weighted samples.
///
/// ESS = (Σw_i)² / Σ(w_i²)
///
/// Measures the efficiency of importance sampling.
/// ESS = N means perfect (uniform weights), ESS << N means poor weighting.
#[must_use]
pub fn effective_sample_size(weights: &[f64]) -> f64 {
    if weights.is_empty() {
        return 0.0;
    }

    let sum_w: f64 = weights.iter().sum();
    let sum_w2: f64 = weights.iter().map(|w| w * w).sum();

    if sum_w2 > 1e-10 {
        (sum_w * sum_w) / sum_w2
    } else {
        0.0
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_tilt() {
        let (tilted, lr) = exponential_tilt(0.5, 1.0);

        // Tilted should be shifted
        assert_eq!(tilted, 1.5);

        // Likelihood ratio should be positive
        assert!(lr > 0.0);
    }

    #[test]
    fn test_weighted_estimate_uniform() {
        let values = vec![1.0, 2.0, 3.0, 4.0];
        let weights = vec![1.0, 1.0, 1.0, 1.0]; // Uniform weights

        let (mean, _) = weighted_estimate(&values, &weights);

        // Should equal simple mean
        assert!((mean - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_weighted_estimate_non_uniform() {
        let values = vec![1.0, 2.0, 3.0];
        let weights = vec![1.0, 2.0, 1.0]; // Higher weight on middle value

        let (mean, _) = weighted_estimate(&values, &weights);

        // Weighted mean: (1*1 + 2*2 + 3*1) / 4 = 8/4 = 2
        assert!((mean - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_effective_sample_size_uniform() {
        let weights = vec![1.0, 1.0, 1.0, 1.0];
        let ess = effective_sample_size(&weights);

        // Uniform weights should give ESS = N
        assert!((ess - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_effective_sample_size_concentrated() {
        let weights = vec![1.0, 0.1, 0.1, 0.1];
        let ess = effective_sample_size(&weights);

        // One dominant weight should give ESS < N
        assert!(ess < 4.0);
        assert!(ess > 1.0);
    }

    #[test]
    fn test_tilting_increases_likelihood() {
        // Tilting toward positive values
        let theta = 1.0;
        let z_positive = 2.0;
        let z_negative = -2.0;

        let (_, lr_pos) = exponential_tilt(theta, z_positive);
        let (_, lr_neg) = exponential_tilt(theta, z_negative);

        // Positive z should have higher likelihood under positive tilt
        assert!(lr_pos > lr_neg);
    }

    #[test]
    fn test_weighted_estimate_with_diagnostics_uniform() {
        let values = vec![1.0, 2.0, 3.0, 4.0];
        let weights = vec![1.0, 1.0, 1.0, 1.0];

        let result =
            super::weighted_estimate_with_diagnostics(&values, &weights, DEFAULT_ESS_THRESHOLD);

        // Uniform weights should give ESS ratio of 1.0 (perfect)
        assert!((result.ess_ratio - 1.0).abs() < 1e-10);
        assert!(!result.low_ess_warning);
    }

    #[test]
    fn test_weighted_estimate_with_diagnostics_low_ess() {
        // Create weights where one sample dominates
        let values = vec![1.0; 100];
        let mut weights = vec![0.001; 100];
        weights[0] = 100.0; // One sample dominates

        let result =
            super::weighted_estimate_with_diagnostics(&values, &weights, DEFAULT_ESS_THRESHOLD);

        // ESS ratio should be very low
        assert!(result.ess_ratio < DEFAULT_ESS_THRESHOLD);
        assert!(result.low_ess_warning);
    }

    #[test]
    fn test_ess_ratio_boundary() {
        // Create weights that give ESS ratio just above and below threshold
        let n = 100;
        let values = vec![1.0; n];

        // Uniform weights -> ESS ratio = 1.0
        let uniform_weights = vec![1.0; n];
        let result_uniform = super::weighted_estimate_with_diagnostics(
            &values,
            &uniform_weights,
            DEFAULT_ESS_THRESHOLD,
        );
        assert!(!result_uniform.low_ess_warning);

        // Highly concentrated weights -> low ESS
        let mut concentrated = vec![1.0; n];
        concentrated[0] = 1000.0;
        let result_concentrated = super::weighted_estimate_with_diagnostics(
            &values,
            &concentrated,
            DEFAULT_ESS_THRESHOLD,
        );
        assert!(result_concentrated.low_ess_warning);
    }
}
