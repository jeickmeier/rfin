//! Interest rate quote types for yield curve calibration.

use super::conventions::InstrumentConventions;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::prelude::*;
use finstack_core::types::IndexId;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Interest rate instrument quotes for yield curve calibration.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatesQuote {
    /// Deposit rate quote
    Deposit {
        /// Maturity date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Quoted rate (decimal)
        rate: f64,
        /// Day count convention
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        day_count: DayCount,
        /// Per-instrument conventions (settlement, calendar, etc.)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// Forward Rate Agreement quote
    FRA {
        /// Start date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        start: Date,
        /// End date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        end: Date,
        /// Quoted rate (decimal)
        rate: f64,
        /// Day count convention
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        day_count: DayCount,
        /// Per-instrument conventions (settlement, reset lag, etc.)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// Interest Rate Future quote
    Future {
        /// Expiry date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        expiry: Date,
        /// Contract price (e.g., 99.25 for 0.75% implied rate)
        price: f64,
        /// Contract specifications
        specs: FutureSpecs,
        /// Per-instrument conventions (reset lag, etc.)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// Interest Rate Swap quote
    Swap {
        /// Swap maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Par rate (decimal)
        rate: f64,
        /// Fixed leg frequency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        fixed_freq: Tenor,
        /// Float leg frequency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        float_freq: Tenor,
        /// Fixed leg day count
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        fixed_dc: DayCount,
        /// Float leg day count
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        float_dc: DayCount,
        /// Float leg index (e.g., "USD-SOFR", "3M-LIBOR")
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        index: IndexId,
        /// Whether this is an OIS (Overnight Index Swap) suitable for discount curve calibration.
        /// Set to true for overnight indices (SOFR, SONIA, €STR, TONA, etc.).
        #[serde(default)]
        is_ois: bool,
        /// Instrument-wide conventions (settlement days, etc.)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
        /// Fixed leg specific conventions (day count, payment calendar, business day convention)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        fixed_leg_conventions: InstrumentConventions,
        /// Float leg specific conventions (reset lag, fixing calendar, reset frequency)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        float_leg_conventions: InstrumentConventions,
    },
    /// Basis Swap quote for multi-curve construction
    BasisSwap {
        /// Swap maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Primary leg index (e.g., "3M-LIBOR", "3M-SOFR")
        primary_index: String,
        /// Reference leg index (e.g., "6M-LIBOR", "1M-SOFR")
        reference_index: String,
        /// Basis spread in basis points (primary pays reference + spread)
        spread_bp: f64,
        /// Primary leg frequency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        primary_freq: Tenor,
        /// Reference leg frequency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        reference_freq: Tenor,
        /// Primary leg day count
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        primary_dc: DayCount,
        /// Reference leg day count
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        reference_dc: DayCount,
        /// Currency for both legs
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Instrument-wide conventions (settlement days, etc.)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
        /// Primary leg specific conventions (reset lag, fixing calendar, reset frequency)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        primary_leg_conventions: InstrumentConventions,
        /// Reference leg specific conventions (reset lag, fixing calendar, reset frequency)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        reference_leg_conventions: InstrumentConventions,
    },
}

impl RatesQuote {
    /// Check if this quote requires a forward curve for pricing
    pub fn requires_forward_curve(&self) -> bool {
        match self {
            RatesQuote::Deposit { .. } => false, // Only needs discount curve
            RatesQuote::FRA { .. } => true,      // Needs forward curve for forward rate
            RatesQuote::Future { .. } => true,   // Needs forward curve for implied rate
            RatesQuote::Swap { .. } => true,     // Float leg needs forward curve
            RatesQuote::BasisSwap { .. } => true, // Both legs need forward curves
        }
    }

    /// Check if this quote is suitable for OIS discount curve calibration.
    ///
    /// Uses the explicit `is_ois` flag on swap quotes for classification.
    /// This method returns `true` for:
    /// - Deposits (always suitable for short-end discount curve)
    /// - Swaps with `is_ois: true` (overnight rate indices: SOFR, SONIA, €STR, etc.)
    ///
    /// Returns `false` for:
    /// - FRAs (require forward curves)
    /// - Futures (require forward curves)
    /// - Swaps with `is_ois: false` (term rates: LIBOR, EURIBOR, Term SOFR)
    /// - Basis swaps (require separate forward curves)
    pub fn is_ois_suitable(&self) -> bool {
        match self {
            RatesQuote::Deposit { .. } => true,
            RatesQuote::Swap { is_ois, .. } => *is_ois,
            _ => false,
        }
    }

    /// Unified maturity date for sorting and time-axis derivations.
    #[inline]
    pub fn maturity_date(&self) -> Date {
        match self {
            RatesQuote::Deposit { maturity, .. } => *maturity,
            RatesQuote::FRA { end, .. } => *end,
            RatesQuote::Future { expiry, specs, .. } => {
                expiry.add_months(specs.delivery_months as i32)
            }
            RatesQuote::Swap { maturity, .. } => *maturity,
            RatesQuote::BasisSwap { maturity, .. } => *maturity,
        }
    }

    /// Get per-instrument conventions for this quote.
    ///
    /// Returns the conventions specified on this quote, which may have
    /// all-None fields if no per-instrument overrides were specified.
    #[inline]
    pub fn conventions(&self) -> &InstrumentConventions {
        match self {
            RatesQuote::Deposit { conventions, .. } => conventions,
            RatesQuote::FRA { conventions, .. } => conventions,
            RatesQuote::Future { conventions, .. } => conventions,
            RatesQuote::Swap { conventions, .. } => conventions,
            RatesQuote::BasisSwap { conventions, .. } => conventions,
        }
    }

    /// Get the effective settlement days for this quote.
    ///
    /// Returns the per-instrument value if specified, otherwise None
    /// (caller should use currency default).
    #[inline]
    pub fn settlement_days(&self) -> Option<i32> {
        self.conventions().settlement_days
    }

    /// Get the effective payment delay for this quote.
    ///
    /// Returns the per-instrument value if specified, otherwise None.
    #[inline]
    pub fn payment_delay_days(&self) -> Option<i32> {
        self.conventions().payment_delay_days
    }

    /// Get the effective reset lag for this quote.
    ///
    /// Returns the per-instrument value if specified, otherwise None.
    #[inline]
    pub fn reset_lag(&self) -> Option<i32> {
        self.conventions().reset_lag
    }

    /// Get the calendar ID for this quote.
    ///
    /// Returns the per-instrument value if specified, otherwise None.
    #[inline]
    pub fn calendar_id(&self) -> Option<&str> {
        self.conventions().calendar_id.as_deref()
    }

    /// Get fixed leg conventions for Swap quotes.
    ///
    /// Returns Some for Swap variants, None for other quote types.
    #[inline]
    pub fn fixed_leg_conventions(&self) -> Option<&InstrumentConventions> {
        match self {
            RatesQuote::Swap {
                fixed_leg_conventions,
                ..
            } => Some(fixed_leg_conventions),
            _ => None,
        }
    }

    /// Get float leg conventions for Swap quotes.
    ///
    /// Returns Some for Swap variants, None for other quote types.
    #[inline]
    pub fn float_leg_conventions(&self) -> Option<&InstrumentConventions> {
        match self {
            RatesQuote::Swap {
                float_leg_conventions,
                ..
            } => Some(float_leg_conventions),
            _ => None,
        }
    }

    /// Get primary leg conventions for BasisSwap quotes.
    ///
    /// Returns Some for BasisSwap variants, None for other quote types.
    #[inline]
    pub fn primary_leg_conventions(&self) -> Option<&InstrumentConventions> {
        match self {
            RatesQuote::BasisSwap {
                primary_leg_conventions,
                ..
            } => Some(primary_leg_conventions),
            _ => None,
        }
    }

    /// Get reference leg conventions for BasisSwap quotes.
    ///
    /// Returns Some for BasisSwap variants, None for other quote types.
    #[inline]
    pub fn reference_leg_conventions(&self) -> Option<&InstrumentConventions> {
        match self {
            RatesQuote::BasisSwap {
                reference_leg_conventions,
                ..
            } => Some(reference_leg_conventions),
            _ => None,
        }
    }

    /// Create a new quote with the rate bumped by the given amount.
    ///
    /// Used for Jacobian calculation (sensitivity analysis).
    /// Preserves per-instrument conventions.
    pub fn bump(&self, amount: f64) -> Self {
        match self {
            RatesQuote::Deposit {
                maturity,
                rate,
                day_count,
                conventions,
            } => RatesQuote::Deposit {
                maturity: *maturity,
                rate: rate + amount,
                day_count: *day_count,
                conventions: conventions.clone(),
            },
            RatesQuote::FRA {
                start,
                end,
                rate,
                day_count,
                conventions,
            } => RatesQuote::FRA {
                start: *start,
                end: *end,
                rate: rate + amount,
                day_count: *day_count,
                conventions: conventions.clone(),
            },
            RatesQuote::Future {
                expiry,
                price,
                specs,
                conventions,
            } => RatesQuote::Future {
                expiry: *expiry,
                // For futures, price = 100 - rate * 100.
                // Bump rate by +amount => price decreases by amount * 100
                price: price - (amount * 100.0),
                specs: specs.clone(),
                conventions: conventions.clone(),
            },
            RatesQuote::Swap {
                maturity,
                rate,
                fixed_freq,
                float_freq,
                fixed_dc,
                float_dc,
                index,
                is_ois,
                conventions,
                fixed_leg_conventions,
                float_leg_conventions,
            } => RatesQuote::Swap {
                maturity: *maturity,
                rate: rate + amount,
                fixed_freq: *fixed_freq,
                float_freq: *float_freq,
                fixed_dc: *fixed_dc,
                float_dc: *float_dc,
                index: index.clone(),
                is_ois: *is_ois,
                conventions: conventions.clone(),
                fixed_leg_conventions: fixed_leg_conventions.clone(),
                float_leg_conventions: float_leg_conventions.clone(),
            },
            RatesQuote::BasisSwap {
                maturity,
                primary_index,
                reference_index,
                spread_bp,
                primary_freq,
                reference_freq,
                primary_dc,
                reference_dc,
                currency,
                conventions,
                primary_leg_conventions,
                reference_leg_conventions,
            } => RatesQuote::BasisSwap {
                maturity: *maturity,
                primary_index: primary_index.clone(),
                reference_index: reference_index.clone(),
                spread_bp: spread_bp + (amount * 10_000.0), // Convert decimal bump to bp
                primary_freq: *primary_freq,
                reference_freq: *reference_freq,
                primary_dc: *primary_dc,
                reference_dc: *reference_dc,
                currency: *currency,
                conventions: conventions.clone(),
                primary_leg_conventions: primary_leg_conventions.clone(),
                reference_leg_conventions: reference_leg_conventions.clone(),
            },
        }
    }

    /// Format a descriptive residual key for calibration reports.
    ///
    /// Generates a unique, human-readable key that identifies the instrument
    /// and its parameters for use in calibration residual tracking.
    ///
    /// # Arguments
    ///
    /// * `counter` - A unique counter value to ensure key uniqueness when
    ///   multiple quotes of the same type exist
    ///
    /// # Returns
    ///
    /// A formatted string key like "DEP-2025-03-15-Act360-000001" or
    /// "SWAP-USD-SOFR-3M-2027-01-15-fix6M-flt3M-000002"
    ///
    /// # Example
    ///
    /// ```ignore
    /// let key = quote.format_residual_key(0);
    /// residuals.insert(key, residual_value);
    /// ```
    pub fn format_residual_key(&self, counter: usize) -> String {
        match self {
            RatesQuote::Deposit {
                maturity,
                day_count,
                ..
            } => {
                format!("DEP-{}-{:?}-{:06}", maturity, day_count, counter)
            }
            RatesQuote::FRA {
                start,
                end,
                day_count,
                ..
            } => {
                format!("FRA-{}-{}-{:?}-{:06}", start, end, day_count, counter)
            }
            RatesQuote::Future { expiry, specs, .. } => {
                format!(
                    "FUT-{}-{}m-{:?}-{:06}",
                    expiry, specs.delivery_months, specs.day_count, counter
                )
            }
            RatesQuote::Swap {
                maturity,
                index,
                fixed_freq,
                float_freq,
                ..
            } => {
                format!(
                    "SWAP-{}-{}-fix{:?}-flt{:?}-{:06}",
                    index.as_str(),
                    maturity,
                    fixed_freq,
                    float_freq,
                    counter
                )
            }
            RatesQuote::BasisSwap {
                maturity,
                primary_index,
                reference_index,
                ..
            } => {
                format!(
                    "BASIS-{}-{}vs{}-{:06}",
                    maturity, primary_index, reference_index, counter
                )
            }
        }
    }

    /// Get the quote type as a string.
    pub fn get_type(&self) -> &'static str {
        match self {
            RatesQuote::Deposit { .. } => "Deposit",
            RatesQuote::FRA { .. } => "FRA",
            RatesQuote::Future { .. } => "Future",
            RatesQuote::Swap { .. } => "Swap",
            RatesQuote::BasisSwap { .. } => "BasisSwap",
        }
    }
}

/// Specifications for interest rate futures contracts.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FutureSpecs {
    /// Contract multiplier
    pub multiplier: f64,
    /// Face value
    pub face_value: f64,
    /// Number of delivery months
    pub delivery_months: u8,
    /// Day count convention
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub day_count: DayCount,
    /// Convexity adjustment (for long-dated futures)
    pub convexity_adjustment: Option<f64>,
    /// Tick size (minimum price increment, e.g., 0.0025 for STIR)
    #[serde(default = "default_tick_size")]
    pub tick_size: f64,
    /// Tick value (dollar value per tick, e.g., 6.25 for CME 3M SOFR)
    #[serde(default = "default_tick_value")]
    pub tick_value: f64,
}

fn default_tick_size() -> f64 {
    0.0025
}

fn default_tick_value() -> f64 {
    6.25
}

impl Default for FutureSpecs {
    fn default() -> Self {
        Self {
            multiplier: 1_000_000.0, // $1MM face value
            face_value: 1_000_000.0,
            delivery_months: 3,
            day_count: DayCount::Act360,
            convexity_adjustment: None,
            tick_size: default_tick_size(),
            tick_value: default_tick_value(),
        }
    }
}

