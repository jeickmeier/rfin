//! Volatility quote types for surface calibration.

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
pub enum VolQuote {
    /// Option implied volatility quote
    OptionVol {
        /// Underlying identifier
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        underlying: UnderlyingId,
        /// Option expiry
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        expiry: Date,
        /// Strike (rate for swaptions, price for equity/FX)
        strike: f64,
        /// Implied volatility
        vol: f64,
        /// Option type ("Call", "Put", "Straddle")
        option_type: String,
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
        /// Quote type (ATM, OTM, etc.)
        quote_type: String,
    },
}

