//! Inflation instrument quote types.
//!
//! Note: Copied from v1 for parallel implementation.

use super::conventions::InstrumentConventions;
use finstack_core::dates::{Date, Tenor};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Inflation instrument quotes.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)]
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
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
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
        /// Instrument-wide conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
        /// Fixed leg specific conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        fixed_leg_conventions: InstrumentConventions,
        /// Inflation leg specific conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        inflation_leg_conventions: InstrumentConventions,
    },
}

impl InflationQuote {
    /// Get per-instrument conventions for this quote.
    pub fn conventions(&self) -> &InstrumentConventions {
        match self {
            InflationQuote::InflationSwap { conventions, .. } => conventions,
            InflationQuote::YoYInflationSwap { conventions, .. } => conventions,
        }
    }

    /// Get fixed leg conventions for YoY inflation swap.
    pub fn fixed_leg_conventions(&self) -> Option<&InstrumentConventions> {
        match self {
            InflationQuote::YoYInflationSwap {
                fixed_leg_conventions,
                ..
            } => Some(fixed_leg_conventions),
            _ => None,
        }
    }

    /// Get inflation leg conventions for YoY inflation swap.
    pub fn inflation_leg_conventions(&self) -> Option<&InstrumentConventions> {
        match self {
            InflationQuote::YoYInflationSwap {
                inflation_leg_conventions,
                ..
            } => Some(inflation_leg_conventions),
            _ => None,
        }
    }

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
                conventions,
            } => InflationQuote::InflationSwap {
                maturity: *maturity,
                rate: rate + rate_bump,
                index: index.clone(),
                conventions: conventions.clone(),
            },
            InflationQuote::YoYInflationSwap {
                maturity,
                rate,
                index,
                frequency,
                conventions,
                fixed_leg_conventions,
                inflation_leg_conventions,
            } => InflationQuote::YoYInflationSwap {
                maturity: *maturity,
                rate: rate + rate_bump,
                index: index.clone(),
                frequency: *frequency,
                conventions: conventions.clone(),
                fixed_leg_conventions: fixed_leg_conventions.clone(),
                inflation_leg_conventions: inflation_leg_conventions.clone(),
            },
        }
    }
}
