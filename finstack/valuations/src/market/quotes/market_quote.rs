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

impl MarketQuote {
    /// Bump the quote in its natural market units.
    ///
    /// The `amount` parameter is interpreted per quote type:
    /// - **Rates**: Decimal rate bump (e.g., `0.0001` = 1bp)
    /// - **Cds**: Decimal-to-bp conversion (`spread_bp += amount * 10_000`)
    /// - **CdsTranche**: Decimal-to-bp conversion
    /// - **Inflation**: Decimal rate bump (e.g., `0.0001` = 1bp)
    /// - **Vol**: Absolute vol bump (e.g., `0.01` = +1 vol point)
    ///
    /// # Arguments
    ///
    /// * `amount` - The bump amount, interpreted according to quote type
    ///
    /// # Returns
    ///
    /// A new `MarketQuote` with the bumped value.
    ///
    /// # Examples
    ///
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
    pub fn bump(&self, amount: f64) -> Self {
        match self {
            MarketQuote::Rates(q) => MarketQuote::Rates(q.bump(amount)),
            MarketQuote::Cds(q) => MarketQuote::Cds(q.bump(amount)),
            MarketQuote::CdsTranche(q) => MarketQuote::CdsTranche(q.bump(amount)),
            MarketQuote::Inflation(q) => MarketQuote::Inflation(q.bump_rate_decimal(amount)),
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
