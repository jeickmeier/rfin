use crate::core::math::callable::JsCallable;
use crate::core::error::js_error;
use finstack_core::math::integration as core_integration;
use finstack_core::math::integration::GaussHermiteQuadrature;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = GaussHermiteQuadrature)]
pub struct JsGaussHermiteQuadrature {
    inner: GaussHermiteQuadrature,
}

impl JsGaussHermiteQuadrature {
    fn from_inner(inner: GaussHermiteQuadrature) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = GaussHermiteQuadrature)]
impl JsGaussHermiteQuadrature {
    /// Create a Gauss-Hermite quadrature rule for integrating functions weighted by e^(-x²).
    ///
    /// Gauss-Hermite quadrature is optimal for integrals of the form:
    /// ∫_{-∞}^∞ f(x) * e^(-x²) dx
    ///
    /// @param {number} order - Quadrature order (5, 7, or 10)
    /// @returns {GaussHermiteQuadrature} Quadrature rule with specified order
    /// @throws {Error} If order is not 5, 7, or 10
    ///
    /// @example
    /// ```javascript
    /// // 5-point rule for integrating standard normal density functions
    /// const quad5 = new GaussHermiteQuadrature(5);
    /// console.log(quad5.order);  // 5
    ///
    /// // Integrate x² * e^(-x²) from -∞ to ∞ (should be √π/2)
    /// const result = quad5.integrate(x => x * x);
    /// console.log(result);  // ~0.886227 (√π/2)
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(order: usize) -> Result<JsGaussHermiteQuadrature, JsValue> {
        match order {
            5 => Ok(Self::from_inner(GaussHermiteQuadrature::order_5())),
            7 => Ok(Self::from_inner(GaussHermiteQuadrature::order_7())),
            10 => Ok(Self::from_inner(GaussHermiteQuadrature::order_10())),
            _ => Err(js_error("Supported orders are 5, 7, or 10")),
        }
    }

    /// Create a 5-point Gauss-Hermite quadrature rule.
    ///
    /// @returns {GaussHermiteQuadrature} 5-point quadrature rule
    ///
    /// @example
    /// ```javascript
    /// const quad = GaussHermiteQuadrature.order5();
    /// console.log(quad.order);  // 5
    /// ```
    #[wasm_bindgen(js_name = order5)]
    pub fn order_5() -> JsGaussHermiteQuadrature {
        Self::from_inner(GaussHermiteQuadrature::order_5())
    }

    /// Create a 7-point Gauss-Hermite quadrature rule.
    ///
    /// @returns {GaussHermiteQuadrature} 7-point quadrature rule
    #[wasm_bindgen(js_name = order7)]
    pub fn order_7() -> JsGaussHermiteQuadrature {
        Self::from_inner(GaussHermiteQuadrature::order_7())
    }

    /// Create a 10-point Gauss-Hermite quadrature rule.
    ///
    /// @returns {GaussHermiteQuadrature} 10-point quadrature rule
    #[wasm_bindgen(js_name = order10)]
    pub fn order_10() -> JsGaussHermiteQuadrature {
        Self::from_inner(GaussHermiteQuadrature::order_10())
    }

    /// Number of quadrature points in this rule.
    ///
    /// @type {number}
    /// @readonly
    #[wasm_bindgen(getter)]
    pub fn order(&self) -> usize {
        self.inner.points.len()
    }

    /// Abscissas (x-coordinates) of the quadrature points.
    ///
    /// @returns {Array<number>} Array of quadrature point coordinates
    ///
    /// @example
    /// ```javascript
    /// const quad = new GaussHermiteQuadrature(5);
    /// const points = quad.points;
    /// console.log(points.length);  // 5
    /// console.log(points[0]);      // ~-2.02018 (first abscissa)
    /// ```
    #[wasm_bindgen(js_name = points)]
    pub fn points(&self) -> Vec<f64> {
        self.inner.points.to_vec()
    }

    /// Weights for each quadrature point.
    ///
    /// @returns {Array<number>} Array of quadrature weights
    ///
    /// @example
    /// ```javascript
    /// const quad = new GaussHermiteQuadrature(5);
    /// const weights = quad.weights;
    /// console.log(weights.length);  // 5
    /// console.log(weights[0]);      // ~0.019953 (first weight)
    /// ```
    #[wasm_bindgen(js_name = weights)]
    pub fn weights(&self) -> Vec<f64> {
        self.inner.weights.to_vec()
    }

    /// Integrate a function using Gauss-Hermite quadrature.
    ///
    /// Approximates ∫_{-∞}^∞ f(x) * e^(-x²) dx using the quadrature rule.
    /// The function f(x) should not include the e^(-x²) weight.
    ///
    /// @param {Function} func - Function to integrate (takes number, returns number)
    /// @returns {number} Approximate value of the weighted integral
    /// @throws {Error} If function evaluation fails
    ///
    /// @example
    /// ```javascript
    /// const quad = new GaussHermiteQuadrature(7);
    ///
    /// // Integrate constant function f(x) = 1
    /// const result1 = quad.integrate(x => 1);
    /// console.log(result1);  // ~1.772454 (√π)
    ///
    /// // Integrate polynomial f(x) = x²
    /// const result2 = quad.integrate(x => x * x);
    /// console.log(result2);  // ~0.886227 (√π/2)
    /// ```
    #[wasm_bindgen(js_name = integrate)]
    pub fn integrate(&self, func: &JsValue) -> Result<f64, JsValue> {
        let callable = JsCallable::new(func)?;
        let closure = callable.closure();
        callable.run_value(|| self.inner.integrate(closure))
    }

