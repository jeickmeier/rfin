//! Market quote data structures and types.

use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::prelude::*;
use finstack_core::types::{IndexId, UnderlyingId};

/// Interest rate instrument quotes for yield curve calibration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

    /// Check if this quote is suitable for OIS discount curve calibration
    pub fn is_ois_suitable(&self) -> bool {
        match self {
            RatesQuote::Deposit { .. } => true,
            // OIS swaps would have index like "SOFR", "EONIA", etc.
            RatesQuote::Swap { index, .. } => {
                // Tokenize and match exact tokens to avoid false positives like "13MOIS"
                let up = index.as_ref().to_uppercase();
                let ascii = up.replace('€', "E");
                let tokens: Vec<String> = ascii
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .map(|s| s.to_string())
                    .collect();
                let ois_tokens = [
                    "OIS", "SOFR", "SONIA", "EONIA", "ESTR", "€STR", "TONAR", "TONA",
                ];
                tokens.iter().any(|t| ois_tokens.contains(&t.as_str()))
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
}

/// Credit instrument quotes for hazard curve and correlation calibration.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
