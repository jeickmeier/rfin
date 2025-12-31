//! CDS market quote schema.

use super::ids::{Pillar, QuoteId};
use crate::market::conventions::ids::CdsConventionKey;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Market quote for credit default swap (CDS) instruments.
///
/// CDS quotes can be specified in two formats:
/// 1. **Par spread**: The spread that makes the CDS have zero present value
/// 2. **Upfront + running**: A fixed upfront payment plus a running spread
///
/// Both formats include recovery rate assumptions and reference entity information.
///
/// # Examples
///
/// Par spread quote:
/// ```rust
/// use finstack_valuations::market::quotes::cds::CdsQuote;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
/// use finstack_core::currency::Currency;
///
/// # fn example() -> finstack_core::Result<()> {
/// let quote = CdsQuote::CdsParSpread {
///     id: QuoteId::new("CDS-ABC-CORP-5Y"),
///     entity: "ABC Corp".to_string(),
///     convention: CdsConventionKey {
///         currency: Currency::USD,
///         doc_clause: CdsDocClause::Cr14,
///     },
///     pillar: Pillar::Tenor("5Y".parse()?),
///     spread_bp: 150.0,
///     recovery_rate: 0.40,
/// };
/// # Ok(())
/// # }
/// ```
///
/// Upfront quote:
/// ```rust
/// use finstack_valuations::market::quotes::cds::CdsQuote;
/// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
/// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
/// use finstack_core::currency::Currency;
///
/// # fn example() -> finstack_core::Result<()> {
/// let quote = CdsQuote::CdsUpfront {
///     id: QuoteId::new("CDS-ABC-CORP-5Y"),
///     entity: "ABC Corp".to_string(),
///     convention: CdsConventionKey {
///         currency: Currency::USD,
///         doc_clause: CdsDocClause::Cr14,
///     },
///     pillar: Pillar::Tenor("5Y".parse()?),
///     running_spread_bp: 500.0,
///     upfront_pct: 0.02, // 2% upfront
///     recovery_rate: 0.40,
/// };
/// # Ok(())
/// # }
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CdsQuote {
    /// Credit Default Swap (par spread).
    CdsParSpread {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Reference entity name.
        entity: String,
        /// Convention key (currency + doc clause).
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: CdsConventionKey,
        /// Maturity pillar.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        pillar: Pillar,
        /// Par spread in basis points (e.g. 100.0).
        spread_bp: f64,
        /// Recovery rate assumption (e.g. 0.40).
        recovery_rate: f64,
    },
    /// Credit Default Swap (upfront + running).
    CdsUpfront {
        /// Unique identifier for the quote.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        id: QuoteId,
        /// Reference entity name.
        entity: String,
        /// Convention key.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: CdsConventionKey,
        /// Maturity pillar.
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        pillar: Pillar,
        /// Running spread in basis points (e.g. 100.0 or 500.0).
        running_spread_bp: f64,
        /// Upfront payment percentage of notional (e.g. 0.01 for 1%).
        upfront_pct: f64,
        /// Recovery rate assumption.
        recovery_rate: f64,
    },
}

impl CdsQuote {
    /// Get the unique identifier of the quote.
    ///
    /// # Returns
    ///
    /// A reference to the quote's [`QuoteId`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::quotes::cds::CdsQuote;
    /// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
    /// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
    /// use finstack_core::currency::Currency;
    ///
    /// # fn example() -> finstack_core::Result<()> {
    /// let quote = CdsQuote::CdsParSpread {
    ///     id: QuoteId::new("CDS-ABC-CORP-5Y"),
    ///     entity: "ABC Corp".to_string(),
    ///     convention: CdsConventionKey {
    ///         currency: Currency::USD,
    ///         doc_clause: CdsDocClause::Cr14,
    ///     },
    ///     pillar: Pillar::Tenor("5Y".parse()?),
    ///     spread_bp: 150.0,
    ///     recovery_rate: 0.40,
    /// };
    ///
    /// assert_eq!(quote.id().as_str(), "CDS-ABC-CORP-5Y");
    /// # Ok(())
    /// # }
    /// ```
    pub fn id(&self) -> &QuoteId {
        match self {
            CdsQuote::CdsParSpread { id, .. } => id,
            CdsQuote::CdsUpfront { id, .. } => id,
        }
    }

    /// Create a new quote with the spread bumped.
    ///
    /// For par spread quotes, bumps `spread_bp`. For upfront quotes, bumps `running_spread_bp`.
    /// The upfront percentage remains unchanged.
    ///
    /// # Arguments
    ///
    /// * `bump_decimal` - The bump amount in decimal terms (e.g., `0.0001` for 1 basis point).
    ///   This is converted to basis points internally (multiplied by 10,000).
    ///
    /// # Returns
    ///
    /// A new `CdsQuote` with the bumped spread.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::quotes::cds::CdsQuote;
    /// use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
    /// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
    /// use finstack_core::currency::Currency;
    ///
    /// # fn example() -> finstack_core::Result<()> {
    /// let quote = CdsQuote::CdsParSpread {
    ///     id: QuoteId::new("CDS-ABC-CORP-5Y"),
    ///     entity: "ABC Corp".to_string(),
    ///     convention: CdsConventionKey {
    ///         currency: Currency::USD,
    ///         doc_clause: CdsDocClause::Cr14,
    ///     },
    ///     pillar: Pillar::Tenor("5Y".parse()?),
    ///     spread_bp: 150.0,
    ///     recovery_rate: 0.40,
    /// };
    ///
    /// // Bump by 1 basis point (0.0001 decimal)
    /// let bumped = quote.bump_spread_decimal(0.0001);
    /// # Ok(())
    /// # }
    /// ```
    pub fn bump_spread_decimal(&self, bump_decimal: f64) -> Self {
        let bump_bp = bump_decimal * 10_000.0;
        self.bump_spread_bp(bump_bp)
    }

    /// Bump by spread in basis points (e.g., `1.0` = 1bp).
    pub fn bump_spread_bp(&self, bump_bp: f64) -> Self {
        match self {
            CdsQuote::CdsParSpread {
                id,
                entity,
                convention,
                pillar,
                spread_bp,
                recovery_rate,
            } => CdsQuote::CdsParSpread {
                id: id.clone(),
                entity: entity.clone(),
                convention: convention.clone(),
                pillar: pillar.clone(),
                spread_bp: spread_bp + bump_bp,
                recovery_rate: *recovery_rate,
            },
            CdsQuote::CdsUpfront {
                id,
                entity,
                convention,
                pillar,
                running_spread_bp,
                upfront_pct,
                recovery_rate,
            } => CdsQuote::CdsUpfront {
                id: id.clone(),
                entity: entity.clone(),
                convention: convention.clone(),
                pillar: pillar.clone(),
                running_spread_bp: running_spread_bp + bump_bp,
                upfront_pct: *upfront_pct,
                recovery_rate: *recovery_rate,
            },
        }
    }
}