    /// Integrate with adaptive tolerance using recursive subdivision.
    ///
    /// Automatically subdivides intervals where the function changes rapidly
    /// to achieve the specified tolerance.
    ///
    /// @param {Function} func - Function to integrate
    /// @param {number} tolerance - Desired absolute tolerance
    /// @returns {number} Approximate integral with specified tolerance
    /// @throws {Error} If function evaluation fails or tolerance not achieved
    ///
    /// @example
    /// ```javascript
    /// const quad = new GaussHermiteQuadrature(10);
    ///
    /// // High-precision integration of oscillatory function
    /// const result = quad.integrateAdaptive(
    ///   x => Math.sin(x) * Math.exp(-x*x/2),
    ///   1e-8
    /// );
    /// console.log(result);  // High-precision result
    /// ```
    #[wasm_bindgen(js_name = integrateAdaptive)]
    pub fn integrate_adaptive(&self, func: &JsValue, tolerance: f64) -> Result<f64, JsValue> {
        let callable = JsCallable::new(func)?;
        let closure = callable.closure();
        callable.run_value(|| self.inner.integrate_adaptive(closure, tolerance))
    }

    /// String representation of the quadrature rule.
    ///
    /// @returns {string} Human-readable description
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("GaussHermiteQuadrature(order={})", self.order())
    }
}

/// Simpson's rule for numerical integration over a fixed interval.
///
/// Uses Simpson's 1/3 rule: ∫_a^b f(x) dx ≈ (b-a)/6 * [f(a) + 4f((a+b)/2) + f(b)]
/// Extended to multiple intervals for higher accuracy.
///
/// @param {Function} func - Function to integrate (takes number, returns number)
/// @param {number} a - Lower bound of integration
/// @param {number} b - Upper bound of integration
/// @param {number} intervals - Number of subintervals (must be even)
/// @returns {number} Approximate integral value
/// @throws {Error} If function evaluation fails or intervals is odd
///
/// @example
/// ```javascript
/// // Integrate x² from 0 to 2 (exact answer: 8/3 ≈ 2.667)
/// const result = simpsonRule(x => x * x, 0, 2, 100);
/// console.log(result);  // ~2.666667
///
/// // Integrate sin(x) from 0 to π (exact answer: 2)
/// const sinIntegral = simpsonRule(Math.sin, 0, Math.PI, 50);
/// console.log(sinIntegral);  // ~2.000000
/// ```
#[wasm_bindgen(js_name = simpsonRule)]
pub fn simpson_rule(func: &JsValue, a: f64, b: f64, intervals: usize) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::simpson_rule(closure, a, b, intervals),
        |err| js_error(err.to_string()),
    )
}

/// Adaptive Simpson's rule with automatic subdivision for desired tolerance.
///
/// Recursively subdivides intervals where the function changes rapidly,
/// stopping when the estimated error is below the tolerance.
///
/// @param {Function} func - Function to integrate
/// @param {number} a - Lower bound of integration
/// @param {number} b - Upper bound of integration
/// @param {number} tol - Absolute tolerance for the result
/// @param {number} max_depth - Maximum recursion depth (prevents infinite subdivision)
/// @returns {number} Approximate integral with specified tolerance
/// @throws {Error} If function evaluation fails or tolerance not achieved
///
/// @example
/// ```javascript
/// // High-precision integration with automatic subdivision
/// const result = adaptiveSimpson(
///   x => Math.exp(-x*x),  // Gaussian-like function
///   0, 2,
///   1e-8,  // Very high precision
///   20     // Max recursion depth
/// );
/// console.log(result);  // High-precision result
/// ```
#[wasm_bindgen(js_name = adaptiveSimpson)]
pub fn adaptive_simpson(
    func: &JsValue,
    a: f64,
    b: f64,
    tol: f64,
    max_depth: usize,
) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::adaptive_simpson(closure, a, b, tol, max_depth),
        |err| js_error(err.to_string()),
    )
}

/// Alias for adaptiveSimpson (same implementation).
///
/// @param {Function} func - Function to integrate
/// @param {number} a - Lower bound of integration
/// @param {number} b - Upper bound of integration
/// @param {number} tol - Absolute tolerance for the result
/// @param {number} max_depth - Maximum recursion depth
/// @returns {number} Approximate integral with specified tolerance
#[wasm_bindgen(js_name = adaptiveQuadrature)]
pub fn adaptive_quadrature(
    func: &JsValue,
    a: f64,
    b: f64,
    tol: f64,
    max_depth: usize,
) -> Result<f64, JsValue> {
    adaptive_simpson(func, a, b, tol, max_depth)
}

