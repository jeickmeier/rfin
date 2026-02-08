//! Shared basis functions for Monte Carlo LSMC regressions.
//!
//! Centralizes common basis types to avoid duplication across pricers.

/// Basis functions used for LSMC regression.
pub trait BasisFunctions: Send + Sync {
    /// Number of basis functions.
    fn num_basis(&self) -> usize;

    /// Evaluate all basis functions at the given state value.
    fn evaluate(&self, state: f64, out: &mut [f64]);
}

/// Polynomial basis: {1, x, x², ...}.
#[derive(Debug, Clone)]
pub struct PolynomialBasis {
    degree: usize,
}

impl PolynomialBasis {
    /// Create polynomial basis of given degree (must be > 0).
    pub fn new(degree: usize) -> Self {
        assert!(degree > 0, "Degree must be positive");
        Self { degree }
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
    /// Create Laguerre basis of given degree with strike normalization.
    ///
    /// `degree` must be in [1, 4] and `strike` must be positive.
    pub fn new(degree: usize, strike: f64) -> Self {
        assert!(degree > 0 && degree <= 4, "Degree must be 1-4");
        assert!(strike > 0.0, "Strike must be positive");
        Self { degree, strike }
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
