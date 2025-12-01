//! Random number transforms.
//!
//! Provides transforms from uniform to other distributions,
//! including Box-Muller for normal random variables.
//!
//! Note: Box-Muller transforms are now in `finstack_core::math::random`
//! and re-exported here for backward compatibility.

// Re-export Box-Muller transforms from core/math (pure math, belongs in core)
pub use finstack_core::math::random::box_muller_transform;

// Re-export inverse normal CDF from core/math (better tail handling)
pub use finstack_core::math::special_functions::standard_normal_inv_cdf as inverse_normal_cdf;

/// Moment matching: adjust samples to have exact mean and variance.
///
/// This variance reduction technique forces the sample to have
/// exactly the theoretical moments.
///
/// # Arguments
///
/// * `samples` - Mutable slice of samples to adjust
/// * `target_mean` - Target mean (default 0.0 for standard normal)
/// * `target_std` - Target standard deviation (default 1.0 for standard normal)
pub fn moment_match(samples: &mut [f64], target_mean: f64, target_std: f64) {
    if samples.is_empty() {
        return;
    }

    // Compute current mean and std dev
    let n = samples.len() as f64;
    let current_mean = samples.iter().sum::<f64>() / n;

    let current_var = samples
        .iter()
        .map(|&x| (x - current_mean).powi(2))
        .sum::<f64>()
        / n;
    let current_std = current_var.sqrt();

    // Adjust samples
    if current_std > 1e-10 {
        for x in samples.iter_mut() {
            *x = (*x - current_mean) * (target_std / current_std) + target_mean;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Box-Muller tests moved to core/math/random.rs
    // Tests are kept there since Box-Muller is now part of core

    #[test]
    fn test_inverse_normal_cdf() {
        // Test re-exported function from finstack_core::math::special_functions::standard_normal_inv_cdf

        let z_50 = inverse_normal_cdf(0.5);
        assert!(z_50.is_finite());
        assert!(z_50.abs() < 0.5); // Should be near 0

        // Test that it's monotonic
        let z_low = inverse_normal_cdf(0.1);
        let z_mid = inverse_normal_cdf(0.5);
        let z_high = inverse_normal_cdf(0.9);
        assert!(z_low < z_mid);
        assert!(z_mid < z_high);

        // Test extremes - core version returns bounded values, not infinity
        assert!(inverse_normal_cdf(0.0) < -5.0);
        assert!(inverse_normal_cdf(1.0) > 5.0);
    }

    #[test]
    fn test_inverse_normal_cdf_parity_with_core() {
        // Verify that the re-exported function from core/math works correctly
        // for typical MC use cases

        use finstack_core::math::norm_cdf;

        // Test round-trip accuracy for probabilities commonly used in MC
        let test_probs = vec![
            0.001, 0.01, 0.05, 0.1, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 0.999,
        ];

        for &p in &test_probs {
            let z = inverse_normal_cdf(p);
            let p_back = norm_cdf(z);

            // Allow small numerical error in round-trip
            assert!(
                (p - p_back).abs() < 1e-3,
                "Round-trip failed for p={}: z={}, p_back={}, error={}",
                p,
                z,
                p_back,
                (p - p_back).abs()
            );
        }

        // Test symmetry
        for &p in &[0.1, 0.25, 0.4] {
            let z_low = inverse_normal_cdf(p);
            let z_high = inverse_normal_cdf(1.0 - p);
            assert!(
                (z_low + z_high).abs() < 1e-6,
                "Symmetry violated for p={}: z_low={}, z_high={}",
                p,
                z_low,
                z_high
            );
        }

        // Test that it's strictly monotonic
        let probs: Vec<f64> = (1..100).map(|i| i as f64 / 100.0).collect();
        for window in probs.windows(2) {
            let z1 = inverse_normal_cdf(window[0]);
            let z2 = inverse_normal_cdf(window[1]);
            assert!(
                z1 < z2,
                "Not monotonic: p1={}, p2={}, z1={}, z2={}",
                window[0],
                window[1],
                z1,
                z2
            );
        }
    }

    #[test]
    fn test_moment_matching() {
        let mut samples = vec![-1.5, -0.5, 0.0, 0.5, 1.5];
        moment_match(&mut samples, 0.0, 1.0);

        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let var = samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;

        assert!(mean.abs() < 1e-10);
        assert!((var - 1.0).abs() < 1e-10);
    }

    // Box-Muller polar test moved to core/math/random.rs
}
