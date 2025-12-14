//! Inflation instrument quote types.

use finstack_core::dates::{Date, Tenor};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Inflation instrument quotes.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InflationQuote {
    /// Zero-coupon inflation swap quote
    InflationSwap {
        /// Swap maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Fixed rate (decimal)
        rate: f64,
        /// Inflation index identifier
        index: String,
    },
    /// Year-on-year inflation swap
    YoYInflationSwap {
        /// Swap maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Fixed rate (decimal)
        rate: f64,
        /// Inflation index identifier
        index: String,
        /// Payment frequency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        frequency: Tenor,
    },
}

