//! Random number transforms.
//!
//! Provides transforms from uniform to other distributions,
//! including Box-Muller for normal random variables.

use std::f64::consts::PI;

// Re-export inverse normal CDF from core/math (better tail handling)
pub use finstack_core::math::special_functions::standard_normal_inv_cdf as inverse_normal_cdf;

/// Box-Muller transform: U(0,1)² → N(0,1)².
///
/// Generates two independent standard normal random variables
/// from two independent uniform random variables.
///
/// # Arguments
///
/// * `u1` - First uniform random variable in (0, 1)
/// * `u2` - Second uniform random variable in (0, 1)
///
/// # Returns
///
/// Tuple of two independent N(0,1) random variables.
///
/// # Algorithm
///
/// ```text
/// z1 = √(-2 ln u1) cos(2π u2)
/// z2 = √(-2 ln u1) sin(2π u2)
/// ```
#[inline]
pub fn box_muller_transform(u1: f64, u2: f64) -> (f64, f64) {
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * PI * u2;
    let z1 = r * theta.cos();
    let z2 = r * theta.sin();
    (z1, z2)
}

/// Polar form of Box-Muller (rejection-based, typically faster).
///
/// # Arguments
///
/// * `gen_u01` - Function that generates U(0,1) random variables
///
/// # Returns
///
/// Tuple of two independent N(0,1) random variables.
pub fn box_muller_polar<F>(mut gen_u01: F) -> (f64, f64)
where
    F: FnMut() -> f64,
{
    loop {
        let u1 = 2.0 * gen_u01() - 1.0;
        let u2 = 2.0 * gen_u01() - 1.0;
        let s = u1 * u1 + u2 * u2;

        if s > 0.0 && s < 1.0 {
            let factor = (-2.0 * s.ln() / s).sqrt();
            return (u1 * factor, u2 * factor);
        }
    }
}

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

    #[test]
    fn test_box_muller() {
        use finstack_core::math::RandomNumberGenerator;
        let (z1, z2) = box_muller_transform(0.5, 0.5);
        assert!(z1.is_finite());
        assert!(z2.is_finite());

        // Test with many samples
        let mut rng = finstack_core::math::random::SimpleRng::new(42);
        let mut samples = Vec::new();
        for _ in 0..500 {
            let u1 = rng.uniform();
            let u2 = rng.uniform();
            let (z1, z2) = box_muller_transform(u1, u2);
            samples.push(z1);
            samples.push(z2);
        }

        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let var =
            samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (samples.len() - 1) as f64;

        assert!(mean.abs() < 0.1);
        assert!((var - 1.0).abs() < 0.2);
    }

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

    #[test]
    fn test_box_muller_polar() {
        use finstack_core::math::RandomNumberGenerator;
        let mut rng = finstack_core::math::random::SimpleRng::new(42);
        let gen_u01 = || rng.uniform();

        let (z1, z2) = box_muller_polar(gen_u01);
        assert!(z1.is_finite());
        assert!(z2.is_finite());
    }
}
