use super::ids::QuoteId;
use crate::market::conventions::ids::CdsConventionKey;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Market quote for CDS Index Tranches.
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
    pub fn id(&self) -> &QuoteId {
        match self {
            CdsTrancheQuote::CDSTranche { id, .. } => id,
        }
    }

    /// Create a new quote with the spread bumped.
    ///
    /// For tranches, bumps `running_spread_bp`.
    ///
    /// The `bump` argument is in basis points.
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
