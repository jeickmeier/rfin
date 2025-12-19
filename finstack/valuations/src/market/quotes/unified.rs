//! Legacy unified market quote wrapper.

use super::cds::CdsQuote;
use super::cds_tranche::CdsTrancheQuote;
use super::rates::RateQuote;
use serde::{Deserialize, Serialize};

/// Unified Market Quote type.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "class", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketQuote {
    /// Interest rate quotes.
    Rates(RateQuote),
    /// Credit quotes.
    Cds(CdsQuote),
    /// CDS Tranches
    CdsTranche(CdsTrancheQuote),
    // Inflation to be added when ready
}
