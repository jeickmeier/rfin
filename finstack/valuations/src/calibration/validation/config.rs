//! Validation configuration for curves and surfaces.

use serde::{Deserialize, Serialize};

/// Validation configuration for different curve types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable forward rate positivity check
    pub check_forward_positivity: bool,
    /// Minimum allowed forward rate (can be slightly negative)
    pub min_forward_rate: f64,
    /// Maximum allowed forward rate
    pub max_forward_rate: f64,
    /// Enable monotonicity checks
    pub check_monotonicity: bool,
    /// Enable arbitrage checks
    pub check_arbitrage: bool,
    /// Numerical tolerance for comparisons
    pub tolerance: f64,
    /// Maximum allowed hazard rate (default 0.5 = 50%)
    pub max_hazard_rate: f64,
    /// Minimum allowed annual CPI growth (default -0.10 = -10%)
    pub min_cpi_growth: f64,
    /// Maximum allowed annual CPI growth (default 0.50 = 50%)
    pub max_cpi_growth: f64,
    /// Minimum allowed forward inflation (default -0.20 = -20%)
    pub min_fwd_inflation: f64,
    /// Maximum allowed forward inflation (default 0.50 = 50%)
    pub max_fwd_inflation: f64,
    /// Maximum allowed volatility (default 5.0 = 500%)
    pub max_volatility: f64,
    /// Allow negative rate environments (DF > 1.0 at short end)
    #[serde(default)]
    pub allow_negative_rates: bool,
    /// When true, arbitrage violations (calendar/butterfly) produce warnings instead of errors.
    /// Default is false - arbitrage violations fail validation.
    /// Set to true only for exploratory analysis or when arbitrage-free fitting is not required.
    #[serde(default)]
    pub lenient_arbitrage: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            check_forward_positivity: true,
            min_forward_rate: -0.01, // Allow 1% negative
            max_forward_rate: 0.50,  // 50% cap
            check_monotonicity: true,
            check_arbitrage: true,
            tolerance: 1e-10,
            max_hazard_rate: 0.50,
            min_cpi_growth: -0.10,
            max_cpi_growth: 0.50,
            min_fwd_inflation: -0.20,
            max_fwd_inflation: 0.50,
            max_volatility: 5.0,
            // Default to strict mode: enforce monotonicity in positive-rate regimes.
            // Set to true for EUR/JPY/CHF negative-rate environments where DFs > 1.0 is valid.
            allow_negative_rates: false,
            // Default to strict mode: arbitrage violations fail validation.
            // Set to true only for exploratory analysis.
            lenient_arbitrage: false,
        }
    }
}

impl ValidationConfig {
    /// Create a strict validation config that enforces monotonicity
    /// even in potentially negative rate environments.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            allow_negative_rates: false,
            lenient_arbitrage: false,
            ..Default::default()
        }
    }

    /// Create a permissive validation config for negative rate environments
    /// (e.g., EUR/JPY/CHF) where discount factors > 1.0 at short tenors is valid.
    #[must_use]
    pub fn negative_rates() -> Self {
        Self {
            allow_negative_rates: true,
            ..Default::default()
        }
    }

    /// Create a lenient configuration that warns but does not fail on arbitrage.
    ///
    /// Use this only for exploratory analysis or when strict arbitrage-free
    /// surfaces are not required. Calendar spread and butterfly arbitrage
    /// violations will log warnings instead of returning errors.
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            lenient_arbitrage: true,
            ..Default::default()
        }
    }

    /// Set whether arbitrage violations should warn (lenient) or error (strict).
    ///
    /// By default, arbitrage violations fail validation. Set `lenient = true`
    /// only for exploratory analysis or when arbitrage-free constraints are
    /// not required.
    #[must_use]
    pub fn with_lenient_arbitrage(mut self, lenient: bool) -> Self {
        self.lenient_arbitrage = lenient;
        self
    }
}


