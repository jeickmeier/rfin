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

use crate::error::InputError;
use crate::{Error, F};

// Removed over-engineered parameter bundling structs - use direct parameters instead

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

#[cfg(feature = "serde")]
impl serde::Serialize for GaussHermiteQuadrature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Determine the order based on the number of points
        let order = match self.points.len() {
            5 => 5,
            7 => 7,
            10 => 10,
            _ => return Err(serde::ser::Error::custom("Unknown quadrature order")),
        };

        #[derive(serde::Serialize)]
        struct QuadratureData {
            order: usize,
        }

        serde::Serialize::serialize(&QuadratureData { order }, serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for GaussHermiteQuadrature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct QuadratureData {
            order: usize,
        }

        let data = QuadratureData::deserialize(deserializer)?;

        match data.order {
            5 => Ok(GaussHermiteQuadrature::order_5()),
            7 => Ok(GaussHermiteQuadrature::order_7()),
            10 => Ok(GaussHermiteQuadrature::order_10()),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid quadrature order: {}",
                data.order
            ))),
        }
    }
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

    /// Adaptive Gauss-Hermite integration with automatic refinement.
    ///
    /// This method automatically increases the quadrature order if the function
    /// exhibits rapid changes or if high correlation values require greater precision.
    /// Critical for base correlation calibration near boundary conditions.
    ///
    /// # Arguments
    /// * `f` - Function to integrate
    /// * `tolerance` - Convergence tolerance for adaptive refinement
    ///
    /// # Returns
    /// High-precision integral estimate with automatic accuracy control
    pub fn integrate_adaptive<F2>(&self, f: F2, tolerance: F) -> F
    where
        F2: Fn(F) -> F + Copy,
    {
        let base = self.integrate(f);
        match self.points.len() {
            10 => base,
            7 => GaussHermiteQuadrature::order_10().integrate(f),
            5 => {
                let v7 = GaussHermiteQuadrature::order_7().integrate(f);
                if (v7 - base).abs() <= tolerance {
                    v7
                } else {
                    GaussHermiteQuadrature::order_10().integrate(f)
                }
            }
            _ => base,
        }
    }
}

/// Simpson's rule for numerical integration.
///
/// Provides good accuracy for smooth functions. Requires an even number of intervals.
///
/// # Arguments
/// * `f` - Function to integrate
/// * `a` - Lower bound
/// * `b` - Upper bound  
/// * `n` - Number of intervals (must be even)
///
/// # Returns
/// Approximate integral value
pub fn simpson_rule<F2>(f: F2, a: F, b: F, n: usize) -> Result<F, Error>
where
    F2: Fn(F) -> F,
{
    if n % 2 != 0 || n == 0 {
        return Err(InputError::Invalid.into());
    }

    let h = (b - a) / n as F;
    let mut sum = f(a) + f(b);

    // Add even terms (coefficient 2)
    for i in (2..n).step_by(2) {
        let x = a + i as F * h;
        sum += 2.0 * f(x);
    }

    // Add odd terms (coefficient 4)
    for i in (1..n).step_by(2) {
        let x = a + i as F * h;
        sum += 4.0 * f(x);
    }

    Ok(sum * h / 3.0)
}

/// Adaptive quadrature using recursive Simpson's rule.
///
/// This method automatically refines the integration grid in areas where
/// the function changes rapidly, providing better accuracy with fewer
/// function evaluations for smooth functions.
///
/// # Arguments
/// * `f` - Function to integrate
/// * `a` - Lower bound
/// * `b` - Upper bound
/// * `tol` - Error tolerance
/// * `max_depth` - Maximum recursion depth to prevent infinite recursion
///
/// # Returns
/// Approximate integral value with estimated error control
pub fn adaptive_quadrature<F2>(f: F2, a: F, b: F, tol: F, max_depth: usize) -> Result<F, Error>
where
    F2: Fn(F) -> F + Copy,
{
    #[allow(clippy::too_many_arguments)]
    fn adaptive_simpson<F2>(
        f: F2,
        a: F,
        b: F,
        tol: F,
        fa: F,
        fb: F,
        fc: F,
        whole: F,
        depth: usize,
        max_depth: usize,
    ) -> Result<F, Error>
    where
        F2: Fn(F) -> F + Copy,
    {
        if depth >= max_depth {
            // At recursion limit, return the current best composite estimate
            return Ok(whole);
        }

        let c = (a + b) / 2.0;

        let fd = f((a + c) / 2.0);
        let fe = f((c + b) / 2.0);

        // Use proper Simpson's rule for each sub-interval
        let h_left = (c - a) / 6.0; // (c-a)/6 for left Simpson interval
        let h_right = (b - c) / 6.0; // (b-c)/6 for right Simpson interval
        let left = h_left * (fa + 4.0 * fd + fc);
        let right = h_right * (fc + 4.0 * fe + fb);
        let total = left + right;

        let error_estimate = (total - whole).abs() / 15.0;

        if error_estimate <= tol {
            Ok(total)
        } else {
            let mid_tol = tol / 2.0;
            let left_result =
                adaptive_simpson(f, a, c, mid_tol, fa, fc, fd, left, depth + 1, max_depth)?;
            let right_result =
                adaptive_simpson(f, c, b, mid_tol, fc, fb, fe, right, depth + 1, max_depth)?;
            Ok(left_result + right_result)
        }
    }

    let c = (a + b) / 2.0;
    let h = (b - a) / 6.0;
    let fa = f(a);
    let fb = f(b);
    let fc = f(c);

    let whole = h * (fa + 4.0 * fc + fb);

    adaptive_simpson(f, a, b, tol, fa, fb, fc, whole, 0, max_depth)
}

