//! Unified market quote wrapper type.

use super::{CreditQuote, InflationQuote, RatesQuote, VolQuote};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Unified market quote that can be any instrument type.
/// Used when multiple quote types need to be handled together.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketQuote {
    /// Interest rate quotes
    Rates(RatesQuote),
    /// Credit quotes
    Credit(CreditQuote),
    /// Volatility quotes
    Vol(VolQuote),
    /// Inflation quotes
    Inflation(InflationQuote),
}

