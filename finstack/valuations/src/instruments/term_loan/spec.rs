//! Serde-stable specification types for term loans and DDTL features.
//!
//! This module defines the serializable specification structures for term loans
//! including delayed-draw term loans (DDTL), covenant events, amortization schedules,
//! and call provisions.
//!
//! # Overview
//!
//! All types in this module are designed for stable serialization with:
//! - `#[serde(deny_unknown_fields)]` to catch configuration errors
//! - Explicit field naming for long-lived pipelines
//! - Conversion to/from runtime [`TermLoan`](super::types::TermLoan) instances
//!
//! # Key Types
//!
//! - [`TermLoanSpec`]: Complete loan specification (serializable)
//! - [`DdtlSpec`]: Delayed-draw term loan features
//! - [`CovenantSpec`]: Covenant-driven events
//! - [`AmortizationSpec`]: Principal repayment schedules
//! - [`LoanCallSchedule`]: Borrower prepayment options
//! - [`OidPolicy`]: Original issue discount handling
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::instruments::term_loan::spec::*;
//! use finstack_valuations::instruments::term_loan::RateSpec;
//! use finstack_valuations::cashflow::builder::specs::CouponType;
//! use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::*;
//! use finstack_core::types::{InstrumentId, CurveId};
//! use time::Month;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let spec = TermLoanSpec {
//!     id: InstrumentId::new("TL-001"),
//!     discount_curve_id: CurveId::new("USD-CREDIT"),
//!     currency: Currency::USD,
//!     issue: create_date(2025, Month::January, 1)?,
//!     maturity: create_date(2030, Month::January, 1)?,
//!     rate: RateSpec::Fixed { rate_bp: 600 },
//!     pay_freq: Frequency::quarterly(),
//!     day_count: DayCount::Act360,
//!     bdc: BusinessDayConvention::ModifiedFollowing,
//!     calendar_id: None,
//!     stub: StubKind::None,
//!     amortization: AmortizationSpec::None,
//!     coupon_type: CouponType::Cash,
//!     upfront_fee: None,
//!     ddtl: None,
//!     covenants: None,
//!     oid_eir: None,
//!     pricing_overrides: PricingOverrides::default(),
//!     call_schedule: None,
//! };
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`super::types::TermLoan`] for the runtime instrument type
//! - [`super::cashflows`] for cashflow generation
//! - [`super::pricing`] for valuation

use crate::instruments::pricing_overrides::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::types::RateSpec;

/// Original Issue Discount (OID) policy for term loan origination.
///
/// OID represents the discount from par value at loan origination. The policy
/// determines how the discount is handled: withheld from proceeds or tracked separately.
///
/// # Industry Practice
///
/// OID is common in institutional term loans and private credit, particularly for:
/// - Leveraged buyout financing (LBO loans)
/// - Distressed refinancings
/// - High-yield institutional term loans
///
/// Typical OID ranges from 1-5% (100-500 bps) of par value.
///
/// # Accounting Treatment
///
/// OID affects accounting under GAAP/IFRS:
/// - **Withheld**: Reduces initial cash proceeds, increases effective yield
/// - **Separate**: May be accounted as upfront fee or amortized discount
///
/// For full effective interest rate (EIR) amortization, see the experimental
/// [`OidEirSpec`] (not yet implemented).
///
/// # Variants
///
/// - `WithheldPct`: Discount as percentage withheld from funded amount
/// - `WithheldAmount`: Fixed amount withheld from funded proceeds  
/// - `SeparatePct`: Percentage tracked separately, not withheld
/// - `SeparateAmount`: Fixed amount tracked separately
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::term_loan::spec::OidPolicy;
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
///
/// // 2% OID withheld from proceeds
/// let oid = OidPolicy::WithheldPct(200);  // 200 bps = 2%
///
/// // $50,000 fixed OID
/// let oid_fixed = OidPolicy::WithheldAmount(Money::new(50_000.0, Currency::USD));
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum OidPolicy {
    /// Discount as percentage (basis points) withheld from funded proceeds
    WithheldPct(i32),
    /// Fixed discount amount withheld from funded proceeds
    WithheldAmount(Money),
    /// Discount as percentage tracked separately for amortization
    SeparatePct(i32),
    /// Fixed discount amount tracked separately for amortization
    SeparateAmount(Money),
}

/// Draw event for delayed-draw term loans (DDTL).
///
/// Represents a scheduled or actual draw against the commitment, reducing
/// available capacity and increasing outstanding principal.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DrawEvent {
    /// Date of the draw
    pub date: Date,
    /// Amount drawn from available commitment
    pub amount: Money,
}

/// Commitment step-down event for DDTL facilities.
///
/// Reduces the total commitment limit at a specified date, typically used
/// to match construction completion or covenant requirements.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CommitmentStepDown {
    /// Effective date of the step-down
    pub date: Date,
    /// New (lower) commitment limit after step-down
    pub new_limit: Money,
}

/// Basis for calculating commitment fees on undrawn portions.
///
/// Determines the denominator for commitment fee calculations on
/// revolving or delayed-draw facilities.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum CommitmentFeeBase {
    /// Fee based on total undrawn amount only
    Undrawn,
    /// Fee based on commitment limit minus outstanding principal
    CommitmentMinusOutstanding,
}

