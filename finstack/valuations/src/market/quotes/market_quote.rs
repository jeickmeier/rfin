//! Unified MarketQuote enum.
//!
//! This module defines the top-level enum for all supported market quotes. The `MarketQuote`
//! enum provides a unified interface for working with quotes across all instrument types,
//! enabling generic calibration workflows and quote processing.

use super::cds::CdsQuote;
use super::cds_tranche::CdsTrancheQuote;
use super::inflation::InflationQuote;
use super::rates::RateQuote;
use super::vol::VolQuote;
use finstack_core::InputError;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Polymorphic container for all supported market quote types.
///
/// This enum unifies all quote types into a single type, enabling generic quote processing,
/// serialization, and calibration workflows. Each variant wraps a specific quote type.
///
/// # Examples
///
/// Creating a rates quote:
/// ```rust
/// use finstack_valuations::market::quotes::market_quote::MarketQuote;
/// use finstack_valuations::market::quotes::rates::RateQuote;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::conventions::ids::IndexId;
///
/// # fn example() -> finstack_core::Result<()> {
/// let rate_quote = RateQuote::Deposit {
///     id: QuoteId::new("USD-SOFR-DEP-1M"),
///     index: IndexId::new("USD-SOFR-1M"),
///     pillar: Pillar::Tenor("1M".parse()?),
///     rate: 0.0525,
/// };
///
/// let market_quote = MarketQuote::Rates(rate_quote);
/// # Ok(())
/// # }
/// ```
///
/// Bumping a quote:
/// ```rust
/// use finstack_valuations::market::quotes::market_quote::MarketQuote;
/// use finstack_valuations::market::quotes::rates::RateQuote;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::conventions::ids::IndexId;
///
/// # fn example() -> finstack_core::Result<()> {
/// let quote = MarketQuote::Rates(RateQuote::Deposit {
///     id: QuoteId::new("USD-SOFR-DEP-1M"),
///     index: IndexId::new("USD-SOFR-1M"),
///     pillar: Pillar::Tenor("1M".parse()?),
///     rate: 0.0525,
/// });
///
/// // Bump by 1 basis point
/// let bumped = quote.bump(0.0001);
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "class", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketQuote {
    /// Interest rate instruments
    Rates(RateQuote),
    /// Credit default swaps
    Cds(CdsQuote),
    /// CDS Tranches
    CdsTranche(CdsTrancheQuote),
    /// Inflation instruments
    Inflation(InflationQuote),
    /// Volatility instruments
    Vol(VolQuote),
}

/// Explicit bump units for market quotes.
#[derive(Clone, Copy, Debug)]
pub enum MarketQuoteBump {
    /// Rate expressed in decimal units (e.g., 0.0001 = 1bp).
    RateDecimal(f64),
    /// Rate expressed in basis-point units (e.g., 1.0 = 1bp).
    RateBp(f64),
    /// Spread expressed in decimal units (e.g., 0.0001 = 1bp).
    SpreadDecimal(f64),
    /// Spread expressed in basis-point units (e.g., 1.0 = 1bp).
    SpreadBp(f64),
    /// Absolute volatility bump (e.g., 0.01 = +1 vol point).
    VolAbsolute(f64),
}

impl MarketQuote {
    /// Bump the quote using explicit unit semantics.
    pub fn bump_with(&self, bump: MarketQuoteBump) -> finstack_core::Result<Self> {
        match (self, bump) {
            (MarketQuote::Rates(q), MarketQuoteBump::RateDecimal(b)) => {
                Ok(MarketQuote::Rates(q.bump_rate_decimal(b)))
            }
            (MarketQuote::Rates(q), MarketQuoteBump::RateBp(bp)) => {
                Ok(MarketQuote::Rates(q.bump_rate_bp(bp)))
            }

            (MarketQuote::Inflation(q), MarketQuoteBump::RateDecimal(b)) => {
                Ok(MarketQuote::Inflation(q.bump_rate_decimal(b)))
            }
            (MarketQuote::Inflation(q), MarketQuoteBump::RateBp(bp)) => {
                Ok(MarketQuote::Inflation(q.bump_rate_decimal(bp / 10_000.0)))
            }

            (MarketQuote::Cds(q), MarketQuoteBump::SpreadDecimal(b)) => {
                Ok(MarketQuote::Cds(q.bump_spread_decimal(b)))
            }
            (MarketQuote::Cds(q), MarketQuoteBump::SpreadBp(bp)) => {
                Ok(MarketQuote::Cds(q.bump_spread_bp(bp)))
            }

            (MarketQuote::CdsTranche(q), MarketQuoteBump::SpreadDecimal(b)) => {
                Ok(MarketQuote::CdsTranche(q.bump_spread_decimal(b)))
            }
            (MarketQuote::CdsTranche(q), MarketQuoteBump::SpreadBp(bp)) => {
                Ok(MarketQuote::CdsTranche(q.bump_spread_bp(bp)))
            }

            (MarketQuote::Vol(q), MarketQuoteBump::VolAbsolute(b)) => {
                Ok(MarketQuote::Vol(q.bump_vol_absolute(b)))
            }

            _ => Err(finstack_core::Error::from(InputError::Invalid)),
        }
    }

