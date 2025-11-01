//! Importance sampling variance reduction.
//!
//! Uses exponential tilting to shift the sampling distribution toward
//! rare events (deep OTM, barriers, tail risks).
//!
//! The tilted estimator is: E[X] ≈ E*[X * L(ω)]
//! where L is the likelihood ratio and E* is under the tilted measure.

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
    assert_eq!(values.len(), weights.len());

    if values.is_empty() {
        return (0.0, 0.0);
    }

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

    let stderr = (variance / values.len() as f64).sqrt();

    (mean, stderr)
}

/// Effective sample size for weighted samples.
///
/// ESS = (Σw_i)² / Σ(w_i²)
///
/// Measures the efficiency of importance sampling.
/// ESS = N means perfect (uniform weights), ESS << N means poor weighting.
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
}
