//! Market quote data structures and types.

use crate::instruments::irs::FloatingLegCompounding;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::prelude::*;
use finstack_core::types::{IndexId, UnderlyingId};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Per-instrument conventions for calibration.
///
/// These optional fields allow each quote to specify its own settlement,
/// payment, and fixing conventions. When not specified, the calibrator
/// uses currency-specific defaults.
///
/// # Example
///
/// ```ignore
/// let conventions = InstrumentConventions {
///     settlement_days: Some(0),  // T+0 for this instrument
///     calendar_id: Some("gblo".to_string()),
///     ..Default::default()
/// };
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(default)]
pub struct InstrumentConventions {
    /// Settlement lag in business days from trade date (e.g., 0 for T+0, 2 for T+2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_days: Option<i32>,
    /// Payment delay in business days after period end
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_delay_days: Option<i32>,
    /// Reset lag in business days for floating rate fixings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_lag: Option<i32>,
    /// Calendar identifier for schedule generation and business day adjustments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,
}

impl InstrumentConventions {
    /// Create conventions with settlement days.
    pub fn with_settlement_days(mut self, days: i32) -> Self {
        self.settlement_days = Some(days);
        self
    }

    /// Create conventions with payment delay.
    pub fn with_payment_delay(mut self, days: i32) -> Self {
        self.payment_delay_days = Some(days);
        self
    }

    /// Create conventions with reset lag.
    pub fn with_reset_lag(mut self, days: i32) -> Self {
        self.reset_lag = Some(days);
        self
    }

    /// Create conventions with calendar ID.
    pub fn with_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Check if all fields are None (i.e., use defaults).
    pub fn is_empty(&self) -> bool {
        self.settlement_days.is_none()
            && self.payment_delay_days.is_none()
            && self.reset_lag.is_none()
            && self.calendar_id.is_none()
    }
}

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
        fixed_freq: Frequency,
        /// Float leg frequency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        float_freq: Frequency,
        /// Fixed leg day count
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        fixed_dc: DayCount,
        /// Float leg day count
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        float_dc: DayCount,
        /// Float leg index (e.g., "3M-LIBOR")
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        index: IndexId,
        /// Per-instrument conventions (settlement, payment delay, reset lag, calendar)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
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
        primary_freq: Frequency,
        /// Reference leg frequency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        reference_freq: Frequency,
        /// Primary leg day count
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        primary_dc: DayCount,
        /// Reference leg day count
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        reference_dc: DayCount,
        /// Currency for both legs
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Per-instrument conventions (settlement, reset lag, calendar)
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
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
    /// Uses the OIS index registry for accurate classification. This method
    /// returns `true` for:
    /// - Deposits (always suitable for short-end discount curve)
    /// - Swaps referencing overnight rate indices (SOFR, SONIA, €STR, etc.)
    ///
    /// Returns `false` for:
    /// - FRAs (require forward curves)
    /// - Futures (require forward curves)
    /// - Swaps referencing term rates (LIBOR, EURIBOR, Term SOFR)
    /// - Basis swaps (require separate forward curves)
    pub fn is_ois_suitable(&self) -> bool {
        match self {
            RatesQuote::Deposit { .. } => true,
            RatesQuote::Swap { index, .. } => {
                // Use the registry for accurate overnight rate classification
                is_overnight_index(index.as_ref())
            }
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
                conventions,
            } => RatesQuote::Swap {
                maturity: *maturity,
                rate: rate + amount,
                fixed_freq: *fixed_freq,
                float_freq: *float_freq,
                fixed_dc: *fixed_dc,
                float_dc: *float_dc,
                index: index.clone(),
                conventions: conventions.clone(),
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

/// Credit instrument quotes for hazard curve and correlation calibration.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CreditQuote {
    /// CDS par spread quote
    CDS {
        /// Reference entity
        entity: String,
        /// CDS maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Par spread in basis points
        spread_bp: f64,
        /// Recovery rate assumption
        recovery_rate: f64,
        /// Currency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
    },
    /// CDS upfront quote (for distressed credits or non-standard contracts)
    CDSUpfront {
        /// Reference entity
        entity: String,
        /// CDS maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Upfront payment (% of notional, positive = protection buyer pays)
        upfront_pct: f64,
        /// Running spread in basis points
        running_spread_bp: f64,
        /// Recovery rate assumption
        recovery_rate: f64,
        /// Currency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
    },
    /// CDS Tranche quote
    CDSTranche {
        /// Index name (e.g., "CDX.NA.IG.42")
        index: String,
        /// Attachment point (%)
        attachment: f64,
        /// Detachment point (%)
        detachment: f64,
        /// Maturity date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Upfront payment (% of notional)
        upfront_pct: f64,
        /// Running spread (bps)
        running_spread_bp: f64,
    },
}

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

/// Inflation instrument quotes.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
        frequency: Frequency,
    },
}

