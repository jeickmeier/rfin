//! Pricing overrides for market-quoted instruments.

use finstack_core::{money::Money, F};

/// Optional parameters that override model pricing with market quotes.
#[derive(Clone, Debug, Default)]
pub struct PricingOverrides {
    /// Quoted clean price (for bonds)
    pub quoted_clean_price: Option<F>,
    /// Implied volatility (overrides vol surface)
    pub implied_volatility: Option<F>,
    /// Quoted spread (for credit instruments)
    pub quoted_spread_bp: Option<F>,
    /// Upfront payment (for CDS, convertibles)
    pub upfront_payment: Option<Money>,
    /// Optional YTM bump size for numerical metrics (e.g., convexity/duration), in decimal (1 bp = 1e-4)
    pub ytm_bump_bp: Option<F>,
}

impl PricingOverrides {
    /// Create empty pricing overrides
    pub fn none() -> Self {
        Self::default()
    }

    /// Set quoted clean price
    pub fn with_clean_price(mut self, price: F) -> Self {
        self.quoted_clean_price = Some(price);
        self
    }

    /// Set implied volatility
    pub fn with_implied_vol(mut self, vol: F) -> Self {
        self.implied_volatility = Some(vol);
        self
    }

    /// Set quoted spread
    pub fn with_spread_bp(mut self, spread_bp: F) -> Self {
        self.quoted_spread_bp = Some(spread_bp);
        self
    }

    /// Set upfront payment
    pub fn with_upfront(mut self, upfront: Money) -> Self {
        self.upfront_payment = Some(upfront);
        self
    }

    /// Set custom YTM bump size (decimal). For 1 bp, pass 1e-4.
    pub fn with_ytm_bump(mut self, bump: F) -> Self {
        self.ytm_bump_bp = Some(bump);
        self
    }
}
