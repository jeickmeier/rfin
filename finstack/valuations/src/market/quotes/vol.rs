//! Volatility quote types for surface calibration.
//!
//! Defines the volatility quote types used for surface calibration of options and swaptions.
//! Volatility quotes include strike, expiry, and implied volatility values for building
//! volatility surfaces.

use crate::instruments::OptionType;
use crate::market::conventions::ids::{OptionConventionId, SwaptionConventionId};
use finstack_core::dates::Date;
use finstack_core::types::UnderlyingId;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Volatility quotes for option and swaption surface calibration.
///
/// Supports two types of volatility quotes:
/// 1. **Option volatility**: For equity, commodity, or FX options with strike and expiry
/// 2. **Swaption volatility**: For interest rate swaptions with strike, expiry, and underlying swap maturity date
///
/// # Examples
///
/// Option volatility quote:
/// ```rust
/// use finstack_valuations::market::quotes::vol::VolQuote;
/// use finstack_valuations::market::conventions::ids::OptionConventionId;
/// use finstack_core::dates::Date;
/// use finstack_core::types::UnderlyingId;
///
/// let quote = VolQuote::OptionVol {
///     underlying: UnderlyingId::new("SPX"),
///     expiry: Date::from_calendar_date(2024, time::Month::December, 20).unwrap(),
///     strike: 4500.0,
///     vol: 0.20, // 20% implied volatility
///     option_type: OptionType::Call,
///     convention: OptionConventionId::new("USD-EQUITY"),
/// };
/// ```
///
/// Swaption volatility quote:
/// ```rust
/// use finstack_valuations::market::quotes::vol::VolQuote;
/// use finstack_valuations::market::conventions::ids::SwaptionConventionId;
/// use finstack_core::dates::Date;
///
/// let quote = VolQuote::SwaptionVol {
///     expiry: Date::from_calendar_date(2025, time::Month::June, 20).unwrap(),
///     maturity: Date::from_calendar_date(2030, time::Month::June, 20).unwrap(),
///     strike: 0.045, // 4.5% strike rate
///     vol: 0.15, // 15% implied volatility
///     quote_type: "Normal".to_string(),
///     convention: SwaptionConventionId::new("USD"),
/// };
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
        /// Option type (Call or Put).
        option_type: OptionType,
        /// Per-instrument conventions
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        convention: OptionConventionId,
    },
    /// Interest rate swaption implied volatility quote.
    SwaptionVol {
        /// Option expiry
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        expiry: Date,
        /// Underlying swap maturity date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
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
    /// Create a new quote with the volatility bumped by an absolute amount.
    ///
    /// # Arguments
    ///
    /// * `vol_bump` - The bump amount in volatility terms (e.g., `0.01` for a +1 vol point bump)
    ///
    /// # Returns
    ///
    /// A new `VolQuote` with the bumped volatility.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::market::quotes::vol::VolQuote;
    /// use finstack_valuations::market::conventions::ids::OptionConventionId;
    /// use finstack_core::dates::Date;
    /// use finstack_core::types::UnderlyingId;
    ///
    /// let quote = VolQuote::OptionVol {
    ///     underlying: UnderlyingId::new("SPX"),
    ///     expiry: Date::from_calendar_date(2024, time::Month::December, 20).unwrap(),
    ///     strike: 4500.0,
    ///     vol: 0.20,
    ///     option_type: OptionType::Call,
    ///     convention: OptionConventionId::new("USD-EQUITY"),
    /// };
    ///
    /// // Bump by 1 vol point
    /// let bumped = quote.bump_vol_absolute(0.01);
    /// ```
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
                option_type: *option_type,
                convention: convention.clone(),
            },
            VolQuote::SwaptionVol {
                expiry,
                maturity,
                strike,
                vol,
                quote_type,
                convention,
            } => VolQuote::SwaptionVol {
                expiry: *expiry,
                maturity: *maturity,
                strike: *strike,
                vol: vol + vol_bump,
                quote_type: quote_type.clone(),
                convention: convention.clone(),
            },
        }
    }
}
