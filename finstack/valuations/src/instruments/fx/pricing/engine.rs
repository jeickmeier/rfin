//! Shared FX underlying parameters used across FX-linked instruments.

use finstack_core::currency::Currency;

/// FX underlying parameters for FX options and swaps.
#[derive(Clone, Debug)]
pub struct FxUnderlyingParams {
    /// Base currency (being priced)
    pub base_currency: Currency,
    /// Quote currency (pricing currency)
    pub quote_currency: Currency,
    /// Domestic discount curve ID (quote currency)
    pub domestic_disc_id: &'static str,
    /// Foreign discount curve ID (base currency)
    pub foreign_disc_id: &'static str,
}

impl FxUnderlyingParams {
    /// Create FX underlying parameters
    pub fn new(
        base_currency: Currency,
        quote_currency: Currency,
        domestic_disc_id: &'static str,
        foreign_disc_id: &'static str,
    ) -> Self {
        Self {
            base_currency,
            quote_currency,
            domestic_disc_id,
            foreign_disc_id,
        }
    }

    /// Standard USD/EUR pair
    pub fn usd_eur() -> Self {
        Self::new(Currency::EUR, Currency::USD, "USD-OIS", "EUR-OIS")
    }

    /// Standard GBP/USD pair
    pub fn gbp_usd() -> Self {
        Self::new(Currency::GBP, Currency::USD, "USD-OIS", "GBP-OIS")
    }
}
