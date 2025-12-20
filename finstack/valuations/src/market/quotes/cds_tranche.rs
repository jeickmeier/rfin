//! CDS tranche market quote schema.

use super::ids::QuoteId;
use crate::market::conventions::ids::CdsConventionKey;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Market quote for CDS Index Tranches.
///
/// CDS tranches represent slices of credit risk on a CDS index, defined by attachment and
/// detachment points. Quotes include upfront payments and running spreads for pricing.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::quotes::cds_tranche::CdsTrancheQuote;
/// use finstack_valuations::market::quotes::ids::QuoteId;
/// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
/// use finstack_core::dates::Date;
/// use finstack_core::currency::Currency;
///
/// # fn example() -> finstack_core::Result<()> {
/// let quote = CdsTrancheQuote::CDSTranche {
///     id: QuoteId::new("CDX-IG-3-7"),
///     index: "CDX.NA.IG".to_string(),
///     attachment: 0.03,  // 3%
///     detachment: 0.07, // 7%
///     maturity: Date::from_calendar_date(2029, time::Month::June, 20).unwrap(),
///     upfront_pct: -2.5, // -2.5% upfront
///     running_spread_bp: 500.0,
///     convention: CdsConventionKey {
///         currency: Currency::USD,
///         doc_clause: CdsDocClause::Cr14,
///     },
/// };
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CdsTrancheQuote {
    /// CDS Index Tranche.
    CDSTranche {
        /// Unique identifier.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Index identifier (e.g. CDX.NA.HY).
        index: String,
        /// Attachment point (decimal, e.g. 0.03).
        attachment: f64,
        /// Detachment point (decimal, e.g. 0.07).
        detachment: f64,
        /// Maturity date.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: finstack_core::dates::Date,
        /// Upfront payment percentage.
        upfront_pct: f64,
        /// Running spread (bps).
        running_spread_bp: f64,
        /// Convention key (currency + doc clause).
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: CdsConventionKey,
    },
}

impl CdsTrancheQuote {
    /// Get the unique identifier of the quote.
    ///
    /// # Returns
    ///
    /// A reference to the quote's [`QuoteId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::quotes::cds_tranche::CdsTrancheQuote;
    /// use finstack_valuations::market::quotes::ids::QuoteId;
    /// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
    /// use finstack_core::dates::Date;
    /// use finstack_core::currency::Currency;
    ///
/// # fn example() -> finstack_core::Result<()> {
/// let quote = CdsTrancheQuote::CDSTranche {
///     id: QuoteId::new("CDX-IG-3-7"),
///     index: "CDX.NA.IG".to_string(),
///     attachment: 0.03,
///     detachment: 0.07,
///     maturity: Date::from_calendar_date(2029, time::Month::June, 20).unwrap(),
///     upfront_pct: -2.5,
///     running_spread_bp: 500.0,
///     convention: CdsConventionKey {
///         currency: Currency::USD,
///         doc_clause: CdsDocClause::Cr14,
///     },
/// };
///
/// assert_eq!(quote.id().as_str(), "CDX-IG-3-7");
/// # Ok(())
/// # }
    /// ```
    pub fn id(&self) -> &QuoteId {
        match self {
            CdsTrancheQuote::CDSTranche { id, .. } => id,
        }
    }

    /// Create a new quote with the running spread bumped.
    ///
    /// The upfront percentage remains unchanged.
    ///
    /// # Arguments
    ///
    /// * `bump_decimal` - The bump amount in decimal terms (e.g., `0.0001` for 1 basis point).
    ///   This is converted to basis points internally (multiplied by 10,000).
    ///
    /// # Returns
    ///
    /// A new `CdsTrancheQuote` with the bumped running spread.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::quotes::cds_tranche::CdsTrancheQuote;
    /// use finstack_valuations::market::quotes::ids::QuoteId;
    /// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
    /// use finstack_core::dates::Date;
    /// use finstack_core::currency::Currency;
    ///
/// # fn example() -> finstack_core::Result<()> {
/// let quote = CdsTrancheQuote::CDSTranche {
///     id: QuoteId::new("CDX-IG-3-7"),
///     index: "CDX.NA.IG".to_string(),
///     attachment: 0.03,
///     detachment: 0.07,
///     maturity: Date::from_calendar_date(2029, time::Month::June, 20).unwrap(),
///     upfront_pct: -2.5,
///     running_spread_bp: 500.0,
///     convention: CdsConventionKey {
///         currency: Currency::USD,
///         doc_clause: CdsDocClause::Cr14,
///     },
/// };
///
/// // Bump by 1 basis point
/// let bumped = quote.bump(0.0001);
/// # Ok(())
/// # }
    /// ```
    pub fn bump(&self, bump_decimal: f64) -> Self {
        let bump_bp = bump_decimal * 10_000.0;
        match self {
            CdsTrancheQuote::CDSTranche {
                id,
                index,
                attachment,
                detachment,
                maturity,
                upfront_pct,
                running_spread_bp,
                convention,
            } => CdsTrancheQuote::CDSTranche {
                id: id.clone(),
                index: index.clone(),
                attachment: *attachment,
                detachment: *detachment,
                maturity: *maturity,
                upfront_pct: *upfront_pct,
                running_spread_bp: running_spread_bp + bump_bp,
                convention: convention.clone(),
            },
        }
    }
}