    /// Convenience wrapper for decimal rate bumps (e.g., `0.0001` = 1bp).
    pub fn bump_rate_decimal(&self, bump: f64) -> finstack_core::Result<Self> {
        self.bump_with(MarketQuoteBump::RateDecimal(bump))
    }

    /// Convenience wrapper for rate bumps in basis-point units (e.g., `1.0` = 1bp).
    pub fn bump_rate_bp(&self, bump_bp: f64) -> finstack_core::Result<Self> {
        self.bump_with(MarketQuoteBump::RateBp(bump_bp))
    }

    /// Convenience wrapper for spread bumps in decimal units (e.g., `0.0001` = 1bp).
    pub fn bump_spread_decimal(&self, bump: f64) -> finstack_core::Result<Self> {
        self.bump_with(MarketQuoteBump::SpreadDecimal(bump))
    }

    /// Convenience wrapper for spread bumps in basis-point units (e.g., `1.0` = 1bp).
    pub fn bump_spread_bp(&self, bump_bp: f64) -> finstack_core::Result<Self> {
        self.bump_with(MarketQuoteBump::SpreadBp(bump_bp))
    }

    /// Convenience wrapper for absolute volatility bumps (e.g., `0.01` = +1 vol point).
    pub fn bump_vol_absolute(&self, bump: f64) -> finstack_core::Result<Self> {
        self.bump_with(MarketQuoteBump::VolAbsolute(bump))
    }

    /// Deprecated: use explicit bump helpers with units.
    #[deprecated(note = "use bump_rate_decimal/bp, bump_spread_decimal/bp, or bump_vol_absolute")]
    pub fn bump(&self, amount: f64) -> Self {
        match self {
            // These transformations are infallible for the matching quote class; the
            // Result-returning APIs exist to enforce explicit bump unit semantics.
            MarketQuote::Rates(q) => MarketQuote::Rates(q.bump_rate_decimal(amount)),
            MarketQuote::Inflation(q) => MarketQuote::Inflation(q.bump_rate_decimal(amount)),
            MarketQuote::Cds(q) => MarketQuote::Cds(q.bump_spread_decimal(amount)),
            MarketQuote::CdsTranche(q) => MarketQuote::CdsTranche(q.bump_spread_decimal(amount)),
            MarketQuote::Vol(q) => MarketQuote::Vol(q.bump_vol_absolute(amount)),
        }
    }
}

/// Trait for filtering quote collections into specific types.
///
/// This trait enables extracting quotes of a specific type from a heterogeneous collection
/// of `MarketQuote` instances. Useful for calibration workflows that need to process
/// quotes by instrument class.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::quotes::market_quote::{ExtractQuotes, MarketQuote};
/// use finstack_valuations::market::quotes::rates::RateQuote;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::conventions::ids::IndexId;
///
/// # fn example() -> finstack_core::Result<()> {
/// let quotes = vec![
///     MarketQuote::Rates(RateQuote::Deposit {
///         id: QuoteId::new("USD-SOFR-DEP-1M"),
///         index: IndexId::new("USD-SOFR-1M"),
///         pillar: Pillar::Tenor("1M".parse()?),
///         rate: 0.0525,
///     }),
///     // ... other quote types
/// ];
///
/// // Extract only rate quotes
/// let rate_quotes: Vec<RateQuote> = quotes.extract_quotes();
/// # Ok(())
/// # }
/// ```
pub trait ExtractQuotes<T> {
    /// Extract all quotes matching type `T` from the collection.
    ///
    /// # Returns
    ///
    /// A vector containing all quotes of type `T` from the collection.
    fn extract_quotes(&self) -> Vec<T>;
}

impl ExtractQuotes<RateQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<RateQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Rates(rq) => Some(rq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<CdsQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<CdsQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Cds(cq) => Some(cq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<CdsTrancheQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<CdsTrancheQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::CdsTranche(ctq) => Some(ctq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<InflationQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<InflationQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Inflation(iq) => Some(iq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<VolQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<VolQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Vol(vq) => Some(vq.clone()),
                _ => None,
            })
            .collect()
    }
}
