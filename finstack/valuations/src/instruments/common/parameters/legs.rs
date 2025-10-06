//! Common leg specification types for interest rate and credit instruments.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::types::CurveId;

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

impl std::fmt::Display for PayReceive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayReceive::PayFixed => write!(f, "pay_fixed"),
            PayReceive::ReceiveFixed => write!(f, "receive_fixed"),
        }
    }
}

impl std::str::FromStr for PayReceive {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "pay_fixed" | "pay" => Ok(PayReceive::PayFixed),
            "receive_fixed" | "receive" | "recv" => Ok(PayReceive::ReceiveFixed),
            other => Err(format!("Unknown pay/receive: {}", other)),
        }
    }
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
    pub disc_id: CurveId,
    /// Fixed rate (e.g., 0.05 for 5%)
    pub rate: f64,
    /// Payment frequency
    pub freq: Frequency,
    /// Day count convention for accrual
    pub dc: DayCount,
    /// Business day convention for payment dates
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments
    pub calendar_id: Option<String>,
    /// Stub period handling rule
    pub stub: StubKind,
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
    pub disc_id: CurveId,
    /// Forward curve identifier for rate projections
    pub fwd_id: CurveId,
    /// Spread in basis points added to the forward rate
    pub spread_bp: f64,
    /// Payment frequency
    pub freq: Frequency,
    /// Day count convention for accrual
    pub dc: DayCount,
    /// Business day convention for payment dates
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments
    pub calendar_id: Option<String>,
    /// Stub period handling rule
    pub stub: StubKind,
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
    pub spread: f64,
}

/// Specification for CDS premium legs
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PremiumLegSpec {
    /// Start date of protection
    pub start: Date,
    /// End date of protection
    pub end: Date,
    /// Payment frequency
    pub freq: Frequency,
    /// Stub convention
    pub stub: StubKind,
    /// Business day convention
    pub bdc: BusinessDayConvention,
    /// Holiday calendar identifier
    pub calendar_id: Option<String>,
    /// Day count convention
    pub dc: DayCount,
    /// Fixed spread in basis points
    pub spread_bp: f64,
    /// Discount curve identifier
    pub disc_id: CurveId,
}

/// Specification for CDS protection legs
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProtectionLegSpec {
    /// Credit curve identifier for default probabilities
    pub credit_id: CurveId,
    /// Recovery rate (0.0 to 1.0)
    pub recovery_rate: f64,
    /// Settlement delay in business days
    pub settlement_delay: u16,
}

// Note: Settlement type (cash/physical/auction) is descriptive-only and does not
// impact current pricing. It has been removed from `ProtectionLegSpec` to keep
// the pricing surface minimal and consistent. If needed, store as metadata in
// instrument `Attributes`.

/// Specification for TRS financing legs
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FinancingLegSpec {
    /// Discount curve identifier for present value calculations
    pub disc_id: CurveId,
    /// Forward curve identifier (e.g., USD-SOFR-3M)
    pub fwd_id: CurveId,
    /// Spread in basis points over the floating rate
    pub spread_bp: f64,
    /// Day count convention for accrual calculations
    pub day_count: DayCount,
}

impl FinancingLegSpec {
    /// Create a new financing leg specification
    pub fn new(
        disc_id: impl Into<String>,
        fwd_id: impl Into<String>,
        spread_bp: f64,
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
    pub initial_level: Option<f64>,
    /// Whether to include dividends/distributions in the return calculation
    pub include_distributions: bool,
}