/// Delayed-draw term loan (DDTL) specification.
///
/// Models a term loan with commitment period during which borrower may draw
/// down funds, subject to availability dates, step-downs, and fees.
///
/// # Industry Practice
///
/// DDTLs are common in:
/// - **Construction financing**: Funds released as construction milestones are met
/// - **Acquisition financing**: Delayed funding for earn-outs or contingent payments
/// - **Working capital facilities**: Drawn as needed within commitment period
///
/// Typical features:
/// - Commitment period: 6-24 months
/// - Commitment fees: 25-50 bps on undrawn amounts
/// - Usage fees: 0-25 bps on drawn amounts
/// - Step-downs: Commitment reduces at milestones (e.g., construction completion)
///
/// # Fee Conventions
///
/// - **Commitment fee**: Paid on undrawn commitment (compensates lender for availability)
/// - **Usage fee**: Paid on drawn amounts (additive to interest margin)
/// - **OID**: May be withheld at each draw or tracked separately
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::term_loan::spec::*;
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::create_date;
/// use time::Month;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let ddtl = DdtlSpec {
///     commitment_limit: Money::new(10_000_000.0, Currency::USD),
///     availability_start: create_date(2025, Month::January, 1)?,
///     availability_end: create_date(2026, Month::January, 1)?,
///     draws: vec![],
///     commitment_step_downs: vec![],
///     usage_fee_bp: 50,        // 50 bps usage fee
///     commitment_fee_bp: 25,   // 25 bps commitment fee
///     fee_base: CommitmentFeeBase::Undrawn,
///     oid_policy: None,
/// };
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DdtlSpec {
    /// Total commitment limit available for draws
    pub commitment_limit: Money,
    /// First date draws are permitted
    pub availability_start: Date,
    /// Last date draws are permitted (commitment expiry)
    pub availability_end: Date,
    /// Scheduled or actual draw events
    pub draws: Vec<DrawEvent>,
    /// Commitment step-down schedule
    pub commitment_step_downs: Vec<CommitmentStepDown>,
    /// Usage fee in basis points (on drawn amounts)
    pub usage_fee_bp: i32,
    /// Commitment fee in basis points (on undrawn amounts)
    pub commitment_fee_bp: i32,
    /// Basis for commitment fee calculation
    pub fee_base: CommitmentFeeBase,
    /// Original issue discount policy, if applicable
    pub oid_policy: Option<OidPolicy>,
}

/// Margin step-up event (covenant penalty or scheduled increase).
///
/// Increases the interest margin by a fixed amount at a specified date,
/// typically triggered by covenant breach or scheduled rating migration.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MarginStepUp {
    /// Effective date of margin increase
    pub date: Date,
    /// Increase in margin (basis points)
    pub delta_bp: i32,
}

/// Payment-in-kind (PIK) toggle event.
///
/// Enables or disables PIK interest at a specified date. When enabled,
/// a portion of interest may be capitalized rather than paid in cash.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PikToggle {
    /// Date PIK feature is toggled
    pub date: Date,
    /// True to enable PIK, false to disable
    pub enable_pik: bool,
}

/// Cash sweep event (mandatory prepayment from excess cash flow).
///
/// Represents scheduled or covenant-triggered prepayment from borrower's
/// excess cash flow, reducing outstanding principal.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CashSweepEvent {
    /// Date of cash sweep prepayment
    pub date: Date,
    /// Amount of mandatory prepayment
    pub amount: Money,
}

/// Covenant-driven events for term loans.
///
/// Aggregates all covenant-triggered or scheduled events that modify
/// loan terms, including margin increases, PIK toggles, cash sweeps,
/// and draw restrictions.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CovenantSpec {
    /// Margin step-up schedule
    pub margin_stepups: Vec<MarginStepUp>,
    /// PIK toggle schedule
    pub pik_toggles: Vec<PikToggle>,
    /// Cash sweep (mandatory prepayment) schedule
    pub cash_sweeps: Vec<CashSweepEvent>,
    /// Dates on which draws are prohibited (covenant breach or scheduled)
    pub draw_stop_dates: Vec<Date>,
}

/// Principal amortization schedule specification.
///
/// Defines how the loan principal is amortized over its life,
/// from no amortization (bullet) to custom schedules.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum AmortizationSpec {
    /// Bullet loan with no scheduled amortization
    None,
    /// Linear amortization between start and end dates
    Linear {
        /// Amortization start date
        start: Date,
        /// Amortization end date (full repayment)
        end: Date,
    },
    /// Fixed percentage of original principal per period
    PercentPerPeriod {
        /// Percentage in basis points per payment period
        bp: i32,
    },
    /// Custom amortization schedule with explicit principal payments
    Custom(Vec<(Date, Money)>),
}

