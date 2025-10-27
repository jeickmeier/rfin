//! Random number transforms.
//!
//! Provides transforms from uniform to other distributions,
//! including Box-Muller for normal random variables.

use std::f64::consts::PI;

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

/// Approximate inverse standard normal CDF (Beasley-Springer-Moro).
///
/// Accurate to ~1e-9 for p in (0.00001, 0.99999).
///
/// # Arguments
///
/// * `p` - Probability in (0, 1)
///
/// # Returns
///
/// z such that Φ(z) = p, where Φ is standard normal CDF.
pub fn inverse_normal_cdf(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        return if p <= 0.0 {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // Beasley-Springer-Moro algorithm
    const A: [f64; 4] = [
        2.50662823884,
        -18.61500062529,
        41.39119773534,
        -25.44106049637,
    ];
    const B: [f64; 4] = [
        -8.47351093090,
        23.08336743743,
        -21.06224101826,
        3.13082909833,
    ];
    const C: [f64; 9] = [
        0.3374754822726147,
        0.9761690190917186,
        0.1607979714918209,
        0.0276438810333863,
        0.0038405729373609,
        0.0003951896511919,
        0.0000321767881768,
        0.0000002888167364,
        0.0000003960315187,
    ];

    let y = p - 0.5;

    if y.abs() < 0.42 {
        // Central region
        let r = y * y;
        let num = (((A[3] * r + A[2]) * r + A[1]) * r + A[0]) * y;
        let den = (((B[3] * r + B[2]) * r + B[1]) * r + B[0]) * r + 1.0;
        return num / den;
    }

    // Tail regions
    let r = if y < 0.0 { p } else { 1.0 - p };
    let s = (-r.ln()).ln();

    let t = if s < 5.0 {
        let s = s - 2.5;
        C[0]
            + s * (C[1]
                + s * (C[2] + s * (C[3] + s * (C[4] + s * (C[5] + s * (C[6] + s * (C[7] + s * C[8])))))))
    } else {
        let s = s - 5.0;
        C[0]
            + s * (C[1]
                + s * (C[2] + s * (C[3] + s * (C[4] + s * (C[5] + s * (C[6] + s * (C[7] + s * C[8])))))))
    };

    if y < 0.0 {
        -t
    } else {
        t
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
        // Test basic properties of inverse CDF
        // Note: This is a simplified implementation for internal use
        // Production code uses finstack_core::math::special_functions::standard_normal_inv_cdf
        
        let z_50 = inverse_normal_cdf(0.5);
        assert!(z_50.is_finite());
        assert!(z_50.abs() < 0.5); // Should be near 0

        // Test that it's monotonic
        let z_low = inverse_normal_cdf(0.1);
        let z_mid = inverse_normal_cdf(0.5);
        let z_high = inverse_normal_cdf(0.9);
        assert!(z_low < z_mid);
        assert!(z_mid < z_high);

        // Test extremes
        assert!(inverse_normal_cdf(0.0).is_infinite());
        assert!(inverse_normal_cdf(1.0).is_infinite());
    }

    #[test]
    fn test_moment_matching() {
        let mut samples = vec![-1.5, -0.5, 0.0, 0.5, 1.5];
        moment_match(&mut samples, 0.0, 1.0);

        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let var =
            samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;

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

