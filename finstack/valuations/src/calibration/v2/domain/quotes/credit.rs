//! Credit instrument quote types for hazard curve and correlation calibration.
//!
//! Note: Copied from v1 for parallel implementation.

use super::conventions::InstrumentConventions;
use finstack_core::dates::Date;
use finstack_core::prelude::*;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Credit instrument quotes for hazard curve and correlation calibration.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CreditQuote {
    /// CDS par spread quote
    CDS {
        /// Reference entity
        entity: String,
        /// CDS maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Par spread in basis points
        spread_bp: f64,
        /// Recovery rate assumption
        recovery_rate: f64,
        /// Currency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// CDS upfront quote
    CDSUpfront {
        /// Reference entity
        entity: String,
        /// CDS maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Upfront payment (% of notional)
        upfront_pct: f64,
        /// Running spread in basis points
        running_spread_bp: f64,
        /// Recovery rate assumption
        recovery_rate: f64,
        /// Currency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// CDS Tranche quote
    CDSTranche {
        /// Index name
        index: String,
        /// Attachment point (%)
        attachment: f64,
        /// Detachment point (%)
        detachment: f64,
        /// Maturity date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Upfront payment (% of notional)
        upfront_pct: f64,
        /// Running spread (bps)
        running_spread_bp: f64,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
}

impl CreditQuote {
    /// Get per-instrument conventions for this quote.
    pub fn conventions(&self) -> &InstrumentConventions {
        match self {
            CreditQuote::CDS { conventions, .. } => conventions,
            CreditQuote::CDSUpfront { conventions, .. } => conventions,
            CreditQuote::CDSTranche { conventions, .. } => conventions,
        }
    }

    /// Get maturity date for this quote if applicable.
    pub fn maturity_date(&self) -> Option<Date> {
        match self {
            CreditQuote::CDS { maturity, .. } => Some(*maturity),
            CreditQuote::CDSUpfront { maturity, .. } => Some(*maturity),
            CreditQuote::CDSTranche { maturity, .. } => Some(*maturity),
        }
    }
}

