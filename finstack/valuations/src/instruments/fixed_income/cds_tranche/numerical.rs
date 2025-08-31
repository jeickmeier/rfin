//! Numerical integration for CDS tranche pricing.
//!
//! Implements Gauss-Hermite quadrature for integrating over the standard
//! normal distribution in the Gaussian Copula model.

use finstack_core::F;

/// Gauss-Hermite quadrature points and weights for numerical integration
/// over the standard normal distribution.
///
/// These are pre-computed for common quadrature orders to avoid runtime
/// computation of the nodes and weights.
pub struct GaussHermiteQuadrature {
    /// Quadrature points (x-coordinates)
    pub points: &'static [F],
    /// Quadrature weights
    pub weights: &'static [F],
}

impl GaussHermiteQuadrature {
    /// Get the 5-point Gauss-Hermite quadrature.
    ///
    /// This provides a good balance between accuracy and performance
    /// for most credit modeling applications.
    pub fn order_5() -> Self {
        Self {
            points: &[
                -2.0201828704560856,
                -0.9585724646138185,
                0.0,
                0.9585724646138185,
                2.0201828704560856,
            ],
            weights: &[
                0.019_953_242_059_045_913,
                0.393_619_323_152_241_2,
                0.945_308_720_482_941_9,
                0.393_619_323_152_241_2,
                0.019_953_242_059_045_913,
            ],
        }
    }

    /// Get the 7-point Gauss-Hermite quadrature.
    ///
    /// Higher accuracy for more demanding applications where precision
    /// is critical and computational cost is acceptable.
    pub fn order_7() -> Self {
        Self {
            points: &[
                -2.6519613568352334,
                -1.6735516287674718,
                -0.8162878828589647,
                0.0,
                0.8162878828589647,
                1.6735516287674718,
                2.6519613568352334,
            ],
            weights: &[
                0.0009717812450995192,
                0.05451558281912703,
                0.4256072526101278,
                0.8102646175568073,
                0.4256072526101278,
                0.05451558281912703,
                0.0009717812450995192,
            ],
        }
    }

    /// Get the 10-point Gauss-Hermite quadrature.
    ///
    /// High accuracy for demanding applications where very precise
    /// integration is required.
    pub fn order_10() -> Self {
        Self {
            points: &[
                -3.4361591188377376,
                -2.5327316742327897,
                -1.7566836492998817,
                -1.0366108297895136,
                -0.3429013272237046,
                0.3429013272237046,
                1.0366108297895136,
                1.7566836492998817,
                2.5327316742327897,
                3.4361591188377376,
            ],
            weights: &[
                7.640_432_855_232_62e-6,
                0.001_343_645_746_781_272_8,
                0.033_874_394_455_481_063,
                0.240_138_611_082_314_67,
                0.610_862_633_735_325_8,
                0.610_862_633_735_325_8,
                0.240_138_611_082_314_67,
                0.033_874_394_455_481_063,
                0.001_343_645_746_781_272_8,
                7.640_432_855_232_62e-6,
            ],
        }
    }

    /// Integrate a function over the standard normal distribution.
    ///
    /// # Arguments
    /// * `f` - Function to integrate, takes x (standard normal variate) as input
    ///
    /// # Returns
    /// The approximate integral of f(x) * φ(x) dx from -∞ to +∞,
    /// where φ(x) is the standard normal PDF.
    pub fn integrate<F2>(&self, f: F2) -> F
    where
        F2: Fn(F) -> F,
    {
        let mut result = 0.0;
        let sqrt_2 = std::f64::consts::SQRT_2; // √2

        for (i, &z) in self.points.iter().enumerate() {
            result += self.weights[i] * f(sqrt_2 * z); // Evaluate at √2 * node
        }

        result / std::f64::consts::PI.sqrt() // 1/√π
    }
}

/// Standard normal cumulative distribution function.
///
/// Delegates to the existing implementation from the options models
/// to maintain consistency across the library.
pub fn standard_normal_cdf(x: F) -> F {
    crate::instruments::options::models::black::norm_cdf(x)
}

/// Inverse standard normal cumulative distribution function.
///
/// Uses a simplified but robust implementation suitable for credit modeling.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gauss_hermite_quadrature_normalization() {
        let quad = GaussHermiteQuadrature::order_5();

        // Test that integrating 1 over standard normal gives approximately 1
        let integral = quad.integrate(|_x| 1.0);
        assert!((integral - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_gauss_hermite_quadrature_polynomial() {
        let quad = GaussHermiteQuadrature::order_7();

        // Test that integrating x^2 over standard normal gives 1 (variance)
        let integral = quad.integrate(|x| x * x);
        assert!(
            (integral - 1.0).abs() < 0.1,
            "Integral of x² should be ~1, got {}",
            integral
        );
    }

    #[test]
    fn test_standard_normal_cdf() {
        // Test known values
        assert!((standard_normal_cdf(0.0) - 0.5).abs() < 1e-6);
        assert!((standard_normal_cdf(1.0) - 0.8413447460685429).abs() < 1e-6);
        assert!((standard_normal_cdf(-1.0) - 0.15865525393145705).abs() < 1e-6);

        // Test extreme values
        assert!(standard_normal_cdf(-10.0) < 1e-10);
        assert!(standard_normal_cdf(10.0) > 1.0 - 1e-10);
    }

    #[test]
    fn test_standard_normal_inv_cdf() {
        // Test known values
        assert!((standard_normal_inv_cdf(0.5) - 0.0).abs() < 1e-6);
        assert!((standard_normal_inv_cdf(0.8413447460685429) - 1.0).abs() < 1e-3);
        assert!((standard_normal_inv_cdf(0.15865525393145705) - (-1.0)).abs() < 1e-3);
    }

    #[test]
    fn test_normal_cdf_inv_cdf_roundtrip() {
        let test_values = [0.1, 0.25, 0.5, 0.75, 0.9]; // Skip extreme values for robustness

        for &p in &test_values {
            let x = standard_normal_inv_cdf(p);
            let p_back = standard_normal_cdf(x);
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
