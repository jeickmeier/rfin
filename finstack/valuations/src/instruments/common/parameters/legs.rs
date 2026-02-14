//! Common leg specification types for interest rate and credit instruments.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::types::{CurveId, Percentage};
use rust_decimal::Decimal;

use serde::{Deserialize, Serialize};

/// Direction for instrument legs (universal for IRS, CDS, etc.)
///
/// For interest rate swaps: Pay = pay fixed/receive floating, Receive = receive fixed/pay floating
/// For credit default swaps: Pay = buy protection (pay premium), Receive = sell protection (receive premium)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayReceive {
    /// Pay the primary leg (fixed rate in IRS, protection premium in CDS)
    #[serde(rename = "pay_fixed", alias = "PayFixed")]
    PayFixed,
    /// Receive the primary leg (fixed rate in IRS, protection premium in CDS)
    #[serde(rename = "receive_fixed", alias = "ReceiveFixed")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ParRateMethod {
    /// Use forward-curve based float PV over the schedule (market standard)
    ForwardBased,
    /// Use discount-curve ratio for bootstrapping
    DiscountRatio,
}

/// Specification for fixed rate legs in interest rate swaps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedLegSpec {
    /// Discount curve identifier for pricing
    pub discount_curve_id: CurveId,
    /// Fixed rate (e.g., 0.05 for 5%)
    pub rate: Decimal,
    /// Payment frequency
    #[serde(alias = "freq")]
    pub frequency: Tenor,
    /// Day count convention for accrual
    #[serde(alias = "dc")]
    pub day_count: DayCount,
    /// Business day convention for payment dates
    #[serde(default = "crate::serde_defaults::bdc_modified_following")]
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments
    pub calendar_id: Option<String>,
    /// Stub period handling rule
    #[serde(default = "crate::serde_defaults::stub_short_front")]
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
    #[serde(default)]
    pub payment_delay_days: i32,
    /// End-of-month roll convention (default: false).
    ///
    /// When `true`, if the start date falls on the last business day of a month,
    /// all subsequent roll dates will also fall on the last business day of their
    /// respective months. This matches QuantLib's `MakeOIS` default behavior.
    ///
    /// # Market Standard
    ///
    /// Per ISDA 2006 Definitions Section 4.18, the End-of-Month convention should
    /// be applied when the effective date is the last business day of a month.
    /// Most professional systems (QuantLib, Bloomberg SWDF) default to `true`.
    #[serde(default)]
    pub end_of_month: bool,
}

/// Specification for floating rate legs in interest rate swaps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatLegSpec {
    /// Discount curve identifier for pricing
    pub discount_curve_id: CurveId,
    /// Forward curve identifier for rate projections
    pub forward_curve_id: CurveId,
    /// Spread in basis points added to the forward rate
    pub spread_bp: Decimal,
    /// Payment frequency
    #[serde(alias = "freq")]
    pub frequency: Tenor,
    /// Day count convention for accrual
    #[serde(alias = "dc")]
    pub day_count: DayCount,
    /// Business day convention for payment dates
    #[serde(default = "crate::serde_defaults::bdc_modified_following")]
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments
    pub calendar_id: Option<String>,
    /// Stub period handling rule
    #[serde(default = "crate::serde_defaults::stub_short_front")]
    pub stub: StubKind,
    /// Reset lag in business days for floating rate
    pub reset_lag_days: i32,
    /// Optional calendar for rate fixing (reset lag)
    #[serde(default)]
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
    #[serde(default)]
    pub compounding: crate::instruments::rates::irs::FloatingLegCompounding,
    /// Payment delay in business days after period end (default: 0).
    ///
    /// Bloomberg OIS swaps typically use 2 business days payment delay.
    /// The actual payment date is adjusted from the period end date by
    /// this many business days using the leg's calendar.
    #[serde(default)]
    pub payment_delay_days: i32,
    /// End-of-month roll convention (default: false).
    ///
    /// When `true`, if the start date falls on the last business day of a month,
    /// all subsequent roll dates will also fall on the last business day of their
    /// respective months. This matches QuantLib's `MakeOIS` default behavior.
    ///
    /// # Market Standard
    ///
    /// Per ISDA 2006 Definitions Section 4.18, the End-of-Month convention should
    /// be applied when the effective date is the last business day of a month.
    /// Most professional systems (QuantLib, Bloomberg SWDF) default to `true`.
    #[serde(default)]
    pub end_of_month: bool,
}

