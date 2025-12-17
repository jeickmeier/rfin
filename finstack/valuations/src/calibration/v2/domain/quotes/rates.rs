//! Interest rate quote types for yield curve calibration.
//!
//! Note: Copied from v1 for parallel implementation.

use super::conventions::InstrumentConventions;
use finstack_core::dates::{Date, DayCount};
use finstack_core::types::{Currency, IndexId};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Interest rate instrument quotes for yield curve calibration.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatesQuote {
    /// Deposit rate quote.
    Deposit {
        /// Maturity date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Quoted rate (decimal)
        rate: f64,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// Forward Rate Agreement quote.
    FRA {
        /// Start date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        start: Date,
        /// End date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        end: Date,
        /// Quoted rate (decimal)
        rate: f64,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// Interest Rate Future quote
    Future {
        /// Expiry date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        expiry: Date,
        /// Underlying rate period start date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        period_start: Date,
        /// Underlying rate period end date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        period_end: Date,
        /// Optional fixing date override (defaults to period_start if None)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
        fixing_date: Option<Date>,
        /// Contract price
        price: f64,
        /// Contract specifications
        specs: FutureSpecs,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// Interest Rate Swap quote.
    Swap {
        /// Swap maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Par rate (decimal)
        rate: f64,
        /// Whether this is an OIS (Overnight Index Swap)
        #[serde(default)]
        is_ois: bool,
        /// Instrument-wide conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
        /// Fixed leg conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        fixed_leg_conventions: InstrumentConventions,
        /// Float leg conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        float_leg_conventions: InstrumentConventions,
    },
    /// Basis Swap quote for multi-curve construction.
    BasisSwap {
        /// Swap maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Basis spread in basis points
        spread_bp: f64,
        /// Instrument-wide conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
        /// Primary leg conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        primary_leg_conventions: InstrumentConventions,
        /// Reference leg conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        reference_leg_conventions: InstrumentConventions,
    },
}

impl RatesQuote {
    /// Check if this quote requires a forward curve for pricing
    pub fn requires_forward_curve(&self) -> bool {
        match self {
            RatesQuote::Deposit { .. } => false,
            RatesQuote::FRA { .. } => true,
            RatesQuote::Future { .. } => true,
            RatesQuote::Swap { .. } => true,
            RatesQuote::BasisSwap { .. } => true,
        }
    }

    /// Check if this quote is suitable for OIS discount curve calibration.
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
            RatesQuote::Future { period_end, .. } => *period_end,
            RatesQuote::Swap { maturity, .. } => *maturity,
            RatesQuote::BasisSwap { maturity, .. } => *maturity,
        }
    }

    /// Get per-instrument conventions for this quote.
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
    #[inline]
    pub fn settlement_days(&self) -> Option<i32> {
        self.conventions().settlement_days
    }

    /// Get the effective payment delay for this quote.
    #[inline]
    pub fn payment_delay_days(&self) -> Option<i32> {
        self.conventions().payment_delay_days
    }

    /// Get the effective reset lag for this quote.
    #[inline]
    pub fn reset_lag(&self) -> Option<i32> {
        self.conventions().reset_lag
    }

    /// Get the calendar ID for this quote.
    #[inline]
    pub fn calendar_id(&self) -> Option<&str> {
        self.conventions().calendar_id.as_deref()
    }

    /// Get fixed leg conventions for Swap quotes.
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

    /// Get the float leg index for Swap quotes.
    #[inline]
    pub fn float_index(&self) -> Option<&IndexId> {
        self.float_leg_conventions()
            .and_then(|c| c.index.as_ref())
    }

    /// Get the primary leg index for BasisSwap quotes.
    #[inline]
    pub fn primary_index(&self) -> Option<&IndexId> {
        self.primary_leg_conventions()
            .and_then(|c| c.index.as_ref())
    }

    /// Get the reference leg index for BasisSwap quotes.
    #[inline]
    pub fn reference_index(&self) -> Option<&IndexId> {
        self.reference_leg_conventions()
            .and_then(|c| c.index.as_ref())
    }

    /// Get the currency for BasisSwap quotes.
    #[inline]
    pub fn basis_swap_currency(&self) -> Option<Currency> {
        self.conventions().currency
    }

    // =========================================================================
    // Mutation and Formatting
    // =========================================================================

    /// Create a new quote with the rate bumped by the given amount.
    pub fn bump(&self, amount: f64) -> Self {
        match self {
            RatesQuote::Deposit {
                maturity,
                rate,
                conventions,
            } => RatesQuote::Deposit {
                maturity: *maturity,
                rate: rate + amount,
                conventions: conventions.clone(),
            },
            RatesQuote::FRA {
                start,
                end,
                rate,
                conventions,
            } => RatesQuote::FRA {
                start: *start,
                end: *end,
                rate: rate + amount,
                conventions: conventions.clone(),
            },
            RatesQuote::Future {
                expiry,
                period_start,
                period_end,
                fixing_date,
                price,
                specs,
                conventions,
            } => RatesQuote::Future {
                expiry: *expiry,
                period_start: *period_start,
                period_end: *period_end,
                fixing_date: *fixing_date,
                price: price - (amount * 100.0),
                specs: specs.clone(),
                conventions: conventions.clone(),
            },
            RatesQuote::Swap {
                maturity,
                rate,
                is_ois,
                conventions,
                fixed_leg_conventions,
                float_leg_conventions,
            } => RatesQuote::Swap {
                maturity: *maturity,
                rate: rate + amount,
                is_ois: *is_ois,
                conventions: conventions.clone(),
                fixed_leg_conventions: fixed_leg_conventions.clone(),
                float_leg_conventions: float_leg_conventions.clone(),
            },
            RatesQuote::BasisSwap {
                maturity,
                spread_bp,
                conventions,
                primary_leg_conventions,
                reference_leg_conventions,
            } => RatesQuote::BasisSwap {
                maturity: *maturity,
                spread_bp: spread_bp + (amount * 10_000.0),
                conventions: conventions.clone(),
                primary_leg_conventions: primary_leg_conventions.clone(),
                reference_leg_conventions: reference_leg_conventions.clone(),
            },
        }
    }

    /// Format a descriptive residual key for calibration reports.
    pub fn format_residual_key(&self, counter: usize, currency: Currency) -> String {
        match self {
            RatesQuote::Deposit { maturity, conventions, .. } => {
                let dc = conventions.effective_day_count_or_default(currency);
                format!("DEP-{}-{:?}-{:06}", maturity, dc, counter)
            }
            RatesQuote::FRA { start, end, conventions, .. } => {
                let dc = conventions.effective_day_count_or_default(currency);
                format!("FRA-{}-{}-{:?}-{:06}", start, end, dc, counter)
            }
            RatesQuote::Future {
                expiry,
                period_start,
                period_end,
                specs,
                ..
            } => {
                format!(
                    "FUT-{}-{}-{}-{:?}-{:06}",
                    expiry, period_start, period_end, specs.day_count, counter
                )
            }
            RatesQuote::Swap {
                maturity,
                float_leg_conventions,
                fixed_leg_conventions,
                ..
            } => {
                let index = float_leg_conventions.index.as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("UNKNOWN");
                let fixed_freq = fixed_leg_conventions.payment_frequency
                    .unwrap_or_else(|| InstrumentConventions::default_fixed_leg_frequency(currency));
                let float_freq = float_leg_conventions.payment_frequency
                    .unwrap_or_else(|| InstrumentConventions::default_float_leg_frequency(currency));
                format!(
                    "SWAP-{}-{}-fix{:?}-flt{:?}-{:06}",
                    index,
                    maturity,
                    fixed_freq,
                    float_freq,
                    counter
                )
            }
            RatesQuote::BasisSwap {
                maturity,
                primary_leg_conventions,
                reference_leg_conventions,
                ..
            } => {
                let primary_idx = primary_leg_conventions.index.as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("PRIMARY");
                let ref_idx = reference_leg_conventions.index.as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("REFERENCE");
                format!(
                    "BASIS-{}-{}vs{}-{:06}",
                    maturity, primary_idx, ref_idx, counter
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
    /// Optional market-implied volatility for convexity calculation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_implied_vol: Option<f64>,
    /// Tick size (minimum price increment)
    #[serde(default = "default_tick_size")]
    pub tick_size: f64,
    /// Tick value (dollar value per tick)
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
            multiplier: 1.0,
            face_value: 1_000_000.0,
            delivery_months: 3,
            day_count: DayCount::Act360,
            convexity_adjustment: None,
            market_implied_vol: None,
            tick_size: default_tick_size(),
            tick_value: default_tick_value(),
        }
    }
}


