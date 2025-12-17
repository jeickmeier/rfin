//! Convexity adjustment calculations for interest rate futures.
//!
//! Calculate convexity adjustment for interest rate futures.

/// Calculate convexity adjustment for interest rate futures.
pub fn calculate_convexity_adjustment(
    time_to_expiry: f64,
    time_to_maturity: f64,
    rate_volatility: f64,
) -> f64 {
    // Hull-White approximation for convexity adjustment
    0.5 * rate_volatility * rate_volatility * time_to_expiry * time_to_maturity
}

/// Calculate implied rate volatility from market data.
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
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum VolatilitySource {
    /// Use hardcoded currency-specific defaults
    #[default]
    Default,
    /// Use explicit volatility value (decimal)
    Explicit(f64),
    /// Use market-implied volatility from ATM swaption at given tenor (years)
    MarketImplied {
        /// Swaption expiry matching futures expiry
        expiry_years: f64,
        /// Underlying swap tenor
        tenor_years: f64,
    },
}

/// Convexity adjustment parameters for interest rate futures.
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

    /// Get convexity parameters for a specific currency.
    pub fn for_currency(currency: finstack_core::currency::Currency) -> Self {
        use finstack_core::currency::Currency;
        match currency {
            Currency::USD => Self::usd_sofr(),
            Currency::EUR => Self::eur_euribor(),
            Currency::GBP => Self::gbp_sonia(),
            Currency::JPY => Self::jpy_tonar(),
            Currency::CHF => Self::chf_saron(),
            _ => Self::usd_sofr(), // Default to USD parameters
        }
    }

    /// Calculate convexity adjustment for an interest rate future.
    pub fn calculate_for_future(
        &self,
        base_date: finstack_core::dates::Date,
        expiry: finstack_core::dates::Date,
        period_end: finstack_core::dates::Date,
        day_count: finstack_core::dates::DayCount,
    ) -> f64 {
        let dc_ctx = finstack_core::dates::DayCountCtx::default();
        let time_to_expiry = day_count
            .year_fraction(base_date, expiry, dc_ctx)
            .unwrap_or(0.0);
        let time_to_maturity = day_count
            .year_fraction(base_date, period_end, dc_ctx)
            .unwrap_or(0.0);
        self.calculate_adjustment(time_to_expiry, time_to_maturity)
    }

    /// Set the mean reversion parameter.
    #[must_use]
    pub fn with_mean_reversion(mut self, mean_reversion: f64) -> Self {
        self.mean_reversion = mean_reversion;
        self
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
#[inline]
pub fn default_convexity_params(
    currency: finstack_core::currency::Currency,
) -> ConvexityParameters {
    ConvexityParameters::for_currency(currency)
}