/// Unified market quote that can be any instrument type.
/// Used when multiple quote types need to be handled together.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketQuote {
    /// Interest rate quotes
    Rates(RatesQuote),
    /// Credit quotes
    Credit(CreditQuote),
    /// Volatility quotes
    Vol(VolQuote),
    /// Inflation quotes
    Inflation(InflationQuote),
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

/// Standard OIS index tokens used for identifying OIS instruments.
#[allow(dead_code)]
pub const STANDARD_OIS_INDICES: &[&str] = &[
    "OIS", "SOFR", "SONIA", "EONIA", "ESTR", "€STR", "TONAR", "TONA", "CORRA", "AONIA", "SARON",
    "SORA",
];

// ============================================================================
// OIS Index Registry - Market-Standard Overnight Rate Classification
// ============================================================================

/// Rate index family classification for multi-curve framework.
///
/// Distinguishes overnight rates (used for discounting) from term rates
/// (used for floating leg projection in non-OIS swaps).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RateIndexFamily {
    /// Overnight rates: SOFR, SONIA, €STR, TONA, etc.
    /// Used for OIS curves and collateralized discounting.
    Overnight,
    /// Term rates: 3M LIBOR, 6M EURIBOR, Term SOFR, etc.
    /// Used for floating leg projection, require separate forward curves.
    Term,
}

/// Detailed information about a rate index.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RateIndexInfo {
    /// Rate index family (overnight vs term)
    pub family: RateIndexFamily,
    /// Currency associated with this index
    pub currency: Currency,
    /// Standard day count convention
    pub day_count: DayCount,
    /// Settlement lag in business days (T+0, T+1, T+2)
    pub settlement_days: i32,
}

impl RateIndexInfo {
    /// Create an overnight rate index info
    #[allow(dead_code)]
    pub const fn overnight(currency: Currency, day_count: DayCount, settlement_days: i32) -> Self {
        Self {
            family: RateIndexFamily::Overnight,
            currency,
            day_count,
            settlement_days,
        }
    }

    /// Create a term rate index info
    #[allow(dead_code)]
    pub const fn term(currency: Currency, day_count: DayCount, settlement_days: i32) -> Self {
        Self {
            family: RateIndexFamily::Term,
            currency,
            day_count,
            settlement_days,
        }
    }
}

/// Registry entry for a known rate index.
struct IndexEntry {
    /// Canonical tokens that identify this index (uppercase, without special chars)
    tokens: &'static [&'static str],
    /// Index metadata
    info: RateIndexInfo,
}

