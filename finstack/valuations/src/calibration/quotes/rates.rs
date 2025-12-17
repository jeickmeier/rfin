//! Interest rate quote types for yield curve calibration.
//!
//! Quote conventions are now consolidated into `InstrumentConventions` structs.
//! Use `InstrumentConventions` defaults and the centralized conventions resolver.

use super::conventions::InstrumentConventions;
use finstack_core::dates::{Date, DayCount};
use finstack_core::prelude::*;
use finstack_core::types::IndexId;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Interest rate instrument quotes for yield curve calibration.
///
/// All convention-related fields (day count, payment frequency, index) are now
/// consolidated into `InstrumentConventions` structs.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatesQuote {
    /// Deposit rate quote.
    ///
    /// Day count convention is specified via `conventions.day_count`.
    /// If not provided, defaults to ACT/360 for USD/EUR/CHF or ACT/365F for GBP/JPY/AUD.
    Deposit {
        /// Maturity date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Quoted rate (decimal)
        rate: f64,
        /// Per-instrument conventions (day_count, settlement, calendar, etc.)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// Forward Rate Agreement quote.
    ///
    /// Day count convention is specified via `conventions.day_count`.
    /// If not provided, defaults to ACT/360 for USD/EUR/CHF or ACT/365F for GBP/JPY/AUD.
    FRA {
        /// Start date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        start: Date,
        /// End date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        end: Date,
        /// Quoted rate (decimal)
        rate: f64,
        /// Per-instrument conventions (day_count, settlement, reset lag, etc.)
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
        /// Contract specifications (includes day_count for the contract)
        specs: FutureSpecs,
        /// Per-instrument conventions (reset lag, etc.)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// Interest Rate Swap quote.
    ///
    /// Leg conventions are specified via `fixed_leg_conventions` and `float_leg_conventions`:
    /// - `payment_frequency`: Payment/coupon frequency (defaults to semi-annual fixed, quarterly float)
    /// - `day_count`: Day count convention (defaults to 30/360 fixed, ACT/360 float for USD)
    /// - `index`: Float leg index (e.g., "USD-SOFR-3M") - required for float leg
    Swap {
        /// Swap maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Par rate (decimal)
        rate: f64,
        /// Whether this is an OIS (Overnight Index Swap) suitable for discount curve calibration.
        /// Set to true for overnight indices (SOFR, SONIA, €STR, TONA, etc.).
        #[serde(default)]
        is_ois: bool,
        /// Instrument-wide conventions (settlement days, calendar, currency, etc.)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
        /// Fixed leg conventions (payment_frequency, day_count, business_day_convention)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        fixed_leg_conventions: InstrumentConventions,
        /// Float leg conventions (payment_frequency, day_count, index, reset_lag)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        float_leg_conventions: InstrumentConventions,
    },
    /// Basis Swap quote for multi-curve construction.
    ///
    /// Leg conventions are specified via `primary_leg_conventions` and `reference_leg_conventions`:
    /// - `payment_frequency`: Payment frequency for each leg
    /// - `day_count`: Day count convention for each leg
    /// - `index`: Index identifier for each leg (e.g., "USD-SOFR-3M", "USD-SOFR-6M")
    ///
    /// Currency is specified via `conventions.currency`.
    BasisSwap {
        /// Swap maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Basis spread in basis points (primary pays reference + spread)
        spread_bp: f64,
        /// Instrument-wide conventions (settlement days, calendar, currency)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
        /// Primary leg conventions (payment_frequency, day_count, index)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        primary_leg_conventions: InstrumentConventions,
        /// Reference leg conventions (payment_frequency, day_count, index)
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

    /// Get the float leg index for Swap quotes.
    ///
    /// Returns the index from `float_leg_conventions.index`.
    /// Panics if called on non-Swap quotes or if index is not specified.
    #[inline]
    pub fn float_index(&self) -> Option<&IndexId> {
        self.float_leg_conventions().and_then(|c| c.index.as_ref())
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
    ///
    /// Returns the currency from `conventions.currency`.
    #[inline]
    pub fn basis_swap_currency(&self) -> Option<Currency> {
        self.conventions().currency
    }

    // =========================================================================
    // Mutation and Formatting
    // =========================================================================

    /// Create a new quote with the rate bumped by the given amount.
    ///
    /// Used for Jacobian calculation (sensitivity analysis).
    /// Preserves per-instrument conventions.
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
                spread_bp: spread_bp + (amount * 10_000.0), // Convert decimal bump to bp
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
    /// * `currency` - Currency for resolving default conventions
    ///
    /// # Returns
    ///
    /// A formatted string key like "DEP-2025-03-15-Act360-000001" or
    /// "SWAP-USD-SOFR-2027-01-15-fix6M-flt3M-000002"
    pub fn format_residual_key(&self, counter: usize, currency: Currency) -> String {
        match self {
            RatesQuote::Deposit {
                maturity,
                conventions,
                ..
            } => {
                let dc = conventions.effective_day_count_or_default(currency);
                format!("DEP-{}-{:?}-{:06}", maturity, dc, counter)
            }
            RatesQuote::FRA {
                start,
                end,
                conventions,
                ..
            } => {
                let dc = conventions.effective_day_count_or_default(currency);
                format!("FRA-{}-{}-{:?}-{:06}", start, end, dc, counter)
            }
            RatesQuote::Future { expiry, specs, .. } => {
                format!(
                    "FUT-{}-{}m-{:?}-{:06}",
                    expiry, specs.delivery_months, specs.day_count, counter
                )
            }
            RatesQuote::Swap {
                maturity,
                float_leg_conventions,
                fixed_leg_conventions,
                ..
            } => {
                let index = float_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("UNKNOWN");
                let fixed_freq = fixed_leg_conventions.payment_frequency.unwrap_or_else(|| {
                    InstrumentConventions::default_fixed_leg_frequency(currency)
                });
                let float_freq = float_leg_conventions.payment_frequency.unwrap_or_else(|| {
                    InstrumentConventions::default_float_leg_frequency(currency)
                });
                format!(
                    "SWAP-{}-{}-fix{:?}-flt{:?}-{:06}",
                    index, maturity, fixed_freq, float_freq, counter
                )
            }
            RatesQuote::BasisSwap {
                maturity,
                primary_leg_conventions,
                reference_leg_conventions,
                ..
            } => {
                let primary_idx = primary_leg_conventions
                    .index
                    .as_ref()
                    .map(|i| i.as_str())
                    .unwrap_or("PRIMARY");
                let ref_idx = reference_leg_conventions
                    .index
                    .as_ref()
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
