//! Pricing overrides for market-quoted instruments.

use finstack_core::money::Money;

/// Optional parameters that override model pricing with market quotes.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PricingOverrides {
    /// Quoted clean price (for bonds)
    pub quoted_clean_price: Option<f64>,
    /// Implied volatility (overrides vol surface)
    pub implied_volatility: Option<f64>,
    /// Quoted spread (for credit instruments)
    pub quoted_spread_bp: Option<f64>,
    /// Upfront payment (for CDS, convertibles)
    pub upfront_payment: Option<Money>,
    /// Optional YTM bump size for numerical metrics (e.g., convexity/duration), in decimal (1 bp = 1e-4)
    pub ytm_bump_bp: Option<f64>,
    /// Theta period for time decay calculations (e.g., "1D", "1W", "1M", "3M")
    pub theta_period: Option<String>,
    /// MC seed scenario override for deterministic greek calculations
    ///
    /// When computing greeks via finite differences, this allows specifying
    /// a scenario name (e.g., "delta_up", "vega_down") to derive deterministic
    /// seeds. If None, uses default seed or derives from instrument ID + "base".
    pub mc_seed_scenario: Option<String>,
    /// Enable adaptive bump sizes based on volatility and moneyness
    ///
    /// When true, bump sizes are scaled based on:
    /// - Volatility level (higher vol → larger bumps)
    /// - Time to expiry (longer dated → larger bumps)
    /// - Moneyness (deep ITM/OTM → smaller bumps)
    ///
    /// Default: false (use fixed bump sizes)
    pub adaptive_bumps: bool,
    /// Custom spot bump size override (as percentage, e.g., 0.01 for 1%)
    ///
    /// When set, overrides both standard and adaptive spot bump calculations.
    pub spot_bump_pct: Option<f64>,
    /// Custom volatility bump size override (as absolute vol, e.g., 0.01 for 1% vol)
    ///
    /// When set, overrides both standard and adaptive volatility bump calculations.
    pub vol_bump_pct: Option<f64>,
    /// Custom rate bump size override (in basis points, e.g., 1.0 for 1bp)
    ///
    /// When set, overrides both standard and adaptive rate bump calculations.
    pub rate_bump_bp: Option<f64>,
}

impl PricingOverrides {
    /// Create empty pricing overrides
    pub fn none() -> Self {
        Self::default()
    }

    /// Set quoted clean price
    pub fn with_clean_price(mut self, price: f64) -> Self {
        self.quoted_clean_price = Some(price);
        self
    }

    /// Set implied volatility
    pub fn with_implied_vol(mut self, vol: f64) -> Self {
        self.implied_volatility = Some(vol);
        self
    }

    /// Set quoted spread
    pub fn with_spread_bp(mut self, spread_bp: f64) -> Self {
        self.quoted_spread_bp = Some(spread_bp);
        self
    }

    /// Set upfront payment
    pub fn with_upfront(mut self, upfront: Money) -> Self {
        self.upfront_payment = Some(upfront);
        self
    }

    /// Set custom YTM bump size (decimal). For 1 bp, pass 1e-4.
    pub fn with_ytm_bump(mut self, bump: f64) -> Self {
        self.ytm_bump_bp = Some(bump);
        self
    }

    /// Set theta period for time decay calculations.
    pub fn with_theta_period(mut self, period: impl Into<String>) -> Self {
        self.theta_period = Some(period.into());
        self
    }

    /// Set MC seed scenario for deterministic greek calculations.
    ///
    /// The scenario name (e.g., "delta_up", "vega_down") is used to derive
    /// a deterministic seed from the instrument ID, ensuring reproducibility.
    pub fn with_mc_seed_scenario(mut self, scenario: impl Into<String>) -> Self {
        self.mc_seed_scenario = Some(scenario.into());
        self
    }

    /// Enable adaptive bump sizes for greek calculations.
    ///
    /// Adaptive bumps scale based on volatility, time to expiry, and moneyness
    /// to improve numerical stability for extreme parameter values.
    pub fn with_adaptive_bumps(mut self, enable: bool) -> Self {
        self.adaptive_bumps = enable;
        self
    }

    /// Set custom spot bump size (as percentage, e.g., 0.01 for 1%).
    ///
    /// Overrides both standard and adaptive calculations when set.
    pub fn with_spot_bump(mut self, bump_pct: f64) -> Self {
        self.spot_bump_pct = Some(bump_pct);
        self
    }

    /// Set custom volatility bump size (as absolute vol, e.g., 0.01 for 1% vol).
    ///
    /// Overrides both standard and adaptive calculations when set.
    pub fn with_vol_bump(mut self, bump_pct: f64) -> Self {
        self.vol_bump_pct = Some(bump_pct);
        self
    }

    /// Set custom rate bump size (in basis points, e.g., 1.0 for 1bp).
    ///
    /// Overrides both standard and adaptive calculations when set.
    pub fn with_rate_bump(mut self, bump_bp: f64) -> Self {
        self.rate_bump_bp = Some(bump_bp);
        self
    }
}