/// Static registry of known rate indices with market-standard conventions.
///
/// This registry provides accurate classification of rate indices for:
/// - OIS vs term rate distinction (multi-curve framework)
/// - Currency-specific settlement conventions
/// - Day count conventions per index
static INDEX_REGISTRY: &[IndexEntry] = &[
    // USD Overnight Rates
    IndexEntry {
        tokens: &["SOFR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::USD,
            day_count: DayCount::Act360,
            settlement_days: 2,
        },
    },
    IndexEntry {
        tokens: &["EFFR", "FEDFUNDS", "FF"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::USD,
            day_count: DayCount::Act360,
            settlement_days: 1,
        },
    },
    // EUR Overnight Rates
    IndexEntry {
        tokens: &["ESTR", "ESTER", "STR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::EUR,
            day_count: DayCount::Act360,
            settlement_days: 2,
        },
    },
    IndexEntry {
        tokens: &["EONIA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::EUR,
            day_count: DayCount::Act360,
            settlement_days: 2,
        },
    },
    // GBP Overnight Rates (T+0 settlement!)
    IndexEntry {
        tokens: &["SONIA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::GBP,
            day_count: DayCount::Act365F,
            settlement_days: 0, // GBP settles T+0
        },
    },
    // JPY Overnight Rates
    IndexEntry {
        tokens: &["TONA", "TONAR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::JPY,
            day_count: DayCount::Act365F,
            settlement_days: 2,
        },
    },
    // CHF Overnight Rates
    IndexEntry {
        tokens: &["SARON"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::CHF,
            day_count: DayCount::Act360,
            settlement_days: 2,
        },
    },
    // AUD Overnight Rates
    IndexEntry {
        tokens: &["AONIA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::AUD,
            day_count: DayCount::Act365F,
            settlement_days: 1,
        },
    },
    // CAD Overnight Rates
    IndexEntry {
        tokens: &["CORRA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::CAD,
            day_count: DayCount::Act365F,
            settlement_days: 1,
        },
    },
    // SGD Overnight Rates
    IndexEntry {
        tokens: &["SORA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::SGD,
            day_count: DayCount::Act365F,
            settlement_days: 2,
        },
    },
    // Generic OIS marker (matches "USD-OIS", "EUR-OIS", etc.)
    IndexEntry {
        tokens: &["OIS"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
            currency: Currency::USD, // Default, currency should be inferred from context
            day_count: DayCount::Act360,
            settlement_days: 2,
        },
    },
    // Term Rates (explicitly NOT overnight)
    IndexEntry {
        tokens: &["LIBOR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Term,
            currency: Currency::USD,
            day_count: DayCount::Act360,
            settlement_days: 2,
        },
    },
    IndexEntry {
        tokens: &["EURIBOR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Term,
            currency: Currency::EUR,
            day_count: DayCount::Act360,
            settlement_days: 2,
        },
    },
    IndexEntry {
        tokens: &["TIBOR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Term,
            currency: Currency::JPY,
            day_count: DayCount::Act365F,
            settlement_days: 2,
        },
    },
];

/// Lookup rate index information from an index identifier string.
///
/// Parses the index string and matches against the registry. Returns `None`
/// if the index is not recognized (caller should treat as term rate).
///
/// # Examples
///
/// ```ignore
/// use finstack_valuations::calibration::quote::lookup_index_info;
///
/// let info = lookup_index_info("USD-SOFR-OIS");
/// assert!(info.is_some());
/// assert_eq!(info.unwrap().family, RateIndexFamily::Overnight);
///
/// let libor = lookup_index_info("3M-USD-LIBOR");
/// assert!(libor.is_some());
/// assert_eq!(libor.unwrap().family, RateIndexFamily::Term);
/// ```
pub fn lookup_index_info(index: &str) -> Option<RateIndexInfo> {
    // Normalize: uppercase, replace € with E, split into tokens
    let normalized = index.to_uppercase().replace('€', "E");
    let tokens: Vec<&str> = normalized
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();

    // Check each registry entry
    for entry in INDEX_REGISTRY {
        for &entry_token in entry.tokens {
            // Match if any token in the index string matches this entry
            if tokens.contains(&entry_token) {
                return Some(entry.info.clone());
            }
        }
    }

    None
}

/// Check if an index identifier represents an overnight rate.
///
/// Uses the index registry for accurate classification. Returns `false`
/// for unrecognized indices (conservative default).
pub fn is_overnight_index(index: &str) -> bool {
    lookup_index_info(index)
        .map(|info| info.family == RateIndexFamily::Overnight)
        .unwrap_or(false)
}