/// Specification for basis swap legs (floating vs floating)
///
/// A basis swap leg represents one side of a floating-for-floating interest rate swap,
/// where two parties exchange payments linked to different floating rate indices
/// (e.g., 3M SOFR vs 6M SOFR).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasisSwapLeg {
    /// Forward curve identifier for this leg
    pub forward_curve_id: CurveId,
    /// Payment frequency for the leg
    pub frequency: Tenor,
    /// Day count convention for accrual calculations
    pub day_count: DayCount,
    /// Business day convention for date adjustments
    #[serde(default = "crate::serde_defaults::bdc_modified_following")]
    pub bdc: BusinessDayConvention,
    /// Spread added to the floating rate, in **decimal** form (not basis points).
    ///
    /// # Units
    ///
    /// - `0.0005` represents 5 basis points (5bp)
    /// - `0.01` represents 100 basis points (1%)
    /// - `-0.001` represents -10 basis points
    ///
    /// # Typical Market Range
    ///
    /// Basis spreads in liquid markets typically range from -50bp to +50bp (-0.005 to +0.005).
    /// Values outside ±5000bp (±50%, i.e., |spread| > 0.50) are considered extreme and
    /// will trigger a validation warning during pricing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_valuations::instruments::rates::basis_swap::BasisSwapLeg;
    /// use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
    /// use finstack_core::types::CurveId;
    ///
    /// let leg = BasisSwapLeg {
    ///     forward_curve_id: CurveId::new("USD-SOFR-3M"),
    ///     frequency: Tenor::quarterly(),
    ///     day_count: DayCount::Act360,
    ///     bdc: BusinessDayConvention::ModifiedFollowing,
    ///     spread: 0.0005, // 5bp spread
    ///     payment_lag_days: 0,
    ///     reset_lag_days: 2,
    /// };
    /// ```
    pub spread: f64,
    /// Payment lag in business days after period end (default: 0).
    ///
    /// E.g., `payment_lag_days: 2` means payment occurs 2 business days after the
    /// accrual period end date.
    #[serde(default)]
    pub payment_lag_days: i32,
    /// Reset lag in business days before period start (default: 0).
    ///
    /// E.g., `reset_lag_days: 2` means the rate fixing occurs 2 business days before
    /// the accrual period start date. This follows standard market convention where
    /// fixing typically precedes the accrual period.
    #[serde(default)]
    pub reset_lag_days: i32,
}

/// Specification for CDS premium legs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumLegSpec {
    /// Start date of protection
    pub start: Date,
    /// End date of protection
    pub end: Date,
    /// Payment frequency
    #[serde(alias = "freq")]
    pub frequency: Tenor,
    /// Stub convention
    #[serde(default = "crate::serde_defaults::stub_short_front")]
    pub stub: StubKind,
    /// Business day convention
    #[serde(default = "crate::serde_defaults::bdc_modified_following")]
    pub bdc: BusinessDayConvention,
    /// Holiday calendar identifier
    pub calendar_id: Option<String>,
    /// Day count convention
    #[serde(alias = "dc")]
    pub day_count: DayCount,
    /// Fixed spread in basis points (e.g., 100 = 100bp = 1%)
    pub spread_bp: Decimal,
    /// Discount curve identifier
    pub discount_curve_id: CurveId,
}

/// Specification for CDS protection legs
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Create a new protection leg specification using typed percentage recovery.
    ///
    /// # Arguments
    /// * `credit_curve_id` - Identifier for the hazard/credit curve
    /// * `recovery_rate` - Recovery rate as a percentage (e.g., 40.0 = 40%)
    /// * `settlement_delay` - Settlement delay in business days
    ///
    /// # Errors
    /// Returns an error if `recovery_rate` is outside [0.0, 1.0] in decimal terms.
    pub fn new_pct(
        credit_curve_id: impl Into<CurveId>,
        recovery_rate: Percentage,
        settlement_delay: u16,
    ) -> finstack_core::Result<Self> {
        let recovery_rate_decimal = recovery_rate.as_decimal();
        Self::validate_recovery_rate(recovery_rate_decimal)?;
        Ok(Self {
            credit_curve_id: credit_curve_id.into(),
            recovery_rate: recovery_rate_decimal,
            settlement_delay,
        })
    }

    /// Validate that recovery rate is within valid bounds [0, 1].
    ///
    /// Delegates to the shared internal recovery-rate validator.
    ///
    /// # Errors
    /// Returns an error if recovery rate is outside the valid range.
    pub fn validate_recovery_rate(recovery_rate: f64) -> finstack_core::Result<()> {
        crate::instruments::common_impl::validation::validate_recovery_rate(recovery_rate)
    }
}

// Note: Settlement type (cash/physical/auction) is descriptive-only and does not
// impact current pricing. It has been removed from `ProtectionLegSpec` to keep
// the pricing surface minimal and consistent. If needed, store as metadata in
// instrument `Attributes`.

/// Specification for TRS financing legs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancingLegSpec {
    /// Discount curve identifier for present value calculations
    pub discount_curve_id: CurveId,
    /// Forward curve identifier (e.g., USD-SOFR-3M)
    pub forward_curve_id: CurveId,
    /// Spread in basis points over the floating rate (e.g., 50 = 50bp = 0.5%)
    pub spread_bp: Decimal,
    /// Day count convention for accrual calculations
    pub day_count: DayCount,
}

impl FinancingLegSpec {
    /// Create a new financing leg specification
    pub fn new(
        discount_curve_id: impl Into<String>,
        forward_curve_id: impl Into<String>,
        spread_bp: Decimal,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalReturnLegSpec {
    /// Reference index or asset identifier
    pub reference_id: String,
    /// Initial price/level (if known, otherwise fetched from market)
    pub initial_level: Option<f64>,
    /// Whether to include dividends/distributions in the return calculation
    pub include_distributions: bool,
}
