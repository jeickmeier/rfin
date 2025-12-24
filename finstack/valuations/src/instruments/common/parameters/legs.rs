//! Common leg specification types for interest rate and credit instruments.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::types::CurveId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Direction for instrument legs (universal for IRS, CDS, etc.)
///
/// For interest rate swaps: Pay = pay fixed/receive floating, Receive = receive fixed/pay floating
/// For credit default swaps: Pay = buy protection (pay premium), Receive = sell protection (receive premium)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PayReceive {
    /// Pay the primary leg (fixed rate in IRS, protection premium in CDS)
    #[cfg_attr(
        feature = "serde",
        serde(rename = "pay_fixed")
    )]
    PayFixed,
    /// Receive the primary leg (fixed rate in IRS, protection premium in CDS)
    #[cfg_attr(
        feature = "serde",
        serde(rename = "receive_fixed")
    )]
    ReceiveFixed,
}

impl PayReceive {
    /// Check if this is the payer side
    pub fn is_payer(&self) -> bool {
        matches!(self, Self::PayFixed)
    }

    /// Check if this is the receiver side
    pub fn is_receiver(&self) -> bool {
        matches!(self, Self::ReceiveFixed)
    }
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
            "pay_fixed" | "pay_protection" | "pay" | "buyer" | "buy" => Ok(PayReceive::PayFixed),
            "receive_fixed" | "receive_protection" | "receive" | "recv" | "seller" | "sell" => {
                Ok(PayReceive::ReceiveFixed)
            }
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
    pub discount_curve_id: CurveId,
    /// Fixed rate (e.g., 0.05 for 5%)
    pub rate: f64,
    /// Payment frequency
    pub freq: Tenor,
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
    /// Payment delay in business days after period end (default: 0).
    ///
    /// Bloomberg OIS swaps typically use 2 business days payment delay.
    /// The actual payment date is adjusted from the period end date by
    /// this many business days using the leg's calendar.
    #[cfg_attr(feature = "serde", serde(default))]
    pub payment_delay_days: i32,
}

/// Specification for floating rate legs in interest rate swaps
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FloatLegSpec {
    /// Discount curve identifier for pricing
    pub discount_curve_id: CurveId,
    /// Forward curve identifier for rate projections
    pub forward_curve_id: CurveId,
    /// Spread in basis points added to the forward rate
    pub spread_bp: f64,
    /// Payment frequency
    pub freq: Tenor,
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
    /// Optional calendar for rate fixing (reset lag)
    #[cfg_attr(feature = "serde", serde(default))]
    pub fixing_calendar_id: Option<String>,
    /// Start date of the floating leg
    pub start: Date,
    /// End date of the floating leg
    pub end: Date,
    /// Compounding method for floating coupons.
    ///
    /// Determines how floating rate coupons are calculated:
    /// - `Simple` (default): LIBOR-style simple interest
    /// - `CompoundedInArrears`: SOFR/SONIA-style daily compounding
    ///
    /// # Implementation Notes
    ///
    /// Compounded-in-arrears is implemented for IRS pricing in `instruments::irs` with
    /// support for lookback and observation shift conventions. For seasoned (already
    /// started) compounded swaps, pricing requires explicit fixings for observation
    /// dates prior to `as_of`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub compounding: crate::instruments::irs::FloatingLegCompounding,
    /// Payment delay in business days after period end (default: 0).
    ///
    /// Bloomberg OIS swaps typically use 2 business days payment delay.
    /// The actual payment date is adjusted from the period end date by
    /// this many business days using the leg's calendar.
    #[cfg_attr(feature = "serde", serde(default))]
    pub payment_delay_days: i32,
}

/// Specification for basis swap legs (floating vs floating)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasisSwapLeg {
    /// Forward curve identifier for this leg
    pub forward_curve_id: CurveId,
    /// Payment frequency for the leg
    pub frequency: Tenor,
    /// Day count convention for accrual calculations
    pub day_count: DayCount,
    /// Business day convention for date adjustments
    pub bdc: BusinessDayConvention,
    /// Optional spread in decimal form (e.g., 0.0005 for 5 basis points)
    pub spread: f64,
    /// Payment lag in business days (default: 0)
    #[cfg_attr(feature = "serde", serde(default))]
    pub payment_lag_days: i32,
    /// Reset lag in business days (default: 0)
    #[cfg_attr(feature = "serde", serde(default))]
    pub reset_lag_days: i32,
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
    pub freq: Tenor,
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
    pub discount_curve_id: CurveId,
}

/// Specification for CDS protection legs
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProtectionLegSpec {
    /// Credit curve identifier for default probabilities
    pub credit_curve_id: CurveId,
    /// Recovery rate (0.0 to 1.0)
    pub recovery_rate: f64,
    /// Settlement delay in business days
    pub settlement_delay: u16,
}

impl ProtectionLegSpec {
    /// Create a new protection leg specification with validation.
    ///
    /// # Arguments
    /// * `credit_curve_id` - Identifier for the hazard/credit curve
    /// * `recovery_rate` - Recovery rate in [0.0, 1.0] (e.g., 0.4 = 40%)
    /// * `settlement_delay` - Settlement delay in business days
    ///
    /// # Errors
    /// Returns an error if `recovery_rate` is outside [0.0, 1.0].
    pub fn new(
        credit_curve_id: impl Into<CurveId>,
        recovery_rate: f64,
        settlement_delay: u16,
    ) -> finstack_core::Result<Self> {
        Self::validate_recovery_rate(recovery_rate)?;
        Ok(Self {
            credit_curve_id: credit_curve_id.into(),
            recovery_rate,
            settlement_delay,
        })
    }

    /// Validate that recovery rate is within valid bounds [0, 1].
    ///
    /// # Errors
    /// Returns an error if recovery rate is outside the valid range.
    pub fn validate_recovery_rate(recovery_rate: f64) -> finstack_core::Result<()> {
        if !(0.0..=1.0).contains(&recovery_rate) {
            return Err(finstack_core::Error::Validation(format!(
                "Recovery rate must be between 0.0 and 1.0, got {}",
                recovery_rate
            )));
        }
        Ok(())
    }
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
    pub discount_curve_id: CurveId,
    /// Forward curve identifier (e.g., USD-SOFR-3M)
    pub forward_curve_id: CurveId,
    /// Spread in basis points over the floating rate
    pub spread_bp: f64,
    /// Day count convention for accrual calculations
    pub day_count: DayCount,
}

impl FinancingLegSpec {
    /// Create a new financing leg specification
    pub fn new(
        discount_curve_id: impl Into<String>,
        forward_curve_id: impl Into<String>,
        spread_bp: f64,
        day_count: DayCount,
    ) -> Self {
        Self {
            discount_curve_id: CurveId::new(discount_curve_id),
            forward_curve_id: CurveId::new(forward_curve_id),
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