/// Get settlement days for a currency (default if index not found).
///
/// Market-standard settlement conventions:
/// - USD: T+2
/// - EUR: T+2
/// - GBP: T+0
/// - JPY: T+2
/// - Others: T+2 (default)
pub fn settlement_days_for_currency(currency: Currency) -> i32 {
    match currency {
        Currency::GBP => 0,                 // GBP settles same-day
        Currency::AUD | Currency::CAD => 1, // T+1 for AUD/CAD
        _ => 2,                             // T+2 for USD, EUR, JPY, CHF, and others
    }
}

/// Get the standard day count convention for a currency's discount curve.
///
/// Market conventions:
/// - USD, EUR, CHF: ACT/360
/// - GBP, JPY, AUD, CAD: ACT/365F
pub fn standard_day_count_for_currency(currency: Currency) -> DayCount {
    match currency {
        Currency::GBP | Currency::JPY | Currency::AUD | Currency::CAD | Currency::NZD => {
            DayCount::Act365F
        }
        _ => DayCount::Act360,
    }
}

/// Get the default settlement calendar ID for a currency.
///
/// Market-standard settlement calendars used for spot/settlement date calculation:
/// - USD: "usny" (US Federal Reserve / New York)
/// - EUR: "target2" (TARGET2 / ECB / SEPA)
/// - GBP: "gblo" (London Stock Exchange / UK Bank Holidays)
/// - JPY: "jpto" (Tokyo Stock Exchange / Japan)
/// - CHF: "chzu" (Zurich / Switzerland)
/// - AUD: "ausy" (Sydney / Australia)
/// - CAD: "cato" (Toronto / Canada)
/// - Others: "usny" (default to US calendar)
///
/// These IDs correspond to calendars in the `CalendarRegistry`.
pub fn default_calendar_for_currency(currency: Currency) -> &'static str {
    match currency {
        Currency::USD => "usny",
        Currency::EUR => "target2",
        Currency::GBP => "gblo",
        Currency::JPY => "jpto",
        Currency::CHF => "chzu",
        Currency::AUD => "ausy",
        Currency::CAD => "cato",
        Currency::NZD => "nzau", // Auckland/Wellington
        Currency::HKD => "hkex",
        Currency::SGD => "sgex",
        _ => "usny", // Default to US calendar for unlisted currencies
    }
}

/// Get the OIS compounding method for a rate index.
///
/// Returns the market-standard OIS compounding method based on the index name
/// and currency. This determines how overnight rates are compounded for OIS swaps.
///
/// # Logic
///
/// 1. **Index-name driven**: Checks for specific index tokens (SONIA, ESTR, TONA, SOFR)
/// 2. **Currency fallback**: For generic indices like "USD-OIS", uses currency conventions
///
/// # Examples
///
/// ```ignore
/// use finstack_valuations::calibration::quote::ois_compounding_for_index;
/// use finstack_core::currency::Currency;
/// use finstack_core::types::IndexId;
///
/// let index: IndexId = "USD-SOFR".into();
/// let compounding = ois_compounding_for_index(&index, Currency::USD);
/// // Returns FloatingLegCompounding::sofr()
/// ```
pub fn ois_compounding_for_index(index: &IndexId, currency: Currency) -> FloatingLegCompounding {
    let upper = index.as_str().to_ascii_uppercase();

    // Index-name driven overrides
    if upper.contains("SONIA") {
        return FloatingLegCompounding::sonia();
    }
    if upper.contains("ESTR") || upper.contains("€STR") {
        return FloatingLegCompounding::estr();
    }
    if upper.contains("TONA") || upper.contains("TONAR") {
        return FloatingLegCompounding::tona();
    }
    if upper.contains("SOFR") {
        return FloatingLegCompounding::sofr();
    }

    // Currency fallback for generic ids like "USD-OIS"
    match currency {
        Currency::GBP => FloatingLegCompounding::sonia(),
        Currency::EUR => FloatingLegCompounding::estr(),
        Currency::JPY => FloatingLegCompounding::tona(),
        _ => FloatingLegCompounding::sofr(),
    }
}
