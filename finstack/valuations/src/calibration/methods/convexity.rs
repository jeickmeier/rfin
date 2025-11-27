//! Convexity adjustment calculations for interest rate futures.
//!
//! Provides market-standard convexity adjustments to convert futures rates
//! to forward rates, accounting for the daily margining of futures contracts.
//!
//! # Market Practice
//!
//! The convexity adjustment corrects for the difference between futures rates
//! and forward rates arising from daily margining of futures contracts. Key sources:
//!
//! - **Hull-White model**: Standard approximation using σ²T₁T₂/2
//! - **Ho-Lee model**: Zero mean reversion, suitable for short-dated futures
//! - **Market-implied**: Derived from cap/floor or swaption volatilities
//!
//! # Volatility Sources
//!
//! In order of preference:
//! 1. **Explicit override**: User-specified adjustment (highest priority)
//! 2. **Market-implied**: From ATM swaption vol surface if available
//! 3. **Currency defaults**: Fallback to hardcoded parameters
//!
//! # References
//!
//! - Hull, J. (2018). *Options, Futures, and Other Derivatives* (10th ed.). Chapter 6.
//! - Burghardt, G., & Hoskins, W. (1995). "The Convexity Bias in Eurodollar Futures."
//!   *Risk*, 8(3), 63-70.

/// Calculate convexity adjustment for interest rate futures.
///
/// The convexity adjustment accounts for the difference between futures and forwards
/// due to daily mark-to-market settlement of futures contracts.
///
/// Formula: CA = 0.5 * σ² * T₁ * T₂
/// where:
/// - σ is the rate volatility
/// - T₁ is time to futures expiry
/// - T₂ is time to rate maturity
///
/// # Arguments
/// * `time_to_expiry` - Time to futures expiry in years
/// * `time_to_maturity` - Time to underlying rate maturity in years
/// * `rate_volatility` - Annualized rate volatility (e.g., 0.01 for 1%)
///
/// # Returns
/// Convexity adjustment to add to futures rate to get forward rate
pub fn calculate_convexity_adjustment(
    time_to_expiry: f64,
    time_to_maturity: f64,
    rate_volatility: f64,
) -> f64 {
    // Hull-White approximation for convexity adjustment
    0.5 * rate_volatility * rate_volatility * time_to_expiry * time_to_maturity
}

/// Calculate implied rate volatility from market data.
///
/// Uses a simplified approach based on historical volatility patterns.
/// In practice, this would be calibrated from option prices.
///
/// # Arguments
/// * `tenor_years` - Tenor of the underlying rate in years
/// * `time_to_expiry` - Time to futures expiry in years
///
/// # Returns
/// Estimated annualized rate volatility
pub fn estimate_rate_volatility(tenor_years: f64, time_to_expiry: f64) -> f64 {
    // Base volatility depends on tenor
    let base_vol = if tenor_years <= 0.25 {
        0.0080 // 80bp for 3M rates
    } else if tenor_years <= 0.5 {
        0.0075 // 75bp for 6M rates
    } else {
        0.0070 // 70bp for longer tenors
    };

    // Volatility increases with time to expiry
    let time_adjustment = (1.0 + 0.1 * time_to_expiry).min(1.5);

    base_vol * time_adjustment
}

/// Calculate convexity adjustment using Ho-Lee model.
///
/// More sophisticated than Hull-White for long-dated futures.
///
/// # Arguments
/// * `time_to_expiry` - Time to futures expiry in years
/// * `time_to_maturity` - Time to underlying rate maturity in years
/// * `rate_volatility` - Annualized rate volatility
/// * `mean_reversion` - Mean reversion parameter (0 for Ho-Lee)
pub fn ho_lee_convexity(
    time_to_expiry: f64,
    time_to_maturity: f64,
    rate_volatility: f64,
    mean_reversion: f64,
) -> f64 {
    if mean_reversion.abs() < 1e-10 {
        // Ho-Lee model (no mean reversion)
        calculate_convexity_adjustment(time_to_expiry, time_to_maturity, rate_volatility)
    } else {
        // Hull-White with mean reversion
        let exp_neg_a_t1 = (-mean_reversion * time_to_expiry).exp();
        let exp_neg_a_t2 = (-mean_reversion * time_to_maturity).exp();

        let b_t1_t2 = (1.0 - exp_neg_a_t2 * exp_neg_a_t1.recip()) / mean_reversion;
        let variance = rate_volatility * rate_volatility * (1.0 - exp_neg_a_t1 * exp_neg_a_t1)
            / (2.0 * mean_reversion);

        0.5 * variance * b_t1_t2 * b_t1_t2
    }
}

/// Source of volatility for convexity adjustment calculation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum VolatilitySource {
    /// Use hardcoded currency-specific defaults
    Default,
    /// Use explicit volatility value (decimal, e.g., 0.0075 for 75bp)
    Explicit(f64),
    /// Use market-implied volatility from ATM swaption at given tenor (years)
    MarketImplied {
        /// Swaption expiry matching futures expiry
        expiry_years: f64,
        /// Underlying swap tenor
        tenor_years: f64,
    },
}

