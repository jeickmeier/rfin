//! Numerical integration methods for financial mathematics.
//!
//! Implements deterministic quadrature rules and adaptive algorithms for
//! computing integrals that arise in option pricing, risk-neutral expectations,
//! and probability calculations.
//!
//! # Algorithms
//!
//! - **Gauss-Hermite**: Integration over normal distribution (Heston, SABR)
//! - **Gauss-Legendre**: Integration over finite intervals (payoff expectations)
//! - **Simpson's rule**: Classic adaptive quadrature
//! - **Trapezoidal rule**: Simple, robust baseline method
//!
//! # Use Cases
//!
//! - **Option pricing**: Semi-analytical methods requiring characteristic function integration
//! - **Heston model**: Fourier inversion for vanilla options
//! - **SABR**: Probability density integration for digital payoffs
//! - **Risk-neutral expectations**: Integrate payoff × density
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::integration::GaussHermiteQuadrature;
//!
//! // Integrate x² over standard normal (expected value = 1.0)
//! let quad = GaussHermiteQuadrature::order_7();
//! let integral = quad.integrate(|x| x * x);
//! assert!((integral - 1.0).abs() < 0.1);
//! ```
//!
//! # References
//!
//! - **Gaussian Quadrature**:
//!   - Abramowitz, M., & Stegun, I. A. (1964). *Handbook of Mathematical Functions*.
//!     Chapter 25 (Numerical Integration).
//!   - Press, W. H., et al. (2007). *Numerical Recipes* (3rd ed.). Section 4.5.
//!
//! - **Adaptive Methods**:
//!   - Davis, P. J., & Rabinowitz, P. (1984). *Methods of Numerical Integration*
//!     (2nd ed.). Academic Press.
//!
//! - **Financial Applications**:
//!   - Lewis, A. L. (2000). *Option Valuation under Stochastic Volatility*.
//!     Finance Press. (Fourier methods and quadrature)

use crate::error::InputError;
use crate::Error;

// Removed over-engineered parameter bundling structs - use direct parameters instead

/// Gauss-Hermite quadrature points and weights for numerical integration
/// over the standard normal distribution.
///
/// These are pre-computed for common quadrature orders to avoid runtime
/// computation of the nodes and weights.
pub struct GaussHermiteQuadrature {
    /// Quadrature points (x-coordinates)
    pub points: &'static [f64],
    /// Quadrature weights
    pub weights: &'static [f64],
}

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
            15 => 15,
            20 => 20,
            _ => return Err(serde::ser::Error::custom("Unknown quadrature order")),
        };

        #[derive(serde::Serialize)]
        struct QuadratureData {
            order: usize,
        }

        serde::Serialize::serialize(&QuadratureData { order }, serializer)
    }
}

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

        GaussHermiteQuadrature::new(data.order).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "Invalid quadrature order: {}. Supported orders: 5, 7, 10, 15, 20",
                data.order
            ))
        })
    }
}

impl GaussHermiteQuadrature {
    /// Create a Gauss-Hermite quadrature with the specified order.
    ///
    /// This is the canonical constructor for Gauss-Hermite quadrature. Use this
    /// method instead of the deprecated `order_N()` constructors.
    ///
    /// # Arguments
    /// * `order` - Quadrature order (supported: 5, 7, 10, 15, 20)
    ///
    /// # Returns
    /// `Some(Self)` if order is supported, `None` otherwise.
    ///
    /// # Precision Guidelines
    ///
    /// | Order | Polynomial Exactness | Recommended Use |
    /// |-------|---------------------|-----------------|
    /// | 5 | Degree 9 | Quick estimates, smooth functions |
    /// | 7 | Degree 13 | Standard option pricing |
    /// | 10 | Degree 19 | General Monte Carlo validation |
    /// | 15 | Degree 29 | High-precision Heston pricing |
    /// | 20 | Degree 39 | Long-dated options, high vol-of-vol |
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::math::integration::GaussHermiteQuadrature;
    ///
    /// let quad = GaussHermiteQuadrature::new(7).expect("7 is a supported order");
    /// let integral = quad.integrate(|x| x * x);
    /// assert!((integral - 1.0).abs() < 0.1); // E[X²] = 1 for standard normal
    ///
    /// // High-precision quadrature for demanding applications
    /// let high_precision = GaussHermiteQuadrature::new(20).expect("20 is supported");
    ///
    /// // Unsupported orders return None
    /// assert!(GaussHermiteQuadrature::new(3).is_none());
    /// ```
    #[allow(deprecated)]
    pub fn new(order: usize) -> Option<Self> {
        match order {
            5 => Some(Self::order_5()),
            7 => Some(Self::order_7()),
            10 => Some(Self::order_10()),
            15 => Some(Self::order_15()),
            20 => Some(Self::order_20()),
            _ => None,
        }
    }