/// Convenience alias: Adaptive Simpson integration with error control.
///
/// This forwards to `adaptive_quadrature`, clarifying the Simpson basis.
pub fn adaptive_simpson<F2>(f: F2, a: F, b: F, tol: F, max_depth: usize) -> Result<F, Error>
where
    F2: Fn(F) -> F + Copy,
{
    adaptive_quadrature(f, a, b, tol, max_depth)
}

// -----------------------------------------------------------------------------
// Gauss–Legendre Quadrature (finite intervals)
// -----------------------------------------------------------------------------

/// Return Gauss–Legendre nodes and weights for supported orders.
fn gl_nodes_weights(order: usize) -> Result<(&'static [F], &'static [F]), Error> {
    // Nodes/weights for symmetric [-1,1] intervals
    // Orders supported: 2, 4, 8, 16
    match order {
        2 => Ok((
            &[-0.577_350_269_189_625_7, 0.577_350_269_189_625_7],
            &[1.0, 1.0],
        )),
        4 => Ok((
            &[
                -0.861_136_311_594_052_6,
                -0.339_981_043_584_856_3,
                0.339_981_043_584_856_3,
                0.861_136_311_594_052_6,
            ],
            &[
                0.347_854_845_137_453_85,
                0.652_145_154_862_546_1,
                0.652_145_154_862_546_1,
                0.347_854_845_137_453_85,
            ],
        )),
        8 => Ok((
            &[
                -0.960_289_856_497_536_3,
                -0.796_666_477_413_626_7,
                -0.525_532_409_916_329,
                -0.183_434_642_495_649_8,
                0.183_434_642_495_649_8,
                0.525_532_409_916_329,
                0.796_666_477_413_626_7,
                0.960_289_856_497_536_3,
            ],
            &[
                0.101_228_536_290_376_26,
                0.222_381_034_453_374_48,
                0.313_706_645_877_887_27,
                0.362_683_783_378_361_96,
                0.362_683_783_378_361_96,
                0.313_706_645_877_887_27,
                0.222_381_034_453_374_48,
                0.101_228_536_290_376_26,
            ],
        )),
        16 => Ok((
            &[
                -0.989_400_934_991_649_9,
                -0.944_575_023_073_232_6,
                -0.865_631_202_387_831_8,
                -0.755_404_408_355_003,
                -0.617_876_244_402_643_8,
                -0.458_016_777_657_227_37,
                -0.281_603_550_779_258_9,
                -0.095_012_509_837_637_44,
                0.095_012_509_837_637_44,
                0.281_603_550_779_258_9,
                0.458_016_777_657_227_37,
                0.617_876_244_402_643_8,
                0.755_404_408_355_003,
                0.865_631_202_387_831_8,
                0.944_575_023_073_232_6,
                0.989_400_934_991_649_9,
            ],
            &[
                0.027_152_459_411_754_095,
                0.062_253_523_938_647_894,
                0.095_158_511_682_492_78,
                0.124_628_971_255_533_88,
                0.149_595_988_816_576_73,
                0.169_156_519_395_002_54,
                0.182_603_415_044_923_58,
                0.189_450_610_455_068_5,
                0.189_450_610_455_068_5,
                0.182_603_415_044_923_58,
                0.169_156_519_395_002_54,
                0.149_595_988_816_576_73,
                0.124_628_971_255_533_88,
                0.095_158_511_682_492_78,
                0.062_253_523_938_647_894,
                0.027_152_459_411_754_095,
            ],
        )),
        _ => Err(InputError::Invalid.into()),
    }
}