impl Default for VolatilitySource {
    fn default() -> Self {
        Self::Default
    }
}

/// Convexity adjustment parameters for interest rate futures.
///
/// Provides market-standard convexity corrections with configurable
/// volatility sources to support both default and market-implied adjustments.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConvexityParameters {
    /// Base rate volatility (used when vol_source is Default)
    pub base_volatility: f64,
    /// Mean reversion parameter (0 for Ho-Lee)
    pub mean_reversion: f64,
    /// Use Ho-Lee model instead of simple Hull-White
    pub use_ho_lee: bool,
    /// Volatility source for calculation
    #[serde(default)]
    pub vol_source: VolatilitySource,
    /// Optional explicit convexity adjustment override (bypasses calculation)
    pub explicit_adjustment: Option<f64>,
}

impl ConvexityParameters {
    /// USD SOFR futures parameters
    pub fn usd_sofr() -> Self {
        Self {
            base_volatility: 0.0075, // 75bp default
            mean_reversion: 0.03,
            use_ho_lee: false,
            vol_source: VolatilitySource::Default,
            explicit_adjustment: None,
        }
    }

    /// EUR EURIBOR futures parameters
    pub fn eur_euribor() -> Self {
        Self {
            base_volatility: 0.0070, // 70bp default
            mean_reversion: 0.025,
            use_ho_lee: false,
            vol_source: VolatilitySource::Default,
            explicit_adjustment: None,
        }
    }

    /// GBP SONIA futures parameters
    pub fn gbp_sonia() -> Self {
        Self {
            base_volatility: 0.0080, // 80bp default
            mean_reversion: 0.035,
            use_ho_lee: false,
            vol_source: VolatilitySource::Default,
            explicit_adjustment: None,
        }
    }

    /// JPY TONAR futures parameters
    pub fn jpy_tonar() -> Self {
        Self {
            base_volatility: 0.0040, // 40bp default for low-rate environment
            mean_reversion: 0.02,
            use_ho_lee: true, // Use Ho-Lee for JPY due to low rates
            vol_source: VolatilitySource::Default,
            explicit_adjustment: None,
        }
    }

    /// CHF SARON futures parameters
    pub fn chf_saron() -> Self {
        Self {
            base_volatility: 0.0050, // 50bp default for Swiss rates
            mean_reversion: 0.025,
            use_ho_lee: true, // Ho-Lee for potentially negative rates
            vol_source: VolatilitySource::Default,
            explicit_adjustment: None,
        }
    }

    /// Set an explicit volatility value (overrides default)
    pub fn with_volatility(mut self, vol: f64) -> Self {
        self.vol_source = VolatilitySource::Explicit(vol);
        self
    }

    /// Set explicit convexity adjustment (bypasses calculation entirely)
    pub fn with_explicit_adjustment(mut self, adj: f64) -> Self {
        self.explicit_adjustment = Some(adj);
        self
    }

    /// Configure for market-implied volatility lookup
    pub fn with_market_implied_vol(mut self, expiry_years: f64, tenor_years: f64) -> Self {
        self.vol_source = VolatilitySource::MarketImplied {
            expiry_years,
            tenor_years,
        };
        self
    }

    /// Get effective volatility for calculation.
    ///
    /// For market-implied source, this returns the base volatility as fallback.
    /// Actual market vol lookup should be done by the caller using `calculate_adjustment_with_market_vol`.
    fn effective_volatility(&self, time_to_expiry: f64) -> f64 {
        match &self.vol_source {
            VolatilitySource::Default => {
                // Apply time-dependent scaling
                self.base_volatility * (1.0 + 0.1 * time_to_expiry).min(1.5)
            }
            VolatilitySource::Explicit(vol) => *vol,
            VolatilitySource::MarketImplied { .. } => {
                // Fallback to default; caller should use calculate_adjustment_with_market_vol
                self.base_volatility * (1.0 + 0.1 * time_to_expiry).min(1.5)
            }
        }
    }

    /// Calculate convexity adjustment using internal volatility source.
    pub fn calculate_adjustment(&self, time_to_expiry: f64, time_to_maturity: f64) -> f64 {
        // If explicit adjustment is set, use it directly
        if let Some(adj) = self.explicit_adjustment {
            return adj;
        }

        let volatility = self.effective_volatility(time_to_expiry);

        if self.use_ho_lee {
            ho_lee_convexity(
                time_to_expiry,
                time_to_maturity,
                volatility,
                self.mean_reversion,
            )
        } else {
            calculate_convexity_adjustment(time_to_expiry, time_to_maturity, volatility)
        }
    }

