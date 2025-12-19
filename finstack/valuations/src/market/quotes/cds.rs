use super::ids::{Pillar, QuoteId};
use crate::market::conventions::ids::CdsConventionKey;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Market quote for credit instruments.
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
    pub fn id(&self) -> &QuoteId {
        match self {
            CdsQuote::CdsParSpread { id, .. } => id,
            CdsQuote::CdsUpfront { id, .. } => id,
        }
    }

    /// Create a new quote with the spread bumped.
    ///
    /// For par spread quotes, bumps `spread_bp`.
    /// For upfront quotes, bumps `running_spread_bp`.
    ///
    /// The `bump` argument is in basis points.
    pub fn bump(&self, bump_decimal: f64) -> Self {
        let bump_bp = bump_decimal * 10_000.0;
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
