//! Unified MarketQuote enum.
//!
//! This module defines the top-level enum for all supported market quotes. The `MarketQuote`
//! enum provides a unified interface for working with quotes across all instrument types,
//! enabling generic calibration workflows and quote processing.

use super::bond::BondQuote;
use super::cds::CdsQuote;
use super::cds_tranche::CDSTrancheQuote;
use super::fx::FxQuote;
use super::inflation::InflationQuote;
use super::rates::RateQuote;
use super::vol::VolQuote;
use super::xccy::XccyQuote;
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
/// // Bump by 1 basis point (0.0001 in decimal)
/// let bumped = quote.bump_rate_decimal(0.0001)?;
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "class", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketQuote {
    /// Bond instruments
    Bond(BondQuote),
    /// Interest rate instruments
    Rates(RateQuote),
    /// Credit default swaps
    Cds(CdsQuote),
    /// CDS Tranches
    #[serde(rename = "cds_tranche")]
    CDSTranche(CDSTrancheQuote),
    /// FX instruments
    Fx(FxQuote),
    /// Inflation instruments
    Inflation(InflationQuote),
    /// Volatility instruments
    Vol(VolQuote),
    /// Cross-currency swap instruments
    Xccy(XccyQuote),
}

/// Explicit bump units for market quotes.
#[derive(Debug, Clone, Copy)]
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

            (MarketQuote::CDSTranche(q), MarketQuoteBump::SpreadDecimal(b)) => {
                Ok(MarketQuote::CDSTranche(q.bump_spread_decimal(b)))
            }
            (MarketQuote::CDSTranche(q), MarketQuoteBump::SpreadBp(bp)) => {
                Ok(MarketQuote::CDSTranche(q.bump_spread_bp(bp)))
            }

            (MarketQuote::Vol(q), MarketQuoteBump::VolAbsolute(b)) => {
                Ok(MarketQuote::Vol(q.bump_vol_absolute(b)))
            }
            (MarketQuote::Fx(q), MarketQuoteBump::RateDecimal(b)) => {
                Ok(MarketQuote::Fx(q.bump_rate_decimal(b)))
            }

            (MarketQuote::Xccy(q), MarketQuoteBump::SpreadDecimal(b)) => {
                Ok(MarketQuote::Xccy(q.bump_spread_decimal(b)))
            }
            (MarketQuote::Xccy(q), MarketQuoteBump::SpreadBp(bp)) => {
                Ok(MarketQuote::Xccy(q.bump_spread_bp(bp)))
            }

            (MarketQuote::Bond(q), MarketQuoteBump::RateDecimal(b)) => {
                Ok(MarketQuote::Bond(q.bump_value_decimal(b)))
            }
            (MarketQuote::Bond(q), MarketQuoteBump::RateBp(bp)) => {
                Ok(MarketQuote::Bond(q.bump_value_bp(bp)))
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
}

/// Trait for filtering quote collections into specific types (owned).
pub(crate) trait ExtractQuotes<T> {
    fn extract_quotes(&self) -> Vec<T>;
}

/// Borrowing variant to avoid cloning when possible.
pub trait ExtractQuoteRefs<'a, T> {
    /// Extract borrowed quotes of a specific type from a heterogeneous collection.
    fn extract_quote_refs(&'a self) -> Vec<&'a T>;
}

macro_rules! impl_extract_quotes {
    ($quote_type:ty, $variant:ident) => {
        impl ExtractQuotes<$quote_type> for [MarketQuote] {
            fn extract_quotes(&self) -> Vec<$quote_type> {
                self.iter()
                    .filter_map(|q| match q {
                        MarketQuote::$variant(inner) => Some(inner.clone()),
                        _ => None,
                    })
                    .collect()
            }
        }

        impl ExtractQuoteRefs<'_, $quote_type> for [MarketQuote] {
            fn extract_quote_refs(&self) -> Vec<&$quote_type> {
                self.iter()
                    .filter_map(|q| match q {
                        MarketQuote::$variant(inner) => Some(inner),
                        _ => None,
                    })
                    .collect()
            }
        }
    };
}

impl_extract_quotes!(RateQuote, Rates);
impl_extract_quotes!(BondQuote, Bond);
impl_extract_quotes!(CdsQuote, Cds);
impl_extract_quotes!(CDSTrancheQuote, CDSTranche);
impl_extract_quotes!(InflationQuote, Inflation);
impl_extract_quotes!(FxQuote, Fx);
impl_extract_quotes!(VolQuote, Vol);
impl_extract_quotes!(XccyQuote, Xccy);
