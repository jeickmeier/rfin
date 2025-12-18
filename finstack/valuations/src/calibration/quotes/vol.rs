//! Volatility quote types for surface calibration.
//!
//! Defines the volatility quote types used for surface calibration.

use super::conventions::InstrumentConventions;
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
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
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
                conventions,
            } => VolQuote::OptionVol {
                underlying: underlying.clone(),
                expiry: *expiry,
                strike: *strike,
                vol: vol + vol_bump,
                option_type: option_type.clone(),
                conventions: conventions.clone(),
            },
            VolQuote::SwaptionVol {
                expiry,
                tenor,
                strike,
                vol,
                quote_type,
                conventions,
                fixed_leg_conventions,
                float_leg_conventions,
            } => VolQuote::SwaptionVol {
                expiry: *expiry,
                tenor: *tenor,
                strike: *strike,
                vol: vol + vol_bump,
                quote_type: quote_type.clone(),
                conventions: conventions.clone(),
                fixed_leg_conventions: fixed_leg_conventions.clone(),
                float_leg_conventions: float_leg_conventions.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use finstack_core::types::UnderlyingId;
    use time::Month;

    fn date(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).expect("valid date")
    }

    #[test]
    fn bump_vol_absolute_adjusts_vol_field() {
        let q = VolQuote::OptionVol {
            underlying: UnderlyingId::from("SPX"),
            expiry: date(2030, Month::January, 1),
            strike: 100.0,
            vol: 0.20,
            option_type: "Call".to_string(),
            conventions: InstrumentConventions::default(),
        };

        let bumped = q.bump_vol_absolute(0.01);
        match bumped {
            VolQuote::OptionVol { vol, .. } => assert!((vol - 0.21).abs() < 1e-12),
            _ => panic!("unexpected variant"),
        }
    }
}
