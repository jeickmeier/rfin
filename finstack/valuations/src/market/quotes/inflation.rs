//! Inflation instrument quote types.
//!
//! Inflation instrument quotes for inflation curve calibration.

use crate::market::conventions::ids::InflationSwapConventionId;
use finstack_core::dates::{Date, Tenor};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Inflation instrument quotes for CPI and inflation curve calibration.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)]
pub enum InflationQuote {
    /// Zero-coupon inflation swap (ZCIS) quote.
    InflationSwap {
        /// Swap maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Fixed rate (decimal)
        rate: f64,
        /// Inflation index identifier
        index: String,
        /// Per-instrument conventions
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: InflationSwapConventionId,
    },
    /// Year-on-year (YoY) inflation swap quote.
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
        /// Instrument-wide conventions
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: InflationSwapConventionId,
    },
}

impl InflationQuote {
    /// Get maturity date for this quote if applicable.
    pub fn maturity_date(&self) -> Option<Date> {
        match self {
            InflationQuote::InflationSwap { maturity, .. } => Some(*maturity),
            InflationQuote::YoYInflationSwap { maturity, .. } => Some(*maturity),
        }
    }

    /// Create a new quote with the inflation rate bumped by a **decimal rate** amount.
    ///
    /// The `rate_bump` parameter is specified in decimal terms (e.g., `0.0001`
    /// for 1 basis point).
    pub fn bump_rate_decimal(&self, rate_bump: f64) -> Self {
        match self {
            InflationQuote::InflationSwap {
                maturity,
                rate,
                index,
                convention,
            } => InflationQuote::InflationSwap {
                maturity: *maturity,
                rate: rate + rate_bump,
                index: index.clone(),
                convention: convention.clone(),
            },
            InflationQuote::YoYInflationSwap {
                maturity,
                rate,
                index,
                frequency,
                convention,
            } => InflationQuote::YoYInflationSwap {
                maturity: *maturity,
                rate: rate + rate_bump,
                index: index.clone(),
                frequency: *frequency,
                convention: convention.clone(),
            },
        }
    }
}
