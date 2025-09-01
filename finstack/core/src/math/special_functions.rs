//! Special mathematical functions (error function, normal distribution, etc.).
//!
//! This module provides implementations of common special functions used
//! throughout financial mathematics, with emphasis on numerical stability
//! and deterministic results.
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::special_functions::{norm_cdf, norm_pdf, standard_normal_inv_cdf};
//! 
//! // Standard normal CDF at zero should be 0.5
//! assert!((norm_cdf(0.0) - 0.5).abs() < 1e-6);
//! 
//! // Round-trip test for inverse CDF
//! let x = standard_normal_inv_cdf(0.84);
//! let p_back = norm_cdf(x);
//! assert!((p_back - 0.84).abs() < 1e-3);
//! ```

use crate::F;
use std::f64::consts::PI;

/// Error function approximation (Abramowitz and Stegun).
///
/// Provides a fast, accurate approximation to the error function
/// using polynomial approximation.
///
/// # Arguments
/// * `x` - Input value
///
/// # Returns
/// erf(x) ≈ 2/√π ∫₀ˣ e^(-t²) dt
#[inline]
pub fn erf(x: F) -> F {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    let y = 1.0 - ((((a5 * t5 + a4 * t4) + a3 * t3) + a2 * t2) + a1 * t) * (-x * x).exp();

    sign * y
}

/// Cumulative standard normal distribution function.
///
/// # Arguments
/// * `x` - Input value
///
/// # Returns
/// Φ(x) = P(Z ≤ x) where Z ~ N(0,1)
#[inline]
pub fn norm_cdf(x: F) -> F {
    0.5 * (1.0 + erf(x / (2.0_f64).sqrt()))
}

/// Standard normal probability density function.
///
/// # Arguments
/// * `x` - Input value
///
/// # Returns
/// φ(x) = (1/√(2π)) * e^(-x²/2)
#[inline]
pub fn norm_pdf(x: F) -> F {
    (-0.5 * x * x).exp() / (2.0 * PI).sqrt()
}

/// Inverse standard normal cumulative distribution function.
///
/// Uses the Beasley-Springer-Moro algorithm for robust approximation
/// suitable for financial modeling.
///
/// # Arguments
/// * `p` - Probability in (0, 1)
///
/// # Returns
/// x such that Φ(x) = p
pub fn standard_normal_inv_cdf(p: F) -> F {
    if p <= 0.0 {
        return -8.0; // Practical limit instead of infinity
    }
    if p >= 1.0 {
        return 8.0; // Practical limit instead of infinity
    }
    if (p - 0.5).abs() < 1e-12 {
        return 0.0;
    }

    // Rational approximation (Beasley-Springer-Moro algorithm)
    let c = [2.515517, 0.802853, 0.010328];
    let d = [1.432788, 0.189269, 0.001308];

    if p < 0.5 {
        // Use symmetry for p < 0.5
        let t = (-2.0 * p.ln()).sqrt();
        let num = c[2] * t + c[1];
        let den = ((d[2] * t + d[1]) * t + d[0]) * t + 1.0;
        -(t - (c[0] + num * t) / den)
    } else {
        let t = (-2.0 * (1.0 - p).ln()).sqrt();
        let num = c[2] * t + c[1];
        let den = ((d[2] * t + d[1]) * t + d[0]) * t + 1.0;
        t - (c[0] + num * t) / den
    }
}

/// Alias for norm_cdf for compatibility
pub use norm_cdf as standard_normal_cdf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_erf() {
        // Test known values with reasonable tolerance for approximation
        assert!((erf(0.0) - 0.0).abs() < 1e-6, "erf(0) should be 0, got {}", erf(0.0));
        assert!((erf(1.0) - 0.8427007929).abs() < 1e-4, "erf(1) should be ~0.8427, got {}", erf(1.0));
        assert!((erf(-1.0) - (-0.8427007929)).abs() < 1e-4, "erf(-1) should be ~-0.8427, got {}", erf(-1.0));
    }

    #[test]
    fn test_norm_cdf() {
        // Test known values
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-6);
        assert!((norm_cdf(1.0) - 0.8413447460685429).abs() < 1e-6);
        assert!((norm_cdf(-1.0) - 0.15865525393145705).abs() < 1e-6);

        // Test extreme values
        assert!(norm_cdf(-10.0) < 1e-10);
        assert!(norm_cdf(10.0) > 1.0 - 1e-10);
    }

    #[test]
    fn test_norm_pdf() {
        // Test known values
        assert!((norm_pdf(0.0) - (1.0 / (2.0 * PI).sqrt())).abs() < 1e-12);
        
        // Test symmetry
        assert!((norm_pdf(1.0) - norm_pdf(-1.0)).abs() < 1e-12);
        
        // Test non-negativity
        assert!(norm_pdf(5.0) >= 0.0);
    }

    #[test]
    fn test_standard_normal_inv_cdf() {
        // Test known values
        assert!((standard_normal_inv_cdf(0.5) - 0.0).abs() < 1e-6);
        assert!((standard_normal_inv_cdf(0.8413447460685429) - 1.0).abs() < 1e-3);
        assert!((standard_normal_inv_cdf(0.15865525393145705) - (-1.0)).abs() < 1e-3);

        // Test extreme values
        assert!(standard_normal_inv_cdf(1e-10) < -5.0);
        assert!(standard_normal_inv_cdf(1.0 - 1e-10) > 5.0);
    }

    #[test]
    fn test_normal_cdf_inv_cdf_roundtrip() {
        let test_values = [0.1, 0.25, 0.5, 0.75, 0.9]; // Skip extreme values for robustness

        for &p in &test_values {
            let x = standard_normal_inv_cdf(p);
            let p_back = norm_cdf(x);
            assert!(
                (p - p_back).abs() < 1e-3,
                "Failed roundtrip for p={}, got x={}, p_back={}",
                p,
                x,
                p_back
            );
        }
    }
}
