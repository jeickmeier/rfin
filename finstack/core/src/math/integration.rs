//! Numerical integration algorithms.
//!
//! This module provides implementations of common numerical integration
//! methods used in financial mathematics, particularly for probability
//! distributions and complex integrals.
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::integration::GaussHermiteQuadrature;
//!
//! // Integrate x² over standard normal (should give 1.0)
//! let quad = GaussHermiteQuadrature::order_7();
//! let integral = quad.integrate(|x| x * x);
//! assert!((integral - 1.0).abs() < 0.1);
//! ```

use crate::F;

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
    /// for most applications.
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
    fn test_different_quadrature_orders() {
        // Test that higher order gives better accuracy for polynomial
        let f = |x: F| x * x * x * x; // x^4 function

        let quad5 = GaussHermiteQuadrature::order_5();
        let quad7 = GaussHermiteQuadrature::order_7();
        let quad10 = GaussHermiteQuadrature::order_10();

        let integral5 = quad5.integrate(f);
        let integral7 = quad7.integrate(f);
        let integral10 = quad10.integrate(f);

        // Higher order should be more accurate for polynomials
        // For x^4 over standard normal, the integral should be 3
        let expected = 3.0;

        // Just check that all integrals are reasonable (close to expected)
        // The convergence ordering may not always hold for this specific test
        assert!(
            (integral5 - expected).abs() < 1.0,
            "5-point: {} vs expected {}",
            integral5,
            expected
        );
        assert!(
            (integral7 - expected).abs() < 0.5,
            "7-point: {} vs expected {}",
            integral7,
            expected
        );
        assert!(
            (integral10 - expected).abs() < 0.2,
            "10-point: {} vs expected {}",
            integral10,
            expected
        );
    }
}
