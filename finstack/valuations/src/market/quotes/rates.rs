//! Interest rate market quote schema.

use super::ids::{Pillar, QuoteId};
use crate::market::conventions::ids::{IndexId, IrFutureContractId};
use finstack_core::dates::Date;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Market quote for interest rate instruments.
///
/// This enum represents all supported interest rate quote types: deposits, forward rate agreements
/// (FRAs), interest rate futures, and interest rate swaps. Each variant includes the necessary
/// identifiers, pillars, and market values for instrument construction.
///
/// # Examples
///
/// Deposit quote:
/// ```rust
/// use finstack_valuations::market::quotes::rates::RateQuote;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::conventions::ids::IndexId;
///
/// # fn example() -> finstack_core::Result<()> {
/// let quote = RateQuote::Deposit {
///     id: QuoteId::new("USD-SOFR-DEP-1M"),
///     index: IndexId::new("USD-SOFR-1M"),
///     pillar: Pillar::Tenor("1M".parse()?),
///     rate: 0.0525,
/// };
/// # Ok(())
/// # }
/// ```
///
/// Swap quote:
/// ```rust
/// use finstack_valuations::market::quotes::rates::RateQuote;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::conventions::ids::IndexId;
///
/// # fn example() -> finstack_core::Result<()> {
/// let quote = RateQuote::Swap {
///     id: QuoteId::new("USD-OIS-SWAP-5Y"),
///     index: IndexId::new("USD-SOFR-OIS"),
///     pillar: Pillar::Tenor("5Y".parse()?),
///     rate: 0.0450,
///     spread: None,
/// };
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RateQuote {
    /// Money market deposit rate.
    Deposit {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Rate index identifier (e.g. "USD-SOFR-3M").
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        index: IndexId,
        /// Maturity pillar (e.g. Tenor("3M") or Date("2024-01-01")).
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        pillar: Pillar,
        /// Rate value (decimal).
        rate: f64,
    },
    /// Forward Rate Agreement.
    Fra {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Rate index identifier.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        index: IndexId,
        /// Start date pillar.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        start: Pillar,
        /// End date pillar.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        end: Pillar,
        /// Rate value (decimal).
        rate: f64,
    },
    /// Interest Rate Future (price).
    Futures {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Future contract identifier (e.g. "CME:SR3").
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        contract: IrFutureContractId,
        /// Expiry date of the future.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        expiry: Date,
        /// Price of the future (e.g. 98.50).
        price: f64,
        /// Optional convexity adjustment (rate, decimal).
        #[serde(default)]
        convexity_adjustment: Option<f64>,
    },
    /// Interest Rate Swap (par rate).
    Swap {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Rate index identifier (floating leg).
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        index: IndexId,
        /// Maturity pillar of the swap.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        pillar: Pillar,
        /// Fixed rate (decimal) making the swap PV=0.
        rate: f64,
        /// Optional spread over the index (decimal).
        #[serde(default)]
        spread: Option<f64>,
    },
}

impl RateQuote {
    /// Get the unique identifier of the quote.
    ///
    /// # Returns
    ///
    /// A reference to the quote's [`QuoteId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::quotes::rates::RateQuote;
    /// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
    /// use finstack_valuations::market::conventions::ids::IndexId;
    ///
    /// # fn example() -> finstack_core::Result<()> {
    /// let quote = RateQuote::Deposit {
    ///     id: QuoteId::new("USD-SOFR-DEP-1M"),
    ///     index: IndexId::new("USD-SOFR-1M"),
    ///     pillar: Pillar::Tenor("1M".parse()?),
    ///     rate: 0.0525,
    /// };
    ///
    /// assert_eq!(quote.id().as_str(), "USD-SOFR-DEP-1M");
    /// # Ok(())
    /// # }
    /// ```
    pub fn id(&self) -> &QuoteId {
        match self {
            RateQuote::Deposit { id, .. } => id,
            RateQuote::Fra { id, .. } => id,
            RateQuote::Futures { id, .. } => id,
            RateQuote::Swap { id, .. } => id,
        }
    }

    /// Get the resolved value (rate or price) of the quote.
    ///
    /// # Returns
    ///
    /// For deposit, FRA, and swap quotes: the rate value (decimal).
    /// For futures quotes: the price value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::quotes::rates::RateQuote;
    /// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
    /// use finstack_valuations::market::conventions::ids::IndexId;
    ///
    /// # fn example() -> finstack_core::Result<()> {
    /// let quote = RateQuote::Deposit {
    ///     id: QuoteId::new("USD-SOFR-DEP-1M"),
    ///     index: IndexId::new("USD-SOFR-1M"),
    ///     pillar: Pillar::Tenor("1M".parse()?),
    ///     rate: 0.0525,
    /// };
    ///
    /// assert_eq!(quote.value(), 0.0525);
    /// # Ok(())
    /// # }
    /// ```
    pub fn value(&self) -> f64 {
        match self {
            RateQuote::Deposit { rate, .. } => *rate,
            RateQuote::Fra { rate, .. } => *rate,
            RateQuote::Futures { price, .. } => *price,
            RateQuote::Swap { rate, .. } => *rate,
        }
    }

    /// Create a new quote with the value bumped by `bump`.
    ///
    /// For rates (deposit, FRA, swap), `bump` is added to the rate (in decimal terms,
    /// e.g., `0.0001` for 1 basis point). For futures, `bump` is added directly to the price.
    ///
    /// # Arguments
    ///
    /// * `bump` - The amount to add to the quote value (decimal for rates, absolute for futures)
    ///
    /// # Returns
    ///
    /// A new `RateQuote` with the bumped value.
    ///
    /// # Examples
    ///
    /// Bumping a deposit rate:
    /// ```rust
    /// use finstack_valuations::market::quotes::rates::RateQuote;
    /// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
    /// use finstack_valuations::market::conventions::ids::IndexId;
    ///
    /// # fn example() -> finstack_core::Result<()> {
    /// let quote = RateQuote::Deposit {
    ///     id: QuoteId::new("USD-SOFR-DEP-1M"),
    ///     index: IndexId::new("USD-SOFR-1M"),
    ///     pillar: Pillar::Tenor("1M".parse()?),
    ///     rate: 0.0525,
    /// };
    ///
    /// // Bump by 1 basis point (0.0001)
    /// let bumped = quote.bump(0.0001);
    /// assert_eq!(bumped.value(), 0.0526);
    /// # Ok(())
    /// # }
    /// ```
    pub fn bump(&self, bump: f64) -> Self {
        match self {
            RateQuote::Deposit {
                id,
                index,
                pillar,
                rate,
            } => RateQuote::Deposit {
                id: id.clone(),
                index: index.clone(),
                pillar: pillar.clone(),
                rate: rate + bump,
            },
            RateQuote::Fra {
                id,
                index,
                start,
                end,
                rate,
            } => RateQuote::Fra {
                id: id.clone(),
                index: index.clone(),
                start: start.clone(),
                end: end.clone(),
                rate: rate + bump,
            },
            RateQuote::Futures {
                id,
                contract,
                expiry,
                price,
                convexity_adjustment,
            } => RateQuote::Futures {
                id: id.clone(),
                contract: contract.clone(),
                expiry: *expiry,
                price: price + bump,
                convexity_adjustment: *convexity_adjustment,
            },
            RateQuote::Swap {
                id,
                index,
                pillar,
                rate,
                spread,
            } => RateQuote::Swap {
                id: id.clone(),
                index: index.clone(),
                pillar: pillar.clone(),
                rate: rate + bump,
                spread: *spread,
            },
        }
    }
}
