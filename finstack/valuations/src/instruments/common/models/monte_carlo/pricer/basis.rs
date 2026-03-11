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

/// Laguerre basis normalized by strike for option-style payoffs.
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
