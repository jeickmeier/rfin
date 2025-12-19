//! Volatility quote types for surface calibration.
//!
//! Defines the volatility quote types used for surface calibration.

use crate::market::conventions::ids::{OptionConventionId, SwaptionConventionId};
use finstack_core::dates::Date;
use finstack_core::types::UnderlyingId;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Volatility quotes for option and swaption surface calibration.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)]
pub enum VolQuote {
    /// Equity or commodity option implied volatility quote.
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
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: OptionConventionId,
    },
    /// Interest rate swaption implied volatility quote.
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
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: SwaptionConventionId,
    },
}

impl VolQuote {
    /// Create a new quote with the volatility bumped by an **absolute** amount.
    ///
    /// The `vol_bump` parameter is specified in volatility terms (e.g., `0.01`
    /// for a +1 vol point bump).
    pub fn bump_vol_absolute(&self, vol_bump: f64) -> Self {
        match self {
            VolQuote::OptionVol {
                underlying,
                expiry,
                strike,
                vol,
                option_type,
                convention,
            } => VolQuote::OptionVol {
                underlying: underlying.clone(),
                expiry: *expiry,
                strike: *strike,
                vol: vol + vol_bump,
                option_type: option_type.clone(),
                convention: convention.clone(),
            },
            VolQuote::SwaptionVol {
                expiry,
                tenor,
                strike,
                vol,
                quote_type,
                convention,
            } => VolQuote::SwaptionVol {
                expiry: *expiry,
                tenor: *tenor,
                strike: *strike,
                vol: vol + vol_bump,
                quote_type: quote_type.clone(),
                convention: convention.clone(),
            },
        }
    }
}