/// Complete term loan specification with covenant and DDTL features.
///
/// Comprehensive specification for institutional term loans including:
/// - Amortization schedules (bullet, linear, custom)
/// - Delayed-draw capabilities with commitment fees
/// - Payment-in-kind (PIK) features
/// - Covenant-driven events (margin step-ups, cash sweeps)
/// - Original issue discount (OID) handling
/// - Optional borrower callability
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::term_loan::spec::*;
/// use finstack_valuations::instruments::term_loan::types::RateSpec;
/// use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
/// use finstack_valuations::cashflow::builder::specs::CouponType;
/// use finstack_valuations::cashflow::builder::FloatingRateSpec;
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::*;
/// use finstack_core::types::{InstrumentId, CurveId};
/// use time::Month;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Example: Floating-rate term loan with SOFR + 300 bps
/// let floating_spec = FloatingRateSpec {
///     index_id: CurveId::new("USD-SOFR-3M"),
///     spread_bp: 300.0,
///     gearing: 1.0,
///     floor_bp: Some(0.0),  // 0% floor
///     cap_bp: None,
///     reset_freq: Frequency::quarterly(),
///     reset_lag_days: 2,
///     dc: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     calendar_id: None,
/// };
///
/// let spec = TermLoanSpec {
///     id: InstrumentId::new("TL-001"),
///     discount_curve_id: CurveId::new("USD-CREDIT"),
///     currency: Currency::USD,
///     issue: create_date(2025, Month::January, 15)?,
///     maturity: create_date(2030, Month::January, 15)?,
///     rate: RateSpec::Floating(floating_spec),
///     pay_freq: Frequency::quarterly(),
///     day_count: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     calendar_id: None,
///     stub: StubKind::None,
///     amortization: AmortizationSpec::None,  // Bullet loan
///     coupon_type: CouponType::Cash,
///     upfront_fee: None,
///     ddtl: None,
///     covenants: None,
///     // oid_eir: None, // Deprecated field removed
///     pricing_overrides: PricingOverrides::default(),
///     call_schedule: None,
/// };
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TermLoanSpec {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Optional credit curve ID for hazard rate / credit risk calculations.
    ///
    /// If not provided, defaults to `discount_curve_id` (risky discounting).
    pub credit_curve_id: Option<CurveId>,
    /// Loan currency
    pub currency: Currency,
    /// Loan issue/origination date
    pub issue: Date,
    /// Final maturity date
    pub maturity: Date,
    /// Interest rate specification (fixed or floating)
    pub rate: RateSpec,
    /// Payment frequency for interest and principal
    pub pay_freq: Frequency,
    /// Day count convention for interest accrual
    pub day_count: DayCount,
    /// Business day convention for schedule adjustment
    pub bdc: BusinessDayConvention,
    /// Optional holiday calendar ID (default: no holidays)
    pub calendar_id: Option<String>,
    /// Stub period treatment
    pub stub: StubKind,
    /// Principal amortization schedule
    pub amortization: AmortizationSpec,
    /// Coupon characterization (Cash, PIK, or Split with optional toggles).
    ///
    /// This field controls whether interest is paid in cash, capitalized (PIK),
    /// or split between the two. It does NOT control payment timing (which is
    /// assumed to be in arrears). For dynamic PIK toggles, see `CovenantSpec::pik_toggles`.
    pub coupon_type: crate::cashflow::builder::specs::CouponType,
    /// Optional upfront origination fee
    pub upfront_fee: Option<Money>,
    /// Optional delayed-draw term loan features
    pub ddtl: Option<DdtlSpec>,
    /// Optional covenant-driven events
    pub covenants: Option<CovenantSpec>,
    /// Pricing overrides (yield, price, etc.)
    pub pricing_overrides: PricingOverrides,
    /// Optional call schedule (borrower callability)
    pub call_schedule: Option<LoanCallSchedule>,
}

/// Borrower call option on term loan.
///
/// Represents the borrower's right to prepay the loan at a specified
/// redemption price (typically at premium to par for early calls,
/// approaching par near maturity).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct LoanCall {
    /// Call date (earliest prepayment date for this call provision)
    pub date: Date,
    /// Redemption price as percentage of par (e.g., 102.0 = 102% of par)
    pub price_pct_of_par: f64,
}

/// Complete call schedule for callable term loans.
///
/// Aggregates all borrower call provisions, typically with step-down
/// premiums as the loan ages (e.g., 103% in year 1, 102% in year 2, par thereafter).
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::term_loan::spec::{LoanCallSchedule, LoanCall};
/// use finstack_core::dates::create_date;
/// use time::Month;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let schedule = LoanCallSchedule {
///     calls: vec![
///         LoanCall {
///             date: create_date(2027, Month::January, 15)?,
///             price_pct_of_par: 103.0,  // 3% premium in year 2
///         },
///         LoanCall {
///             date: create_date(2028, Month::January, 15)?,
///             price_pct_of_par: 101.5,  // 1.5% premium in year 3
///         },
///         LoanCall {
///             date: create_date(2029, Month::January, 15)?,
///             price_pct_of_par: 100.0,  // At par thereafter
///         },
///     ],
/// };
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct LoanCallSchedule {
    /// Ordered call provisions (typically sorted by date with descending premiums)
    pub calls: Vec<LoanCall>,
}
