//! Market quote data structures and types.

use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::prelude::*;
use finstack_core::types::{IndexId, UnderlyingId};

/// Interest rate instrument quotes for yield curve calibration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RatesQuote {
    /// Deposit rate quote
    Deposit {
        /// Maturity date
        maturity: Date,
        /// Quoted rate (decimal)
        rate: f64,
        /// Day count convention
        day_count: DayCount,
    },
    /// Forward Rate Agreement quote
    FRA {
        /// Start date
        start: Date,
        /// End date  
        end: Date,
        /// Quoted rate (decimal)
        rate: f64,
        /// Day count convention
        day_count: DayCount,
    },
    /// Interest Rate Future quote
    Future {
        /// Expiry date
        expiry: Date,
        /// Contract price (e.g., 99.25 for 0.75% implied rate)
        price: f64,
        /// Contract specifications
        specs: FutureSpecs,
    },
    /// Interest Rate Swap quote
    Swap {
        /// Swap maturity
        maturity: Date,
        /// Par rate (decimal)
        rate: f64,
        /// Fixed leg frequency
        fixed_freq: Frequency,
        /// Float leg frequency  
        float_freq: Frequency,
        /// Fixed leg day count
        fixed_dc: DayCount,
        /// Float leg day count
        float_dc: DayCount,
        /// Float leg index (e.g., "3M-LIBOR")
        index: IndexId,
    },
    /// Basis Swap quote for multi-curve construction
    BasisSwap {
        /// Swap maturity
        maturity: Date,
        /// Primary leg index (e.g., "3M-LIBOR", "3M-SOFR")
        primary_index: String,
        /// Reference leg index (e.g., "6M-LIBOR", "1M-SOFR")
        reference_index: String,
        /// Basis spread in basis points (primary pays reference + spread)
        spread_bp: f64,
        /// Primary leg frequency
        primary_freq: Frequency,
        /// Reference leg frequency  
        reference_freq: Frequency,
        /// Primary leg day count
        primary_dc: DayCount,
        /// Reference leg day count
        reference_dc: DayCount,
        /// Currency for both legs
        currency: Currency,
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
                finstack_core::dates::add_months(*expiry, specs.delivery_months as i32)
            }
            RatesQuote::Swap { maturity, .. } => *maturity,
            RatesQuote::BasisSwap { maturity, .. } => *maturity,
        }
    }

    /// Create a new quote with the rate bumped by the given amount.
    ///
    /// Used for Jacobian calculation (sensitivity analysis).
    pub fn bump(&self, amount: f64) -> Self {
        match self {
            RatesQuote::Deposit {
                maturity,
                rate,
                day_count,
            } => RatesQuote::Deposit {
                maturity: *maturity,
                rate: rate + amount,
                day_count: *day_count,
            },
            RatesQuote::FRA {
                start,
                end,
                rate,
                day_count,
            } => RatesQuote::FRA {
                start: *start,
                end: *end,
                rate: rate + amount,
                day_count: *day_count,
            },
            RatesQuote::Future {
                expiry,
                price,
                specs,
            } => RatesQuote::Future {
                expiry: *expiry,
                // For futures, price = 100 - rate * 100.
                // Bump rate by +amount => price decreases by amount * 100
                price: price - (amount * 100.0),
                specs: specs.clone(),
            },
            RatesQuote::Swap {
                maturity,
                rate,
                fixed_freq,
                float_freq,
                fixed_dc,
                float_dc,
                index,
            } => RatesQuote::Swap {
                maturity: *maturity,
                rate: rate + amount,
                fixed_freq: *fixed_freq,
                float_freq: *float_freq,
                fixed_dc: *fixed_dc,
                float_dc: *float_dc,
                index: index.clone(),
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
            },
        }
    }
}

/// Credit instrument quotes for hazard curve and correlation calibration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CreditQuote {
    /// CDS par spread quote
    CDS {
        /// Reference entity
        entity: String,
        /// CDS maturity
        maturity: Date,
        /// Par spread in basis points
        spread_bp: f64,
        /// Recovery rate assumption
        recovery_rate: f64,
        /// Currency
        currency: Currency,
    },
    /// CDS upfront quote (for distressed credits or non-standard contracts)
    CDSUpfront {
        /// Reference entity
        entity: String,
        /// CDS maturity
        maturity: Date,
        /// Upfront payment (% of notional, positive = protection buyer pays)
        upfront_pct: f64,
        /// Running spread in basis points
        running_spread_bp: f64,
        /// Recovery rate assumption
        recovery_rate: f64,
        /// Currency
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
        maturity: Date,
        /// Upfront payment (% of notional)
        upfront_pct: f64,
        /// Running spread (bps)
        running_spread_bp: f64,
    },
}

/// Volatility quotes for surface calibration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum VolQuote {
    /// Option implied volatility quote
    OptionVol {
        /// Underlying identifier
        underlying: UnderlyingId,
        /// Option expiry
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
        expiry: Date,
        /// Underlying swap tenor
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
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InflationQuote {
    /// Zero-coupon inflation swap quote
    InflationSwap {
        /// Swap maturity
        maturity: Date,
        /// Fixed rate (decimal)
        rate: f64,
        /// Inflation index identifier  
        index: String,
    },
    /// Year-on-year inflation swap
    YoYInflationSwap {
        /// Swap maturity
        maturity: Date,
        /// Fixed rate (decimal)
        rate: f64,
        /// Inflation index identifier  
        index: String,
        /// Payment frequency
        frequency: Frequency,
    },
}

/// Unified market quote that can be any instrument type.
/// Used when multiple quote types need to be handled together.
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
    pub day_count: DayCount,
    /// Convexity adjustment (for long-dated futures)
    pub convexity_adjustment: Option<f64>,
}

impl Default for FutureSpecs {
    fn default() -> Self {
        Self {
            multiplier: 1_000_000.0, // $1MM face value
            face_value: 1_000_000.0,
            delivery_months: 3,
            day_count: DayCount::Act360,
            convexity_adjustment: None,
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
        Currency::GBP => 0,  // GBP settles same-day
        Currency::AUD | Currency::CAD => 1,  // T+1 for AUD/CAD
        _ => 2,  // T+2 for USD, EUR, JPY, CHF, and others
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