/// Gauss-Legendre quadrature for high-accuracy integration.
///
/// Uses optimal quadrature points and weights for polynomial integration.
/// Very accurate for smooth functions over finite intervals.
///
/// @param {Function} func - Function to integrate
/// @param {number} a - Lower bound of integration
/// @param {number} b - Upper bound of integration
/// @param {number} order - Quadrature order (typically 5, 7, 10, 15, 20)
/// @returns {number} Approximate integral value
/// @throws {Error} If function evaluation fails
///
/// @example
/// ```javascript
/// // High-accuracy integration of polynomial
/// const result = gaussLegendreIntegrate(
///   x => x*x*x*x,  // x⁴
///   -1, 1,
///   10  // 10-point rule
/// );
/// console.log(result);  // ~0.4 (exact: 2/5)
/// ```
#[wasm_bindgen(js_name = gaussLegendreIntegrate)]
pub fn gauss_legendre_integrate(
    func: &JsValue,
    a: f64,
    b: f64,
    order: usize,
) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::gauss_legendre_integrate(closure, a, b, order),
        |err| js_error(err.to_string()),
    )
}

/// Composite Gauss-Legendre quadrature with multiple panels.
///
/// Divides the interval into multiple panels, applying Gauss-Legendre
/// quadrature to each panel for improved accuracy over long intervals.
///
/// @param {Function} func - Function to integrate
/// @param {number} a - Lower bound of integration
/// @param {number} b - Upper bound of integration
/// @param {number} order - Quadrature order per panel
/// @param {number} panels - Number of subintervals/panels
/// @returns {number} Approximate integral value
/// @throws {Error} If function evaluation fails
///
/// @example
/// ```javascript
/// // Integrate over long interval with multiple panels
/// const result = gaussLegendreIntegrateComposite(
///   x => Math.sin(x),
///   0, 10,  // Long interval
///   7,      // 7-point rule per panel
///   20      // 20 panels
/// );
/// console.log(result);  // High accuracy over long interval
/// ```
#[wasm_bindgen(js_name = gaussLegendreIntegrateComposite)]
pub fn gauss_legendre_integrate_composite(
    func: &JsValue,
    a: f64,
    b: f64,
    order: usize,
    panels: usize,
) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::gauss_legendre_integrate_composite(closure, a, b, order, panels),
        |err| js_error(err.to_string()),
    )
}

/// Adaptive Gauss-Legendre quadrature with automatic subdivision.
///
/// Combines high-accuracy Gauss-Legendre quadrature with adaptive subdivision
/// for optimal performance on functions with varying smoothness.
///
/// @param {Function} func - Function to integrate
/// @param {number} a - Lower bound of integration
/// @param {number} b - Upper bound of integration
/// @param {number} order - Quadrature order per panel
/// @param {number} tol - Absolute tolerance for the result
/// @param {number} max_depth - Maximum recursion depth
/// @returns {number} Approximate integral with specified tolerance
/// @throws {Error} If function evaluation fails or tolerance not achieved
///
/// @example
/// ```javascript
/// // Best of both worlds: high accuracy + adaptive subdivision
/// const result = gaussLegendreIntegrateAdaptive(
///   x => Math.exp(-x*x) * Math.sin(x),  // Oscillatory exponential
///   0, 5,
///   10,     // 10-point Gauss-Legendre
///   1e-10,  // Very high tolerance
///   15      // Max recursion depth
/// );
/// console.log(result);  // Very high precision result
/// ```
#[wasm_bindgen(js_name = gaussLegendreIntegrateAdaptive)]
pub fn gauss_legendre_integrate_adaptive(
    func: &JsValue,
    a: f64,
    b: f64,
    order: usize,
    tol: f64,
    max_depth: usize,
) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || {
            core_integration::gauss_legendre_integrate_adaptive(
                closure, a, b, order, tol, max_depth,
            )
        },
        |err| js_error(err.to_string()),
    )
}

/// Trapezoidal rule for numerical integration.
///
/// Uses the trapezoidal rule: ∫_a^b f(x) dx ≈ (b-a)/n * [f(a)/2 + f(x₁) + ... + f(xₙ₋₁) + f(b)/2]
/// Simple but less accurate than Simpson's rule or Gauss-Legendre.
///
/// @param {Function} func - Function to integrate
/// @param {number} a - Lower bound of integration
/// @param {number} b - Upper bound of integration
/// @param {number} intervals - Number of subintervals
/// @returns {number} Approximate integral value
/// @throws {Error} If function evaluation fails
///
/// @example
/// ```javascript
/// // Simple trapezoidal integration
/// const result = trapezoidalRule(x => x * x, 0, 2, 100);
/// console.log(result);  // ~2.6668 (less accurate than Simpson's)
/// ```
#[wasm_bindgen(js_name = trapezoidalRule)]
pub fn trapezoidal_rule(func: &JsValue, a: f64, b: f64, intervals: usize) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::trapezoidal_rule(closure, a, b, intervals),
        |err| js_error(err.to_string()),
    )
}
