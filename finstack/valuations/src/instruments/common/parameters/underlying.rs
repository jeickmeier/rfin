//! Underlying parameter types for different asset classes.

use finstack_core::currency::Currency;
use finstack_core::types::CurveId;
use finstack_core::types::IndexId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Base trait for underlying parameters to enable polymorphic behavior
pub trait UnderlyingParams {
    /// Get the base currency for pricing
    fn base_currency(&self) -> Currency;

    /// Get the primary curve identifier
    fn primary_curve_id(&self) -> &str;
}

/// FX underlying parameters used by FX options and FX swaps.
///
/// This struct encapsulates the market data curve identifiers and
/// currency pair information needed for pricing FX-related instruments.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FxUnderlyingParams {
    /// Base currency (being priced)
    pub base_currency: Currency,
    /// Quote currency (pricing currency)
    pub quote_currency: Currency,
    /// Domestic discount curve ID (quote currency)
    pub domestic_discount_curve_id: CurveId,
    /// Foreign discount curve ID (base currency)
    pub foreign_discount_curve_id: CurveId,
}

impl FxUnderlyingParams {
    /// Create FX underlying parameters
    pub fn new(
        base_currency: Currency,
        quote_currency: Currency,
        domestic_discount_curve_id: impl Into<CurveId>,
        foreign_discount_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            base_currency,
            quote_currency,
            domestic_discount_curve_id: domestic_discount_curve_id.into(),
            foreign_discount_curve_id: foreign_discount_curve_id.into(),
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

    /// Standard USD/JPY pair
    pub fn usd_jpy() -> Self {
        Self::new(Currency::JPY, Currency::USD, "USD-OIS", "JPY-OIS")
    }
}

impl UnderlyingParams for FxUnderlyingParams {
    fn base_currency(&self) -> Currency {
        self.base_currency
    }

    fn primary_curve_id(&self) -> &str {
        self.domestic_discount_curve_id.as_ref()
    }
}

/// Equity underlying parameters for options and equity-linked swaps.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EquityUnderlyingParams {
    /// Underlying ticker/identifier
    pub ticker: String,
    /// Spot price identifier in market data
    pub spot_id: String,
    /// Optional dividend yield identifier
    pub div_yield_id: Option<String>,
    /// Contract size (shares per contract)
    pub contract_size: f64,
    /// Base currency for pricing
    pub currency: Currency,
}

impl EquityUnderlyingParams {
    /// Create equity underlying parameters
    pub fn new(ticker: impl Into<String>, spot_id: impl Into<String>, currency: Currency) -> Self {
        Self {
            ticker: ticker.into(),
            spot_id: spot_id.into(),
            div_yield_id: None,
            contract_size: 1.0,
            currency,
        }
    }

    /// Set dividend yield identifier
    pub fn with_dividend_yield(mut self, div_yield_id: impl Into<String>) -> Self {
        self.div_yield_id = Some(div_yield_id.into());
        self
    }

    /// Set contract size
    pub fn with_contract_size(mut self, size: f64) -> Self {
        self.contract_size = size;
        self
    }
}

impl UnderlyingParams for EquityUnderlyingParams {
    fn base_currency(&self) -> Currency {
        self.currency
    }

    fn primary_curve_id(&self) -> &str {
        match self.currency {
            Currency::USD => "USD-OIS",
            Currency::EUR => "EUR-OIS",
            Currency::GBP => "GBP-OIS",
            Currency::JPY => "JPY-OIS",
            Currency::CHF => "CHF-OIS",
            Currency::CAD => "CAD-OIS",
            Currency::AUD => "AUD-OIS",
            Currency::NZD => "NZD-OIS",
            _ => "USD-OIS", // Fallback for less common currencies
        }
    }
}

/// Index underlying parameters for total return swaps and index-linked instruments.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IndexUnderlyingParams {
    /// Index identifier (e.g., "CDX.IG", "HY.BOND.INDEX")
    pub index_id: IndexId,
    /// Base currency of the index
    pub base_currency: Currency,
    /// Optional yield curve/scalar identifier for carry calculation
    pub yield_id: Option<String>,
    /// Optional duration identifier for risk calculations
    pub duration_id: Option<String>,
    /// Optional convexity identifier for risk calculations
    pub convexity_id: Option<String>,
    /// Contract size (index units per contract, defaults to 1.0)
    pub contract_size: f64,
}

impl IndexUnderlyingParams {
    /// Create index underlying parameters
    pub fn new(index_id: impl Into<String>, base_currency: Currency) -> Self {
        Self {
            index_id: IndexId::new(index_id),
            base_currency,
            yield_id: None,
            duration_id: None,
            convexity_id: None,
            contract_size: 1.0,
        }
    }

    /// Set yield identifier for carry calculation
    pub fn with_yield(mut self, yield_id: impl Into<String>) -> Self {
        self.yield_id = Some(yield_id.into());
        self
    }

    /// Set duration identifier for risk calculations
    pub fn with_duration(mut self, duration_id: impl Into<String>) -> Self {
        self.duration_id = Some(duration_id.into());
        self
    }

    /// Set convexity identifier for risk calculations
    pub fn with_convexity(mut self, convexity_id: impl Into<String>) -> Self {
        self.convexity_id = Some(convexity_id.into());
        self
    }

    /// Set contract size
    pub fn with_contract_size(mut self, size: f64) -> Self {
        self.contract_size = size;
        self
    }
}

impl UnderlyingParams for IndexUnderlyingParams {
    fn base_currency(&self) -> Currency {
        self.base_currency
    }

    fn primary_curve_id(&self) -> &str {
        // Default curve - could be enhanced to be configurable per index
        match self.base_currency {
            Currency::USD => "USD-OIS",
            Currency::EUR => "EUR-OIS",
            Currency::GBP => "GBP-OIS",
            Currency::JPY => "JPY-OIS",
            _ => "USD-OIS", // Fallback
        }
    }
}
