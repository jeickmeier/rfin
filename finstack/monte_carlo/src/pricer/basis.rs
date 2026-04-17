//! Shared basis functions for Monte Carlo LSMC regressions.
//!
//! Centralizes common basis types to avoid duplication across pricers.

/// Basis functions used for LSMC regression.
pub trait BasisFunctions: Send + Sync {
    /// Number of basis functions.
    fn num_basis(&self) -> usize;

    /// Evaluate all basis functions at the given state value.
    fn evaluate(&self, state: f64, out: &mut [f64]);

    /// Evaluate basis functions with an optional auxiliary state variable.
    ///
    /// Implementations that only depend on the primary state can ignore `aux`
    /// and rely on the default behavior.
    fn evaluate_with_aux(&self, state: f64, _aux: Option<f64>, out: &mut [f64]) {
        self.evaluate(state, out);
    }
}

/// Polynomial basis: {1, x, x², ...}.
#[derive(Debug, Clone)]
pub struct PolynomialBasis {
    degree: usize,
}

impl PolynomialBasis {
    /// Create polynomial basis of given degree (panics if `degree == 0`).
    pub fn new(degree: usize) -> Self {
        assert!(degree > 0, "Degree must be positive");
        Self { degree }
    }

    /// Create a validated polynomial basis, returning an error if `degree == 0`.
    pub fn try_new(degree: usize) -> Result<Self, String> {
        if degree == 0 {
            return Err("degree must be positive".to_string());
        }
        Ok(Self { degree })
    }
}

impl BasisFunctions for PolynomialBasis {
    fn num_basis(&self) -> usize {
        self.degree + 1
    }

    fn evaluate(&self, state: f64, out: &mut [f64]) {
        debug_assert_eq!(
            out.len(),
            self.num_basis(),
            "Buffer size mismatch: expected {}, got {}",
            self.num_basis(),
            out.len()
        );

        out[0] = 1.0;
        for i in 1..=self.degree {
            out[i] = out[i - 1] * state;
        }
    }
}

/// Normalized polynomial basis: {1, x̃, x̃², ...} where x̃ = (x - center) / scale.
///
/// Centering and scaling dramatically improve the condition number of the
/// Vandermonde-like regression matrix in LSMC, especially for higher degrees
/// or wide spot ranges. Recommended over [`PolynomialBasis`] when degree > 2.
#[derive(Debug, Clone)]
pub struct NormalizedPolynomialBasis {
    degree: usize,
    center: f64,
    scale: f64,
}

impl NormalizedPolynomialBasis {
    /// Create a normalized polynomial basis.
    ///
    /// # Arguments
    ///
    /// * `degree` - Polynomial degree (must be > 0)
    /// * `center` - Centering value (typically the mean or ATM spot)
    /// * `scale` - Scaling value (typically the standard deviation or strike)
    pub fn new(degree: usize, center: f64, scale: f64) -> Self {
        assert!(degree > 0, "Degree must be positive");
        assert!(scale.abs() > 1e-14, "Scale must be non-zero");
        Self {
            degree,
            center,
            scale,
        }
    }

    /// Create a validated normalized polynomial basis.
    pub fn try_new(degree: usize, center: f64, scale: f64) -> Result<Self, String> {
        if degree == 0 {
            return Err("degree must be positive".to_string());
        }
        if scale.abs() <= 1e-14 {
            return Err("scale must be non-zero".to_string());
        }
        Ok(Self {
            degree,
            center,
            scale,
        })
    }
}

impl BasisFunctions for NormalizedPolynomialBasis {
    fn num_basis(&self) -> usize {
        self.degree + 1
    }

    fn evaluate(&self, state: f64, out: &mut [f64]) {
        debug_assert_eq!(
            out.len(),
            self.num_basis(),
            "Buffer size mismatch: expected {}, got {}",
            self.num_basis(),
            out.len()
        );

        let x = (state - self.center) / self.scale;
        out[0] = 1.0;
        for i in 1..=self.degree {
            out[i] = out[i - 1] * x;
        }
    }
}

/// Laguerre basis normalised by strike for option-style payoffs.
///
/// Emits `[1, L_1(x), …, L_degree(x)]` where `x = S/K` and `L_k` are the
/// standard (non-weighted) Laguerre polynomials. In classical LSMC (Longstaff
/// & Schwartz, 2001) the regressors are weighted as `w_k(x) = exp(−x/2)·L_k(x)`
/// to make them orthonormal under the Lebesgue measure on `[0, ∞)`. We omit
/// the weight because the `S/K` normalisation already bounds the design
/// matrix's condition number for typical option payoffs, and because the
/// `exp(−x/2)` term has been observed to under-weight deep-ITM paths where
/// the continuation value is most sensitive. **Implication:** fitted
/// coefficients and regression-table reproducibility *will differ* from
/// published Longstaff–Schwartz benchmark tables by an `O(1)` rotation of
/// the basis; the resulting LSMC prices converge to the same limit but
/// finite-sample values are not bit-identical.
///
/// If you need to reproduce published benchmark tables, apply the
/// `exp(−x/2)` weight externally on the basis outputs or switch to
/// [`NormalizedPolynomialBasis`].
#[derive(Debug, Clone)]
pub struct LaguerreBasis {
    degree: usize,
    strike: f64,
}

impl LaguerreBasis {
    /// Create Laguerre basis of given degree with strike normalization
    /// (panics on invalid inputs).
    ///
    /// `degree` must be in [1, 4] and `strike` must be positive.
    pub fn new(degree: usize, strike: f64) -> Self {
        assert!(degree > 0 && degree <= 4, "Degree must be 1-4");
        assert!(strike > 0.0, "Strike must be positive");
        Self { degree, strike }
    }

    /// Create a validated Laguerre basis, returning an error on invalid inputs.
    pub fn try_new(degree: usize, strike: f64) -> Result<Self, String> {
        if degree == 0 || degree > 4 {
            return Err("degree must be 1-4".to_string());
        }
        if strike <= 0.0 {
            return Err("strike must be positive".to_string());
        }
        Ok(Self { degree, strike })
    }

    /// Strike price used for normalization.
    pub fn strike(&self) -> f64 {
        self.strike
    }
}

impl BasisFunctions for LaguerreBasis {
    fn num_basis(&self) -> usize {
        self.degree + 1
    }

    fn evaluate(&self, spot: f64, out: &mut [f64]) {
        debug_assert_eq!(
            out.len(),
            self.num_basis(),
            "Buffer size mismatch: expected {}, got {}",
            self.num_basis(),
            out.len()
        );

        // Laguerre polynomials evaluated at x = S / K (normalized spot)
        let x = spot / self.strike;

        out[0] = 1.0;
        if self.degree >= 1 {
            out[1] = 1.0 - x;
        }
        if self.degree >= 2 {
            out[2] = 1.0 - 2.0 * x + x * x / 2.0;
        }
        if self.degree >= 3 {
            out[3] = 1.0 - 3.0 * x + 3.0 * x * x / 2.0 - x * x * x / 6.0;
        }
        if self.degree >= 4 {
            out[4] = 1.0 - 4.0 * x + 3.0 * x * x - 2.0 * x * x * x / 3.0 + x * x * x * x / 24.0;
        }
    }
}
