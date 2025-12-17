//! Unified market quote wrapper type.
//!
//! Note: Copied from v1 for parallel implementation.

use super::{CreditQuote, InflationQuote, RatesQuote, VolQuote};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Unified market quote that can be any instrument type.
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

impl MarketQuote {
    /// Get the underlying quote type name.
    pub fn quote_type(&self) -> &'static str {
        match self {
            MarketQuote::Rates(q) => q.get_type(),
            MarketQuote::Credit(_) => "Credit",
            MarketQuote::Vol(_) => "Vol",
            MarketQuote::Inflation(_) => "Inflation",
        }
    }

    /// Bump the underlying rate quote by a decimal rate amount (e.g., 0.0001 = 1bp).
    ///
    /// Currently only supported for RatesQuote; all others are returned unchanged.
    pub fn bump(&self, amount: f64) -> Self {
        match self {
            MarketQuote::Rates(q) => MarketQuote::Rates(q.bump_rate_decimal(amount)),
            _ => self.clone(), // No-op for others for now
        }
    }
}