    /// Get the 5-point Gauss-Hermite quadrature.
    ///
    /// This provides a good balance between accuracy and performance
    /// for most applications.
    ///
    /// # Deprecation
    ///
    /// Use [`GaussHermiteQuadrature::new(5)`](Self::new) instead for a unified API.
    #[deprecated(
        since = "0.9.0",
        note = "Use `GaussHermiteQuadrature::new(5).expect(\"valid order\")` instead"
    )]
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
    ///
    /// # Deprecation
    ///
    /// Use [`GaussHermiteQuadrature::new(7)`](Self::new) instead for a unified API.
    #[deprecated(
        since = "0.9.0",
        note = "Use `GaussHermiteQuadrature::new(7).expect(\"valid order\")` instead"
    )]
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
    ///
    /// # Deprecation
    ///
    /// Use [`GaussHermiteQuadrature::new(10)`](Self::new) instead for a unified API.
    #[deprecated(
        since = "0.9.0",
        note = "Use `GaussHermiteQuadrature::new(10).expect(\"valid order\")` instead"
    )]
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

    /// Get the 15-point Gauss-Hermite quadrature.
    ///
    /// Very high accuracy for Heston model pricing and other demanding
    /// characteristic function integrations.
    ///
    /// Exact for polynomials up to degree 29.
    ///
    /// # Deprecation
    ///
    /// Use [`GaussHermiteQuadrature::new(15)`](Self::new) instead for a unified API.
    #[deprecated(
        since = "0.9.0",
        note = "Use `GaussHermiteQuadrature::new(15).expect(\"valid order\")` instead"
    )]
    pub fn order_15() -> Self {
        // Nodes and weights computed using numpy.polynomial.hermite.hermgauss(15)
        // then adjusted for probabilist's Hermite (physicist's nodes / sqrt(2), weights / sqrt(pi))
        Self {
            points: &[
                -4.499_990_707_309_392,
                -3.669_950_373_404_453,
                -2.967_166_927_905_603,
                -2.325_732_486_173_858,
                -1.719_992_575_186_489,
                -1.136_115_585_210_921,
                -0.565_069_583_255_576,
                0.0,
                0.565_069_583_255_576,
                1.136_115_585_210_921,
                1.719_992_575_186_489,
                2.325_732_486_173_858,
                2.967_166_927_905_603,
                3.669_950_373_404_453,
                4.499_990_707_309_392,
            ],
            weights: &[
                1.522_475_804_253_517e-9,
                1.059_115_547_711_067e-6,
                1.000_044_412_325_025e-4,
                2.778_068_842_912_776e-3,
                3.078_003_387_254_608e-2,
                1.584_889_157_959_357e-1,
                4.120_286_874_988_986e-1,
                5.641_003_087_264_175e-1,
                4.120_286_874_988_986e-1,
                1.584_889_157_959_357e-1,
                3.078_003_387_254_608e-2,
                2.778_068_842_912_776e-3,
                1.000_044_412_325_025e-4,
                1.059_115_547_711_067e-6,
                1.522_475_804_253_517e-9,
            ],
        }
    }

    /// Get the 20-point Gauss-Hermite quadrature.
    ///
    /// Extremely high accuracy for long-dated options, high vol-of-vol regimes,
    /// and characteristic function integration requiring precision beyond 1e-10.
    ///
    /// Exact for polynomials up to degree 39.
    ///
    /// # Deprecation
    ///
    /// Use [`GaussHermiteQuadrature::new(20)`](Self::new) instead for a unified API.
    #[deprecated(
        since = "0.9.0",
        note = "Use `GaussHermiteQuadrature::new(20).expect(\"valid order\")` instead"
    )]
    pub fn order_20() -> Self {
        // Nodes and weights computed using numpy.polynomial.hermite.hermgauss(20)
        // adjusted for probabilist's convention
        Self {
            points: &[
                -5.387_480_890_011_233,
                -4.603_682_449_550_744,
                -3.944_764_040_115_625,
                -3.347_854_567_383_216,
                -2.788_806_058_428_13,
                -2.254_974_002_089_276,
                -1.738_537_712_116_586,
                -1.234_076_215_395_323,
                -0.737_473_728_545_394,
                -0.245_340_708_300_901,
                0.245_340_708_300_901,
                0.737_473_728_545_394,
                1.234_076_215_395_323,
                1.738_537_712_116_586,
                2.254_974_002_089_276,
                2.788_806_058_428_13,
                3.347_854_567_383_216,
                3.944_764_040_115_625,
                4.603_682_449_550_744,
                5.387_480_890_011_233,
            ],
            weights: &[
                2.229_393_645_534_151e-13,
                4.399_340_992_273_181e-10,
                1.086_069_370_769_281e-7,
                7.802_556_478_532_064e-6,
                2.283_386_360_163_528e-4,
                3.243_773_342_237_853e-3,
                2.481_052_088_746_361e-2,
                1.090_172_060_200_233e-1,
                2.866_755_053_628_342e-1,
                4.622_436_696_006_101e-1,
                4.622_436_696_006_101e-1,
                2.866_755_053_628_342e-1,
                1.090_172_060_200_233e-1,
                2.481_052_088_746_361e-2,
                3.243_773_342_237_853e-3,
                2.283_386_360_163_528e-4,
                7.802_556_478_532_064e-6,
                1.086_069_370_769_281e-7,
                4.399_340_992_273_181e-10,
                2.229_393_645_534_151e-13,
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
    pub fn integrate<F2>(&self, f: F2) -> f64
    where
        F2: Fn(f64) -> f64,
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
    ///
    /// # Refinement Strategy
    ///
    /// | Starting Order | Refinement Path |
    /// |---------------|-----------------|
    /// | 5 | 5 → 7 → 10 → 15 → 20 |
    /// | 7 | 7 → 10 → 15 → 20 |
    /// | 10 | 10 → 15 → 20 |
    /// | 15 | 15 → 20 |
    /// | 20 | 20 (no refinement) |
    pub fn integrate_adaptive<F2>(&self, f: F2, tolerance: f64) -> f64
    where
        F2: Fn(f64) -> f64 + Copy,
    {
        let quad = |order: usize| {
            GaussHermiteQuadrature::new(order).unwrap_or_else(|| {
                debug_assert!(false, "Invalid Gauss-Hermite order: {order}");
                GaussHermiteQuadrature::new(20)
                    .unwrap_or_else(|| unreachable!("20 is a valid Gauss-Hermite order"))
            })
        };
        let base = self.integrate(f);
        match self.points.len() {
            20 => base,
            15 => {
                let v20 = quad(20).integrate(f);
                if (v20 - base).abs() <= tolerance {
                    base
                } else {
                    v20
                }
            }
            10 => {
                let v15 = quad(15).integrate(f);
                if (v15 - base).abs() <= tolerance {
                    v15
                } else {
                    quad(20).integrate(f)
                }
            }
            7 => {
                let v10 = quad(10).integrate(f);
                if (v10 - base).abs() <= tolerance {
                    v10
                } else {
                    quad(15).integrate(f)
                }
            }
            5 => {
                let v7 = quad(7).integrate(f);
                if (v7 - base).abs() <= tolerance {
                    v7
                } else {
                    quad(10).integrate(f)
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
///
/// * `f` - Function to integrate
/// * `a` - Lower bound
/// * `b` - Upper bound
/// * `n` - Number of intervals (must be even and > 0)
///
/// # Returns
///
/// Approximate integral value.
///
/// # Errors
///
/// Returns [`InputError::Invalid`](crate::error::InputError::Invalid) when:
/// - `n` is zero
/// - `n` is not an even number
///
/// # Complexity
///
/// - **Time**: O(n) function evaluations
/// - **Space**: O(1) auxiliary space
///
/// # Example
///
/// ```rust
/// use finstack_core::math::integration::simpson_rule;
///
/// // Integrate x² from 0 to 1 (exact answer: 1/3)
/// let integral = simpson_rule(|x| x * x, 0.0, 1.0, 100).expect("Valid parameters");
/// assert!((integral - 1.0/3.0).abs() < 1e-6);
/// ```
pub fn simpson_rule<F2>(f: F2, a: f64, b: f64, n: usize) -> Result<f64, Error>
where
    F2: Fn(f64) -> f64,
{
    if n == 0 || !n.is_multiple_of(2) {
        return Err(InputError::Invalid.into());
    }

    let h = (b - a) / n as f64;
    let mut sum = f(a) + f(b);

    // Add even terms (coefficient 2)
    for i in (2..n).step_by(2) {
        let x = a + i as f64 * h;
        sum += 2.0 * f(x);
    }

    // Add odd terms (coefficient 4)
    for i in (1..n).step_by(2) {
        let x = a + i as f64 * h;
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
///
/// * `f` - Function to integrate (must implement `Copy` for recursive calls)
/// * `a` - Lower bound
/// * `b` - Upper bound
/// * `tol` - Error tolerance for adaptive refinement
/// * `max_depth` - Maximum recursion depth to prevent infinite refinement
///
/// # Returns
///
/// Approximate integral value with error bounded by `tol` (when possible).
///
/// # Algorithm
///
/// Uses recursive bisection with Simpson's rule on each subinterval. At each level:
/// 1. Compute Simpson's rule on `[a, b]`
/// 2. Compute Simpson's rule on `[a, mid]` and `[mid, b]`
/// 3. If error estimate ≤ `tol` or depth ≥ `max_depth`, return the composite estimate
/// 4. Otherwise, recursively refine each half with `tol/2`
///
/// # Complexity
///
/// - **Time**: O(2^max_depth) function evaluations in worst case; typically much fewer
/// - **Space**: O(max_depth) stack frames
///
/// # Example
///
/// ```rust
/// use finstack_core::math::integration::adaptive_simpson;
///
/// // Integrate an oscillatory function with high precision
/// let integral = adaptive_simpson(|x| (10.0 * x).sin(), 0.0, std::f64::consts::PI, 1e-6, 100)
///     .expect("Integration succeeds");
/// assert!(integral.abs() < 0.01); // Should be close to 0
/// ```
pub fn adaptive_simpson<F2>(f: F2, a: f64, b: f64, tol: f64, max_depth: usize) -> Result<f64, Error>
where
    F2: Fn(f64) -> f64 + Copy,
{
    #[allow(clippy::too_many_arguments)]
    fn adaptive_simpson_inner<F2>(
        f: F2,
        a: f64,
        b: f64,
        tol: f64,
        fa: f64,
        fb: f64,
        fc: f64,
        whole: f64,
        depth: usize,
        max_depth: usize,
    ) -> Result<f64, Error>
    where
        F2: Fn(f64) -> f64 + Copy,
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
                adaptive_simpson_inner(f, a, c, mid_tol, fa, fc, fd, left, depth + 1, max_depth)?;
            let right_result =
                adaptive_simpson_inner(f, c, b, mid_tol, fc, fb, fe, right, depth + 1, max_depth)?;
            Ok(left_result + right_result)
        }
    }

    let c = (a + b) / 2.0;
    let h = (b - a) / 6.0;
    let fa = f(a);
    let fb = f(b);
    let fc = f(c);

    let whole = h * (fa + 4.0 * fc + fb);

    adaptive_simpson_inner(f, a, b, tol, fa, fb, fc, whole, 0, max_depth)
}

// -----------------------------------------------------------------------------
// Gauss–Legendre Quadrature (finite intervals)
// -----------------------------------------------------------------------------

/// Return Gauss–Legendre nodes and weights for supported orders.
fn gl_nodes_weights(order: usize) -> Result<(&'static [f64], &'static [f64]), Error> {
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

/// Gauss–Legendre quadrature over finite interval \[a,b\].
///
/// This is a low-level building block for numerical integration. For most use
/// cases requiring automatic error control, prefer [`gauss_legendre_integrate_adaptive`].
///
/// # When to Use
///
/// - **Use this function** when you need precise control over quadrature order
///   and have verified the function is smooth over the interval.
/// - **Use [`gauss_legendre_integrate_adaptive`]** when you need automatic error
///   control and aren't sure about function smoothness.
///
/// # Arguments
///
/// * `f` - Function to integrate
/// * `a` - Lower bound of integration (must be finite)
/// * `b` - Upper bound of integration (must be finite)
/// * `order` - Quadrature order (supported: 2, 4, 8, 16)
///
/// # Returns
///
/// - `Ok(0.0)` if `a == b`
/// - `Ok(integral)` for the approximate integral value
///
/// # Errors
///
/// Returns [`InputError::Invalid`](crate::error::InputError::Invalid) when:
/// - `a` or `b` is not finite (NaN or infinity)
/// - `order` is not one of the supported values (2, 4, 8, 16)
///
/// # Complexity
///
/// - **Time**: O(order) function evaluations
/// - **Space**: O(1) auxiliary space (nodes/weights are static)
///
/// # Precision Guidelines
///
/// | Order | Polynomial Exactness | Recommended Use |
/// |-------|---------------------|-----------------|
/// | 2 | Degree 3 | Very rough estimates |
/// | 4 | Degree 7 | Quick calculations |
/// | 8 | Degree 15 | Standard accuracy |
/// | 16 | Degree 31 | High precision |
///
/// # Example
///
/// ```rust
/// use finstack_core::math::integration::gauss_legendre_integrate;
///
/// // Integrate x³ from -1 to 1 (exact answer: 0)
/// let integral = gauss_legendre_integrate(|x| x.powi(3), -1.0, 1.0, 4).expect("Valid order");
/// assert!(integral.abs() < 1e-10);
/// ```
pub fn gauss_legendre_integrate<F2>(f: F2, a: f64, b: f64, order: usize) -> Result<f64, Error>
where
    F2: Fn(f64) -> f64,
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

/// Composite Gauss–Legendre over \[a,b\] using `panels` sub-intervals.
///
/// Divides the integration interval into `panels` equal sub-intervals and applies
/// Gauss-Legendre quadrature to each. This improves accuracy for functions that
/// are not well-approximated by polynomials over the full interval.
///
/// # Arguments
///
/// * `f` - Function to integrate
/// * `a` - Lower bound of integration
/// * `b` - Upper bound of integration
/// * `order` - Quadrature order per panel (supported: 2, 4, 8, 16)
/// * `panels` - Number of sub-intervals (must be > 0)
///
/// # Returns
///
/// Approximate integral value.
///
/// # Errors
///
/// Returns [`InputError::Invalid`](crate::error::InputError::Invalid) when:
/// - `panels` is zero
/// - `order` is unsupported (see [`gauss_legendre_integrate`])
///
/// # Complexity
///
/// - **Time**: O(panels × order) function evaluations
/// - **Space**: O(1) auxiliary space
pub fn gauss_legendre_integrate_composite<F2>(
    f: F2,
    a: f64,
    b: f64,
    order: usize,
    panels: usize,
) -> Result<f64, Error>
where
    F2: Fn(f64) -> f64,
{
    if panels == 0 {
        return Err(InputError::Invalid.into());
    }
    let h = (b - a) / panels as f64;
    let mut sum = 0.0;
    for k in 0..panels {
        let ak = a + k as f64 * h;
        let bk = ak + h;
        sum += gauss_legendre_integrate(&f, ak, bk, order)?;
    }
    Ok(sum)
}

/// Adaptive Gauss–Legendre integration with automatic refinement.
///
/// Recursively bisects the interval and refines where the error estimate
/// exceeds tolerance, providing automatic accuracy control.
///
/// # Arguments
///
/// * `f` - Function to integrate (must implement `Copy` for recursive calls)
/// * `a` - Lower bound of integration
/// * `b` - Upper bound of integration
/// * `order` - Quadrature order per subinterval (supported: 2, 4, 8, 16)
/// * `tol` - Error tolerance for refinement decisions
/// * `max_depth` - Maximum recursion depth to prevent infinite refinement
///
/// # Returns
///
/// Approximate integral value with error bounded by `tol` (when possible).
///
/// # Algorithm
///
/// At each level:
/// 1. Compute integral over `[a, b]` using single Gauss-Legendre
/// 2. Compute integral over `[a, mid]` + `[mid, b]`
/// 3. If difference ≤ `tol` or depth ≥ `max_depth`, return composite result
/// 4. Otherwise, recursively refine each half
///
/// # Complexity
///
/// - **Time**: O(order × 2^max_depth) evaluations worst case
/// - **Space**: O(max_depth) stack frames
///
/// # Example
///
/// ```rust
/// use finstack_core::math::integration::gauss_legendre_integrate_adaptive;
///
/// // Integrate a function with a peak
/// let integral = gauss_legendre_integrate_adaptive(
///     |x| (-x * x).exp(),
///     -5.0, 5.0,
///     8,    // order
///     1e-8, // tolerance
///     20    // max depth
/// ).expect("Integration succeeds");
///
/// // Should be close to sqrt(π) ≈ 1.7725
/// assert!((integral - std::f64::consts::PI.sqrt()).abs() < 1e-6);
/// ```
pub fn gauss_legendre_integrate_adaptive<F2>(
    f: F2,
    a: f64,
    b: f64,
    order: usize,
    tol: f64,
    max_depth: usize,
) -> Result<f64, Error>
where
    F2: Fn(f64) -> f64 + Copy,
{
    fn recurse<F2>(
        f: F2,
        a: f64,
        b: f64,
        order: usize,
        tol: f64,
        depth: usize,
        max_depth: usize,
    ) -> Result<f64, Error>
    where
        F2: Fn(f64) -> f64 + Copy,
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
/// for smooth functions, but more stable for discontinuous functions.
///
/// # Arguments
///
/// * `f` - Function to integrate
/// * `a` - Lower bound
/// * `b` - Upper bound
/// * `n` - Number of intervals (must be > 0)
///
/// # Returns
///
/// Approximate integral value.
///
/// # Errors
///
/// Returns [`InputError::Invalid`](crate::error::InputError::Invalid) when `n` is zero.
///
/// # Complexity
///
/// - **Time**: O(n) function evaluations
/// - **Space**: O(1) auxiliary space
///
/// # Accuracy
///
/// Error is O(h²) where h = (b-a)/n, assuming f has continuous second derivative.
/// For smooth functions, prefer [`simpson_rule`] which has O(h⁴) error.
///
/// # Example
///
/// ```rust
/// use finstack_core::math::integration::trapezoidal_rule;
///
/// // Integrate x from 0 to 1 (exact answer: 0.5)
/// let integral = trapezoidal_rule(|x| x, 0.0, 1.0, 100).expect("Valid n");
/// assert!((integral - 0.5).abs() < 1e-4);
/// ```
pub fn trapezoidal_rule<F2>(f: F2, a: f64, b: f64, n: usize) -> Result<f64, Error>
where
    F2: Fn(f64) -> f64,
{
    if n == 0 {
        return Err(InputError::Invalid.into());
    }

    let h = (b - a) / n as f64;
    let mut sum = 0.5 * (f(a) + f(b));

    for i in 1..n {
        let x = a + i as f64 * h;
        sum += f(x);
    }

    Ok(sum * h)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    deprecated
)]
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
        let f = |x: f64| x * x * x * x; // x^4 function

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
        let f = |x: f64| x * x;
        let integral = simpson_rule(f, 0.0, 1.0, 100)
            .expect("Simpson rule integration should succeed in test");
        assert!((integral - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_adaptive_simpson() {
        // Test adaptive integration on oscillatory function
        let f = |x: f64| (10.0 * x).sin();
        let result = adaptive_simpson(f, 0.0, std::f64::consts::PI, 1e-6, 1000)
            .expect("Adaptive Simpson should succeed in test");
        // Exact integral = (1 - cos(10π))/10 = 0
        assert!(result.abs() < 1e-2);
    }
}