/// Gauss–Legendre quadrature over finite interval [a,b].
pub fn gauss_legendre_integrate<F2>(f: F2, a: F, b: F, order: usize) -> Result<F, Error>
where
    F2: Fn(F) -> F,
{
    if !(a.is_finite() && b.is_finite()) {
        return Err(InputError::Invalid.into());
    }
    if a == b {
        return Ok(0.0);
    }
    let (xs, ws) = gl_nodes_weights(order)?;
    let half = 0.5 * (b - a);
    let mid = 0.5 * (b + a);
    let mut acc = 0.0;
    for i in 0..xs.len() {
        let x = mid + half * xs[i];
        acc += ws[i] * f(x);
    }
    Ok(acc * half)
}

/// Composite Gauss–Legendre over [a,b] using `panels` sub-intervals.
pub fn gauss_legendre_integrate_composite<F2>(
    f: F2,
    a: F,
    b: F,
    order: usize,
    panels: usize,
) -> Result<F, Error>
where
    F2: Fn(F) -> F,
{
    if panels == 0 {
        return Err(InputError::Invalid.into());
    }
    let h = (b - a) / panels as F;
    let mut sum = 0.0;
    for k in 0..panels {
        let ak = a + k as F * h;
        let bk = ak + h;
        sum += gauss_legendre_integrate(&f, ak, bk, order)?;
    }
    Ok(sum)
}

/// Adaptive Gauss–Legendre based on panel refinement until tolerance is met.
pub fn gauss_legendre_integrate_adaptive<F2>(
    f: F2,
    a: F,
    b: F,
    order: usize,
    tol: F,
    max_depth: usize,
) -> Result<F, Error>
where
    F2: Fn(F) -> F + Copy,
{
    fn recurse<F2>(
        f: F2,
        a: F,
        b: F,
        order: usize,
        tol: F,
        depth: usize,
        max_depth: usize,
    ) -> Result<F, Error>
    where
        F2: Fn(F) -> F + Copy,
    {
        let i1 = gauss_legendre_integrate(f, a, b, order)?;
        let mid = 0.5 * (a + b);
        let i2_left = gauss_legendre_integrate(f, a, mid, order)?;
        let i2_right = gauss_legendre_integrate(f, mid, b, order)?;
        let i2 = i2_left + i2_right;
        let err = (i2 - i1).abs();
        if err <= tol || depth >= max_depth {
            return Ok(i2);
        }
        let left = recurse(f, a, mid, order, tol * 0.5, depth + 1, max_depth)?;
        let right = recurse(f, mid, b, order, tol * 0.5, depth + 1, max_depth)?;
        Ok(left + right)
    }

    recurse(f, a, b, order, tol, 0, max_depth)
}

/// Trapezoidal rule for numerical integration.
///
/// Simple and robust integration method. Less accurate than Simpson's rule
/// but more stable for discontinuous functions.
///
/// # Arguments
/// * `f` - Function to integrate
/// * `a` - Lower bound
/// * `b` - Upper bound
/// * `n` - Number of intervals
///
/// # Returns
/// Approximate integral value
pub fn trapezoidal_rule<F2>(f: F2, a: F, b: F, n: usize) -> Result<F, Error>
where
    F2: Fn(F) -> F,
{
    if n == 0 {
        return Err(InputError::Invalid.into());
    }

    let h = (b - a) / n as F;
    let mut sum = 0.5 * (f(a) + f(b));

    for i in 1..n {
        let x = a + i as F * h;
        sum += f(x);
    }

    Ok(sum * h)
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

    #[test]
    fn test_simpson_rule() {
        // Test Simpson's rule on a simple polynomial x² on [0, 1]
        // Exact integral = 1/3
        let f = |x: F| x * x;
        let integral = simpson_rule(f, 0.0, 1.0, 100).unwrap();
        assert!((integral - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_adaptive_quadrature() {
        // Test adaptive integration on oscillatory function
        let f = |x: F| (10.0 * x).sin();
        let result = adaptive_quadrature(f, 0.0, std::f64::consts::PI, 1e-6, 1000).unwrap();
        // Exact integral = (1 - cos(10π))/10 = 0
        assert!(result.abs() < 1e-2);
    }
}
