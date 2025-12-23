//! Term loan instrument type and core specifications.
//!
//! This module defines the [`TermLoan`] instrument type and its associated specifications
//! including rate types, trait implementations, and conversion from [`TermLoanSpec`].
//!
//! # Overview
//!
//! The [`TermLoan`] type represents a fully-validated term loan instrument with:
//! - Fixed or floating rate specifications
//! - Optional DDTL (delayed-draw) features
//! - Covenant-driven events
//! - Amortization schedules
//! - Call schedules
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::instruments::term_loan::{TermLoan, RateSpec};
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::dates::*;
//! use finstack_core::types::{InstrumentId, CurveId};
//! use time::Month;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a simple example term loan
//! let loan = TermLoan::example();
//!
//! assert_eq!(loan.currency, Currency::USD);
//! assert_eq!(loan.notional_limit, Money::new(10_000_000.0, Currency::USD));
//! # Ok(())
//! # }
//! ```
//!
//! # See Also
//!
//! - [`TermLoanSpec`] for the serializable specification type
//! - [`RateSpec`] for rate type definitions
//! - [`super::spec`] module for all specification types

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::spec::{AmortizationSpec, CovenantSpec, DdtlSpec, LoanCallSchedule};
use crate::cashflow::builder::specs::CouponType;
use crate::cashflow::builder::FloatingRateSpec;
use crate::instruments::common::traits::Attributes;
use crate::instruments::pricing_overrides::PricingOverrides;

/// Rate specification for term loans.
///
///  Defines whether the loan uses fixed or floating rate interest, with full
/// support for floating rate features including floors, caps, and leverage.
///
/// # Variants
///
/// - [`Fixed`](RateSpec::Fixed): Constant rate specified in basis points
/// - [`Floating`](RateSpec::Floating): Index-based rate with spread and optional limits
///
/// # Examples
///
/// Fixed rate loan:
/// ```rust
/// use finstack_valuations::instruments::term_loan::RateSpec;
///
/// let fixed_rate = RateSpec::Fixed { rate_bp: 600 };  // 6% fixed
/// ```
///
/// Floating rate with floor:
/// ```rust
/// use finstack_valuations::instruments::term_loan::RateSpec;
/// use finstack_valuations::cashflow::builder::FloatingRateSpec;
/// use finstack_core::dates::{DayCount, BusinessDayConvention, Tenor};
/// use finstack_core::types::CurveId;
///
/// let floating = RateSpec::Floating(FloatingRateSpec {
///     index_id: CurveId::new("USD-SOFR-3M"),
///     spread_bp: 300.0,     // +300 bps spread
///     gearing: 1.0,
///     gearing_includes_spread: true,
///     floor_bp: Some(0.0),  // 0% floor
///     all_in_floor_bp: None,
///     cap_bp: None,
///     index_cap_bp: None,
///     reset_freq: Tenor::quarterly(),
///     reset_lag_days: 2,
///     dc: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     calendar_id: None,
///     fixing_calendar_id: None,
/// });
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RateSpec {
    /// Fixed annual rate in basis points
    Fixed {
        /// Fixed rate in basis points (e.g., 600 = 6%)
        rate_bp: i32,
    },

    /// Floating rate using canonical FloatingRateSpec.
    ///
    /// Uses the standard floating rate specification with full support
    /// for floors, caps, gearing, and reset conventions.
    Floating(FloatingRateSpec),
}

/// Term loan instrument with covenant and DDTL support.
///
/// Represents a fully-validated institutional term loan with support for:
/// - Fixed or floating interest rates
/// - Delayed-draw term loan (DDTL) features
/// - Payment-in-kind (PIK) interest
/// - Flexible amortization schedules
/// - Covenant-driven events (margin step-ups, cash sweeps, PIK toggles)
/// - Original issue discount (OID) handling
/// - Borrower call schedules
///
/// # Construction
///
/// Create via [`TermLoanSpec`] conversion or use the builder pattern:
///
/// ```rust,no_run
/// use finstack_valuations::instruments::term_loan::TermLoan;
///
/// // `TermLoanSpec -> TermLoan` conversion is not part of the public API;
/// // use the builder or `TermLoan::example()` for a fully validated instance.
/// let loan = TermLoan::example();
/// # let _ = loan;
/// ```
///
/// # Cashflow Generation
///
/// Uses the [`CashflowProvider`](crate::cashflow::traits::CashflowProvider) trait:
/// - `build_schedule()` returns holder-view flows (coupons, amortization, redemptions)
/// - `build_full_schedule()` returns internal engine schedule with all flow types
///
/// # Pricing
///
/// Implements [`Instrument::value()`](crate::instruments::common::traits::Instrument::value)
/// using deterministic cashflow discounting. PIK interest is capitalized and excluded from PV.
///
/// # Invariants
///
/// - `issue < maturity`
/// - `notional_limit.currency() == currency`
/// - All monetary amounts are in the same currency
/// - Amortization does not exceed outstanding principal
///
/// # Thread Safety
///
/// This type is `Send + Sync` as all fields are thread-safe.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TermLoan {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Currency for all cashflows
    pub currency: Currency,

    /// Maximum commitment / notional limit
    pub notional_limit: Money,

    /// Issue (effective) date
    pub issue: Date,

    /// Maturity date
    pub maturity: Date,

    /// Rate specification (fixed or floating)
    pub rate: RateSpec,

    /// Payment frequency for coupons/fees
    pub pay_freq: Tenor,

    /// Day count convention
    pub day_count: DayCount,

    /// Business day convention
    pub bdc: BusinessDayConvention,

    /// Optional calendar id for adjustments
    pub calendar_id: Option<String>,

    /// Stub rule
    pub stub: StubKind,

    /// Discount curve identifier
    pub discount_curve_id: CurveId,

    /// Optional credit curve identifier (defaults to discount_curve_id if None)
    pub credit_curve_id: Option<CurveId>,

    /// Amortization specification
    pub amortization: AmortizationSpec,

    /// Coupon split type (Cash/PIK/Split)
    pub coupon_type: CouponType,

    /// Upfront fee at issue (if any)
    pub upfront_fee: Option<Money>,

    /// Optional DDTL parameters; None => plain term loan
    pub ddtl: Option<DdtlSpec>,

    /// Optional covenant spec
    pub covenants: Option<CovenantSpec>,

    /// Pricing overrides (quoted price, seed, etc.)
    pub pricing_overrides: PricingOverrides,

    /// Optional call schedule (borrower callability)
    pub call_schedule: Option<LoanCallSchedule>,

    /// Attributes for tagging and scenarios
    pub attributes: Attributes,
}

