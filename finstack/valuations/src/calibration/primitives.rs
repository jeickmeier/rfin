//! Core primitives for calibration framework.

use finstack_core::dates::Date;
use finstack_core::prelude::*;
use finstack_core::F;
use ordered_float::OrderedFloat;

/// Type alias for hashable floating point values used as HashMap keys.
///
/// Uses OrderedFloat which provides total ordering and hashing for f64 values.
/// This simplifies the code compared to a custom HashableFloat implementation.
pub type HashableFloat = OrderedFloat<F>;

/// Market instrument quote for calibration.
#[derive(Clone, Debug)]
pub enum InstrumentQuote {
    /// Deposit rate quote
    Deposit {
        /// Maturity date
        maturity: Date,
        /// Quoted rate (decimal)
        rate: F,
        /// Day count convention
        day_count: finstack_core::dates::DayCount,
    },
    /// Forward Rate Agreement quote
    FRA {
        /// Start date
        start: Date,
        /// End date  
        end: Date,
        /// Quoted rate (decimal)
        rate: F,
        /// Day count convention
        day_count: finstack_core::dates::DayCount,
    },
    /// Interest Rate Future quote
    Future {
        /// Expiry date
        expiry: Date,
        /// Contract price (e.g., 99.25 for 0.75% implied rate)
        price: F,
        /// Contract specifications
        specs: FutureSpecs,
    },
    /// Interest Rate Swap quote
    Swap {
        /// Swap maturity
        maturity: Date,
        /// Par rate (decimal)
        rate: F,
        /// Fixed leg frequency
        fixed_freq: finstack_core::dates::Frequency,
        /// Float leg frequency  
        float_freq: finstack_core::dates::Frequency,
        /// Fixed leg day count
        fixed_dc: finstack_core::dates::DayCount,
        /// Float leg day count
        float_dc: finstack_core::dates::DayCount,
        /// Float leg index (e.g., "3M-LIBOR")
        index: String,
    },
    /// CDS par spread quote
    CDS {
        /// Reference entity
        entity: String,
        /// CDS maturity
        maturity: Date,
        /// Par spread in basis points
        spread_bp: F,
        /// Recovery rate assumption
        recovery_rate: F,
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
        upfront_pct: F,
        /// Running spread in basis points
        running_spread_bp: F,
        /// Recovery rate assumption
        recovery_rate: F,
        /// Currency
        currency: Currency,
    },
    /// Option implied volatility quote
    OptionVol {
        /// Underlying identifier
        underlying: String,
        /// Option expiry
        expiry: Date,
        /// Strike (rate for swaptions, price for equity/FX)
        strike: F,
        /// Implied volatility
        vol: F,
        /// Option type ("Call", "Put", "Straddle")
        option_type: String,
    },
    /// Zero-coupon inflation swap quote
    InflationSwap {
        /// Swap maturity
        maturity: Date,
        /// Fixed rate (decimal)
        rate: F,
        /// Inflation index identifier  
        index: String,
    },
    /// CDS Tranche quote
    CDSTranche {
        /// Index name (e.g., "CDX.NA.IG.42")
        index: String,
        /// Attachment point (%)
        attachment: F,
        /// Detachment point (%)
        detachment: F,
        /// Maturity date
        maturity: Date,
        /// Upfront payment (% of notional)
        upfront_pct: F,
        /// Running spread (bps)
        running_spread_bp: F,
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
        spread_bp: F,
        /// Primary leg frequency
        primary_freq: finstack_core::dates::Frequency,
        /// Reference leg frequency  
        reference_freq: finstack_core::dates::Frequency,
        /// Primary leg day count
        primary_dc: finstack_core::dates::DayCount,
        /// Reference leg day count
        reference_dc: finstack_core::dates::DayCount,
        /// Currency for both legs
        currency: Currency,
    },
}

/// Specifications for interest rate futures contracts.
#[derive(Clone, Debug)]
pub struct FutureSpecs {
    /// Contract multiplier
    pub multiplier: F,
    /// Face value
    pub face_value: F,
    /// Number of delivery months
    pub delivery_months: u8,
    /// Day count convention
    pub day_count: finstack_core::dates::DayCount,
    /// Convexity adjustment (for long-dated futures)
    pub convexity_adjustment: Option<F>,
}

impl Default for FutureSpecs {
    fn default() -> Self {
        Self {
            multiplier: 1_000_000.0, // $1MM face value
            face_value: 1_000_000.0,
            delivery_months: 3,
            day_count: finstack_core::dates::DayCount::Act360,
            convexity_adjustment: None,
        }
    }
}

/// Calibration constraint for optimization.
#[derive(Clone, Debug)]
pub struct CalibrationConstraint {
    /// Instrument identifier
    pub instrument_id: String,
    /// Target value (rate, price, spread, etc.)
    pub target_value: F,
    /// Weight in objective function
    pub weight: F,
    /// Constraint type
    pub constraint_type: ConstraintType,
}

/// Type of calibration constraint.
#[derive(Clone, Debug)]
pub enum ConstraintType {
    /// Exact match (zero PV for par instruments)
    Exact,
    /// Weighted least squares fit
    WeightedFit,
    /// Inequality constraint (e.g., no-arbitrage)
    Inequality {
        bound: F,
        direction: InequalityDirection,
    },
}

/// Direction for inequality constraints.
#[derive(Clone, Debug)]
pub enum InequalityDirection {
    /// Value >= bound
    GreaterEqual,
    /// Value <= bound  
    LessEqual,
}

impl CalibrationConstraint {
    /// Create an exact constraint.
    pub fn exact(instrument_id: impl Into<String>, target_value: F) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            target_value,
            weight: 1.0,
            constraint_type: ConstraintType::Exact,
        }
    }

    /// Create a weighted least squares constraint.
    pub fn weighted(instrument_id: impl Into<String>, target_value: F, weight: F) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            target_value,
            weight,
            constraint_type: ConstraintType::WeightedFit,
        }
    }

    /// Set constraint weight.
    pub fn with_weight(mut self, weight: F) -> Self {
        self.weight = weight;
        self
    }
}