    /// Calculate convexity adjustment using market-implied volatility.
    ///
    /// This method accepts an optional ATM volatility from a swaption surface,
    /// using it instead of the default volatility when available.
    ///
    /// # Arguments
    ///
    /// * `time_to_expiry` - Time to futures expiry in years
    /// * `time_to_maturity` - Time to underlying rate maturity in years
    /// * `market_vol` - Optional market-implied ATM volatility from swaption surface
    ///
    /// # Returns
    ///
    /// Convexity adjustment in rate terms (add to futures rate to get forward rate)
    pub fn calculate_adjustment_with_market_vol(
        &self,
        time_to_expiry: f64,
        time_to_maturity: f64,
        market_vol: Option<f64>,
    ) -> f64 {
        // If explicit adjustment is set, use it directly
        if let Some(adj) = self.explicit_adjustment {
            return adj;
        }

        // Use market vol if provided, otherwise fall back to configured source
        let volatility = market_vol.unwrap_or_else(|| self.effective_volatility(time_to_expiry));

        if self.use_ho_lee {
            ho_lee_convexity(
                time_to_expiry,
                time_to_maturity,
                volatility,
                self.mean_reversion,
            )
        } else {
            calculate_convexity_adjustment(time_to_expiry, time_to_maturity, volatility)
        }
    }
}

/// Get default convexity parameters for a currency.
///
/// Returns market-standard parameters based on currency conventions.
pub fn default_convexity_params(currency: finstack_core::currency::Currency) -> ConvexityParameters {
    use finstack_core::currency::Currency;
    match currency {
        Currency::USD => ConvexityParameters::usd_sofr(),
        Currency::EUR => ConvexityParameters::eur_euribor(),
        Currency::GBP => ConvexityParameters::gbp_sonia(),
        Currency::JPY => ConvexityParameters::jpy_tonar(),
        Currency::CHF => ConvexityParameters::chf_saron(),
        _ => ConvexityParameters::usd_sofr(), // Default to USD parameters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_convexity_adjustment() {
        // 1-year future on 3-month rate with 1% volatility
        let adj = calculate_convexity_adjustment(1.0, 1.25, 0.01);
        assert!((adj - 0.0000625).abs() < 1e-8); // 0.625 bp
    }

    #[test]
    fn test_volatility_estimation() {
        let vol_3m = estimate_rate_volatility(0.25, 1.0);
        assert!((vol_3m - 0.0088).abs() < 1e-3); // ~88bp

        let vol_6m = estimate_rate_volatility(0.5, 2.0);
        assert!((vol_6m - 0.00975).abs() < 1e-3); // ~97.5bp
    }

    #[test]
    fn test_currency_specific_parameters() {
        let usd_params = ConvexityParameters::usd_sofr();
        let adj = usd_params.calculate_adjustment(2.0, 2.25);
        assert!(adj > 0.0 && adj < 0.001); // Reasonable range
    }

    #[test]
    fn test_ho_lee_model() {
        // With zero mean reversion, should match simple model
        let simple = calculate_convexity_adjustment(1.0, 1.25, 0.01);
        let ho_lee = ho_lee_convexity(1.0, 1.25, 0.01, 0.0);
        assert!((simple - ho_lee).abs() < 1e-10);

        // With mean reversion, should be smaller
        let with_mr = ho_lee_convexity(1.0, 1.25, 0.01, 0.03);
        assert!(with_mr < simple);
    }

    #[test]
    fn test_explicit_volatility_override() {
        let mut params = ConvexityParameters::usd_sofr();
        let default_adj = params.calculate_adjustment(2.0, 2.25);

        // Override with explicit volatility
        params = params.with_volatility(0.015); // 150bp vol
        let explicit_adj = params.calculate_adjustment(2.0, 2.25);

        // Higher vol should give higher adjustment
        assert!(explicit_adj > default_adj);
    }

    #[test]
    fn test_explicit_adjustment_override() {
        let params = ConvexityParameters::usd_sofr().with_explicit_adjustment(0.0005); // 5bp

        // Should return explicit adjustment regardless of times
        let adj = params.calculate_adjustment(1.0, 2.0);
        assert!((adj - 0.0005).abs() < 1e-10);
    }

    #[test]
    fn test_market_vol_override() {
        let params = ConvexityParameters::usd_sofr();

        // Without market vol - uses default
        let default_adj = params.calculate_adjustment_with_market_vol(2.0, 2.25, None);

        // With market vol override
        let market_adj = params.calculate_adjustment_with_market_vol(2.0, 2.25, Some(0.012));

        // Different vol should give different adjustment
        assert!((default_adj - market_adj).abs() > 1e-6);
    }

    #[test]
    fn test_default_params_by_currency() {
        use finstack_core::currency::Currency;

        let usd = default_convexity_params(Currency::USD);
        assert!((usd.base_volatility - 0.0075).abs() < 1e-10);

        let jpy = default_convexity_params(Currency::JPY);
        assert!(jpy.use_ho_lee); // JPY uses Ho-Lee

        let chf = default_convexity_params(Currency::CHF);
        assert!(chf.use_ho_lee); // CHF uses Ho-Lee for negative rates
    }

    #[test]
    fn test_chf_saron_parameters() {
        let chf_params = ConvexityParameters::chf_saron();
        let adj = chf_params.calculate_adjustment(2.0, 2.25);
        // Should work for potentially negative rate environment
        assert!(adj.is_finite());
        assert!(adj >= 0.0); // Convexity adjustment is always positive
    }
}