impl TermLoan {
    /// Create a canonical example term loan for testing and documentation.
    ///
    /// Generates a 5-year USD term loan with:
    /// - $10M notional
    /// - 6% fixed rate
    /// - Quarterly payments
    /// - 2.5% per-period amortization
    /// - Act/360 day count
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::term_loan::TermLoan;
    /// use finstack_core::currency::Currency;
    ///
    /// let loan = TermLoan::example();
    /// assert_eq!(loan.currency, Currency::USD);
    /// assert_eq!(loan.notional_limit.amount(), 10_000_000.0);
    /// ```
    pub fn example() -> Self {
        use finstack_core::dates::BusinessDayConvention;
        use finstack_core::dates::StubKind;
        use time::Month;
        TermLoanBuilder::new()
            .id(InstrumentId::new("TERM-LOAN-USD-5Y"))
            .currency(Currency::USD)
            .notional_limit(Money::new(10_000_000.0, Currency::USD))
            .issue(Date::from_calendar_date(2024, Month::January, 1).expect("Valid example date"))
            .maturity(
                Date::from_calendar_date(2029, Month::January, 1).expect("Valid example date"),
            )
            .rate(RateSpec::Fixed { rate_bp: 600 }) // 6%
            .pay_freq(Tenor::quarterly())
            .day_count(DayCount::Act360)
            .bdc(BusinessDayConvention::ModifiedFollowing)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .credit_curve_id_opt(None)
            .amortization(super::spec::AmortizationSpec::PercentPerPeriod { bp: 250 }) // 2.5% per period
            .coupon_type(crate::cashflow::builder::specs::CouponType::Cash)
            .upfront_fee_opt(None)
            .ddtl_opt(None)
            .covenants_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .call_schedule_opt(None)
            .attributes(Attributes::new())
            .build()
            .expect("Example TermLoan construction should not fail")
    }
}

impl crate::instruments::common::traits::Instrument for TermLoan {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::TermLoan
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        // Delegate to discounting pricer (deterministic v1)
        crate::instruments::term_loan::pricing::TermLoanDiscountingPricer::price(
            self, curves, as_of,
        )
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }
}

impl crate::cashflow::traits::CashflowProvider for TermLoan {
    fn notional(&self) -> Option<finstack_core::money::Money> {
        Some(self.notional_limit)
    }

    fn build_schedule(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::cashflow::DatedFlows> {
        use finstack_core::cashflow::primitives::CFKind;

        // Get full internal schedule
        let schedule =
            crate::instruments::term_loan::cashflows::generate_cashflows(self, curves, as_of)?;

        // Filter to holder-view: only contractual inflows to a long lender
        // Include: coupons, amortization, positive notional redemptions
        // Exclude: funding legs (negative notional draws), PIK capitalization
        let mut flows: Vec<(finstack_core::dates::Date, finstack_core::money::Money)> = Vec::new();

        for cf in &schedule.flows {
            match cf.kind {
                // Include coupons and interest flows as-is (holder receives them)
                CFKind::Fixed | CFKind::FloatReset | CFKind::Stub => {
                    flows.push((cf.date, cf.amount));
                }
                // Amortization principal repayment: holder receives this
                CFKind::Amortization => {
                    flows.push((cf.date, cf.amount));
                }
                // Notional: only redemptions (positive), exclude draws (negative)
                CFKind::Notional if cf.amount.amount() > 0.0 => {
                    flows.push((cf.date, cf.amount));
                }
                // Exclude funding legs (negative notional), PIK capitalization, and other kinds
                _ => {}
            }
        }

        // Sort by date for deterministic ordering
        flows.sort_by_key(|(d, _)| *d);

        Ok(flows)
    }

    fn build_full_schedule(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::cashflow::builder::CashFlowSchedule> {
        crate::instruments::term_loan::cashflows::generate_cashflows(self, curves, as_of)
    }
}

// Allow generic metric calculators to fetch discount curve id
impl crate::instruments::common::pricing::HasDiscountCurve for TermLoan {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement HasCreditCurve for generic CS01 calculators.
//
// For term loans we currently reuse the discount curve as the credit curve identifier.
// This is sufficient for 80/20 CS01 support; users should ensure a corresponding
// hazard/credit curve exists in the market data if they request CS01 metrics.
impl crate::metrics::HasCreditCurve for TermLoan {
    fn credit_curve_id(&self) -> &finstack_core::types::CurveId {
        self.credit_curve_id
            .as_ref()
            .unwrap_or(&self.discount_curve_id)
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for TermLoan {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        let mut builder = crate::instruments::common::traits::InstrumentCurves::builder();
        builder = builder.discount(self.discount_curve_id.clone());
        if let Some(cc) = &self.credit_curve_id {
            builder = builder.credit(cc.clone());
        }
        builder.build()
    }
}
