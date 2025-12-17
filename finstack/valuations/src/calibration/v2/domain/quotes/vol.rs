//! Volatility quote types for surface calibration.
//!
//! Note: Copied from v1 for parallel implementation.

use super::conventions::InstrumentConventions;
use finstack_core::dates::Date;
use finstack_core::types::UnderlyingId;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Volatility quotes for surface calibration.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)]
pub enum VolQuote {
    /// Option implied volatility quote
    OptionVol {
        /// Underlying identifier
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        underlying: UnderlyingId,
        /// Option expiry
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        expiry: Date,
        /// Strike
        strike: f64,
        /// Implied volatility
        vol: f64,
        /// Option type ("Call", "Put", "Straddle")
        option_type: String,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// Swaption implied volatility
    SwaptionVol {
        /// Option expiry
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        expiry: Date,
        /// Underlying swap tenor
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        tenor: Date,
        /// Strike rate
        strike: f64,
        /// Implied volatility
        vol: f64,
        /// Quote type
        quote_type: String,
        /// Option exercise conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
        /// Underlying swap fixed leg conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        fixed_leg_conventions: InstrumentConventions,
        /// Underlying swap float leg conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        float_leg_conventions: InstrumentConventions,
    },
}

impl VolQuote {
    /// Get per-instrument conventions for this quote.
    pub fn conventions(&self) -> &InstrumentConventions {
        match self {
            VolQuote::OptionVol { conventions, .. } => conventions,
            VolQuote::SwaptionVol { conventions, .. } => conventions,
        }
    }

    /// Get fixed leg conventions for swaption quotes.
    pub fn fixed_leg_conventions(&self) -> Option<&InstrumentConventions> {
        match self {
            VolQuote::SwaptionVol {
                fixed_leg_conventions,
                ..
            } => Some(fixed_leg_conventions),
            _ => None,
        }
    }

    /// Get float leg conventions for swaption quotes.
    pub fn float_leg_conventions(&self) -> Option<&InstrumentConventions> {
        match self {
            VolQuote::SwaptionVol {
                float_leg_conventions,
                ..
            } => Some(float_leg_conventions),
            _ => None,
        }
    }
}
