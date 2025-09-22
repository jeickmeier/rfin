//! Common leg specification types for interest rate and credit instruments.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency};
use finstack_core::types::CurveId;
use finstack_core::F;

use crate::cashflow::builder::ScheduleParams;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Direction for swap legs from the perspective of the fixed rate payer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PayReceive {
    /// Pay fixed rate, receive floating rate
    PayFixed,
    /// Receive fixed rate, pay floating rate  
    ReceiveFixed,
}

/// Method for calculating par rates in swaps
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ParRateMethod {
    /// Use forward-curve based float PV over the schedule (market standard)
    ForwardBased,
    /// Use discount-curve ratio for bootstrapping
    DiscountRatio,
}

/// Specification for fixed rate legs in interest rate swaps
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FixedLegSpec {
    /// Discount curve identifier for pricing
    pub disc_id: &'static str,
    /// Fixed rate (e.g., 0.05 for 5%)
    pub rate: F,
    /// Schedule parameters
    pub schedule: ScheduleParams,
    /// Start date of the fixed leg
    pub start: Date,
    /// End date of the fixed leg
    pub end: Date,
    /// Optional par-rate calculation method override
    pub par_method: Option<ParRateMethod>,
    /// If true, use simple interest on accrual fraction
    pub compounding_simple: bool,
}

/// Specification for floating rate legs in interest rate swaps
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FloatLegSpec {
    /// Discount curve identifier for pricing
    pub disc_id: &'static str,
    /// Forward curve identifier for rate projections
    pub fwd_id: &'static str,
    /// Spread in basis points added to the forward rate
    pub spread_bp: F,
    /// Schedule parameters
    pub schedule: ScheduleParams,
    /// Reset lag in business days for floating rate
    pub reset_lag_days: i32,
    /// Start date of the floating leg
    pub start: Date,
    /// End date of the floating leg
    pub end: Date,
}

/// Specification for basis swap legs (floating vs floating)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasisSwapLeg {
    /// Forward curve identifier for this leg
    pub forward_curve_id: CurveId,
    /// Payment frequency for the leg
    pub frequency: Frequency,
    /// Day count convention for accrual calculations
    pub day_count: DayCount,
    /// Business day convention for date adjustments
    pub bdc: BusinessDayConvention,
    /// Optional spread in decimal form (e.g., 0.0005 for 5 basis points)
    pub spread: F,
}

/// Specification for CDS premium legs
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PremiumLegSpec {
    /// Start date of protection
    pub start: Date,
    /// End date of protection
    pub end: Date,
    /// Schedule parameters
    pub schedule: ScheduleParams,
    /// Fixed spread in basis points
    pub spread_bp: F,
    /// Discount curve identifier
    pub disc_id: &'static str,
}

/// Specification for CDS protection legs
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProtectionLegSpec {
    /// Credit curve identifier for default probabilities
    pub credit_id: &'static str,
    /// Recovery rate (0.0 to 1.0)
    pub recovery_rate: F,
    /// Settlement type on default  
    pub settlement: CdsSettlementType,
    /// Settlement delay in business days
    pub settlement_delay: u16,
}

/// Settlement type for CDS protection payment
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CdsSettlementType {
    /// Physical delivery of defaulted bonds
    Physical,
    /// Cash settlement based on recovery rate
    Cash,
    /// Auction-based settlement
    Auction,
}

/// Specification for TRS financing legs
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FinancingLegSpec {
    /// Discount curve identifier for present value calculations
    pub disc_id: CurveId,
    /// Forward curve identifier (e.g., USD-SOFR-3M)
    pub fwd_id: CurveId,
    /// Spread in basis points over the floating rate
    pub spread_bp: F,
    /// Day count convention for accrual calculations
    pub day_count: DayCount,
}

impl FinancingLegSpec {
    /// Create a new financing leg specification
    pub fn new(
        disc_id: impl Into<String>,
        fwd_id: impl Into<String>,
        spread_bp: F,
        day_count: DayCount,
    ) -> Self {
        Self {
            disc_id: CurveId::new(disc_id),
            fwd_id: CurveId::new(fwd_id),
            spread_bp,
            day_count,
        }
    }
}

/// Specification for TRS total return legs
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TotalReturnLegSpec {
    /// Reference index or asset identifier
    pub reference_id: String,
    /// Initial price/level (if known, otherwise fetched from market)
    pub initial_level: Option<F>,
    /// Whether to include dividends/distributions in the return calculation
    pub include_distributions: bool,
}
