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
//! use finstack_valuations::instruments::fixed_income::term_loan::{TermLoan, RateSpec};
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
use finstack_core::dates::{
    calendar::calendar_by_id, BusinessDayConvention, Date, DateExt, DayCount, StubKind, Tenor,
};
use finstack_core::money::Money;
use finstack_core::types::{Bps, CurveId, InstrumentId, Rate};
use finstack_core::InputError;

use super::spec::{
    AmortizationSpec, CovenantSpec, DdtlSpec, LoanCallSchedule, OidEirSpec, TermLoanSpec,
};
use crate::cashflow::builder::specs::CouponType;
use crate::cashflow::builder::FloatingRateSpec;
use crate::instruments::common_impl::traits::Attributes;
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
/// use finstack_valuations::instruments::fixed_income::term_loan::RateSpec;
///
/// let fixed_rate = RateSpec::Fixed { rate_bp: 600 };  // 6% fixed
/// ```
///
/// Floating rate with floor:
/// ```rust
/// use finstack_valuations::instruments::fixed_income::term_loan::RateSpec;
/// use finstack_valuations::cashflow::builder::FloatingRateSpec;
/// use finstack_core::dates::{DayCount, BusinessDayConvention, Tenor};
/// use finstack_core::types::CurveId;
/// use rust_decimal_macros::dec;
///
/// let floating = RateSpec::Floating(FloatingRateSpec {
///     index_id: CurveId::new("USD-SOFR-3M"),
///     spread_bp: dec!(300),     // +300 bps spread
///     gearing: dec!(1),
///     gearing_includes_spread: true,
///     floor_bp: Some(dec!(0)),  // 0% floor
///     all_in_floor_bp: None,
///     cap_bp: None,
///     index_cap_bp: None,
///     reset_freq: Tenor::quarterly(),
///     reset_lag_days: 2,
///     dc: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     calendar_id: "weekends_only".to_string(),
///     fixing_calendar_id: None,
///     end_of_month: false,
///     payment_lag_days: 0,
///     overnight_compounding: None,
/// });
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
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

impl RateSpec {
    /// Create a fixed-rate spec using typed basis points.
    pub fn fixed_bps(rate: Bps) -> Self {
        Self::Fixed {
            rate_bp: rate.as_bps(),
        }
    }

    /// Create a fixed-rate spec using a typed rate.
    pub fn fixed_rate(rate: Rate) -> Self {
        Self::Fixed {
            rate_bp: rate.as_bps(),
        }
    }
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
/// use finstack_valuations::instruments::fixed_income::term_loan::spec::TermLoanSpec;
/// use finstack_valuations::instruments::fixed_income::term_loan::TermLoan;
///
/// # fn example(spec: TermLoanSpec) -> Result<(), Box<dyn std::error::Error>> {
/// let loan: TermLoan = spec.try_into()?;
/// # let _ = loan;
/// # Ok(())
/// # }
/// ```
///
/// # Cashflow Generation
///
/// Uses the [`CashflowProvider`](crate::cashflow::traits::CashflowProvider) trait:
/// - `build_dated_flows()` returns holder-view flows (coupons, amortization, redemptions)
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
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
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

    /// Optional EIR amortization settings for reporting schedules
    pub oid_eir: Option<OidEirSpec>,

    /// Optional call schedule (borrower callability)
    pub call_schedule: Option<LoanCallSchedule>,

    /// Settlement days (T+n). Default is 1 for leveraged loans per LSTA conventions.
    #[builder(default = 1)]
    pub settlement_days: u32,

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
    /// use finstack_valuations::instruments::fixed_income::term_loan::TermLoan;
    /// use finstack_core::currency::Currency;
    ///
    /// let loan = TermLoan::example();
    /// assert_eq!(loan.currency, Currency::USD);
    /// assert_eq!(loan.notional_limit.amount(), 10_000_000.0);
    /// ```
    pub fn example() -> Self {
        use finstack_core::dates::BusinessDayConvention;
        use finstack_core::dates::StubKind;
        use time::macros::date;
        TermLoan::builder()
            .id(InstrumentId::new("TERM-LOAN-USD-5Y"))
            .currency(Currency::USD)
            .notional_limit(Money::new(10_000_000.0, Currency::USD))
            .issue(date!(2024 - 01 - 01))
            .maturity(date!(2029 - 01 - 01))
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
            .oid_eir_opt(None)
            .call_schedule_opt(None)
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example TermLoan with valid constants should never fail")
            })
    }

    /// Resolve settlement date from `as_of` using business-day conventions when available.
    ///
    /// If `calendar_id` is set, settlement days are treated as business days on that calendar.
    /// Otherwise, a weekends-only weekday roll is used for backward compatibility.
    pub fn settlement_date(&self, as_of: Date) -> finstack_core::Result<Date> {
        if self.settlement_days == 0 {
            return Ok(as_of);
        }

        if let Some(calendar_id) = &self.calendar_id {
            let calendar = calendar_by_id(calendar_id).ok_or_else(|| {
                finstack_core::Error::Input(InputError::NotFound {
                    id: format!("calendar:{}", calendar_id),
                })
            })?;
            return as_of.add_business_days(self.settlement_days as i32, calendar);
        }

        Ok(as_of.add_weekdays(self.settlement_days as i32))
    }
}

impl TryFrom<TermLoanSpec> for TermLoan {
    type Error = finstack_core::Error;

    fn try_from(spec: TermLoanSpec) -> Result<Self, Self::Error> {
        if spec.issue >= spec.maturity {
            return Err(InputError::InvalidDateRange.into());
        }

        let TermLoanSpec {
            id,
            discount_curve_id,
            credit_curve_id,
            currency,
            notional_limit,
            issue,
            maturity,
            rate,
            pay_freq,
            day_count,
            bdc,
            calendar_id,
            stub,
            amortization,
            coupon_type,
            upfront_fee,
            ddtl,
            covenants,
            pricing_overrides,
            oid_eir,
            call_schedule,
            settlement_days,
        } = spec;

        let resolved_notional = match (notional_limit, ddtl.as_ref()) {
            (Some(limit), _) => limit,
            (None, Some(ddtl_spec)) => ddtl_spec.commitment_limit,
            (None, None) => {
                return Err(InputError::NotFound {
                    id: "notional_limit".to_string(),
                }
                .into())
            }
        };

        validate_currency(currency, resolved_notional)?;
        if let Some(fee) = upfront_fee.as_ref() {
            validate_currency(currency, *fee)?;
        }

        if let AmortizationSpec::Custom(items) = &amortization {
            for (_, amt) in items {
                validate_currency(currency, *amt)?;
            }
        }

        if let Some(cov) = &covenants {
            for sweep in &cov.cash_sweeps {
                validate_currency(currency, sweep.amount)?;
            }
        }

        if let Some(ddtl_spec) = &ddtl {
            validate_currency(currency, ddtl_spec.commitment_limit)?;
            if resolved_notional.amount() > ddtl_spec.commitment_limit.amount() {
                return Err(InputError::Invalid.into());
            }
            for draw in &ddtl_spec.draws {
                validate_currency(currency, draw.amount)?;
            }
            for step in &ddtl_spec.commitment_step_downs {
                validate_currency(currency, step.new_limit)?;
            }
            if let Some(
                super::spec::OidPolicy::WithheldAmount(m)
                | super::spec::OidPolicy::SeparateAmount(m),
            ) = &ddtl_spec.oid_policy
            {
                validate_currency(currency, *m)?;
            }
        }

        TermLoan::builder()
            .id(id)
            .currency(currency)
            .notional_limit(resolved_notional)
            .issue(issue)
            .maturity(maturity)
            .rate(rate)
            .pay_freq(pay_freq)
            .day_count(day_count)
            .bdc(bdc)
            .calendar_id_opt(calendar_id)
            .stub(stub)
            .discount_curve_id(discount_curve_id)
            .credit_curve_id_opt(credit_curve_id)
            .amortization(amortization)
            .coupon_type(coupon_type)
            .upfront_fee_opt(upfront_fee)
            .ddtl_opt(ddtl)
            .covenants_opt(covenants)
            .pricing_overrides(pricing_overrides)
            .oid_eir_opt(oid_eir)
            .call_schedule_opt(call_schedule)
            .settlement_days(settlement_days)
            .attributes(Attributes::new())
            .build()
    }
}

fn validate_currency(expected: Currency, money: Money) -> Result<(), finstack_core::Error> {
    if money.currency() != expected {
        return Err(InputError::Invalid.into());
    }
    Ok(())
}

impl crate::instruments::common_impl::traits::Instrument for TermLoan {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::TermLoan
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        // If the loan has a borrower call schedule, use tree-based pricing to capture
        // optionality with frictional exercise.
        if let Some(ref cs) = self.call_schedule {
            if !cs.calls.is_empty() {
                return crate::instruments::fixed_income::term_loan::pricing::TermLoanTreePricer::new(
                )
                .price_callable(self, curves, as_of);
            }
        }

        // Otherwise delegate to deterministic discounting pricer.
        crate::instruments::fixed_income::term_loan::pricing::TermLoanDiscountingPricer::price(
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
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.issue)
    }
}

impl crate::cashflow::traits::CashflowProvider for TermLoan {
    fn notional(&self) -> Option<finstack_core::money::Money> {
        Some(self.notional_limit)
    }

    fn build_dated_flows(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::cashflow::DatedFlows> {
        use finstack_core::cashflow::CFKind;

        // Get full internal schedule
        let schedule = crate::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
            self, curves, as_of,
        )?;

        // Filter to holder-view: only contractual inflows to a long lender.
        // Include: coupons, amortization, fees, positive notional redemptions.
        // Exclude: funding legs (negative notional draws), PIK capitalization.
        //
        // Fees (commitment, usage, facility) ARE included because they represent
        // cash inflows to the lender.  This ensures YTM (which consumes these
        // flows) captures the full economic return, consistent with YTC/YTW
        // which also include fees via the kind-aware filter in irr_helpers.
        let mut flows: Vec<(finstack_core::dates::Date, finstack_core::money::Money)> = Vec::new();

        for cf in &schedule.flows {
            match cf.kind {
                // Include coupons, interest, and all fee variants (holder receives them)
                CFKind::Fixed
                | CFKind::FloatReset
                | CFKind::Stub
                | CFKind::Fee
                | CFKind::CommitmentFee
                | CFKind::UsageFee
                | CFKind::FacilityFee => {
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
        crate::instruments::fixed_income::term_loan::cashflows::generate_cashflows(
            self, curves, as_of,
        )
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common_impl::traits::CurveDependencies for TermLoan {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        let mut builder = crate::instruments::common_impl::traits::InstrumentCurves::builder();
        builder = builder.discount(self.discount_curve_id.clone());
        if let Some(cc) = &self.credit_curve_id {
            builder = builder.credit(cc.clone());
        }
        builder.build()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::builder::specs::CouponType;
    use crate::instruments::fixed_income::term_loan::spec::CommitmentFeeBase;
    use crate::instruments::pricing_overrides::PricingOverrides;
    use finstack_core::dates::Date;
    use time::Month;

    #[test]
    fn test_term_loan_spec_conversion_plain() {
        let issue = Date::from_calendar_date(2024, Month::January, 2).expect("valid date");
        let maturity = Date::from_calendar_date(2029, Month::January, 2).expect("valid date");

        let spec = TermLoanSpec {
            id: InstrumentId::new("TL-PLAIN"),
            discount_curve_id: CurveId::new("USD-CREDIT"),
            credit_curve_id: None,
            currency: Currency::USD,
            notional_limit: Some(Money::new(5_000_000.0, Currency::USD)),
            issue,
            maturity,
            rate: RateSpec::Fixed { rate_bp: 550 },
            pay_freq: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            amortization: AmortizationSpec::None,
            coupon_type: CouponType::Cash,
            upfront_fee: None,
            ddtl: None,
            covenants: None,
            pricing_overrides: PricingOverrides::default(),
            oid_eir: None,
            call_schedule: None,
            settlement_days: 1,
        };

        let loan: TermLoan = spec.try_into().expect("conversion should succeed");
        assert_eq!(loan.notional_limit.amount(), 5_000_000.0);
        assert_eq!(loan.currency, Currency::USD);
    }

    #[test]
    fn test_term_loan_spec_conversion_ddtl_defaults_notional() {
        let issue = Date::from_calendar_date(2025, Month::March, 1).expect("valid date");
        let maturity = Date::from_calendar_date(2030, Month::March, 1).expect("valid date");
        let commitment = Money::new(12_000_000.0, Currency::USD);

        let ddtl = DdtlSpec {
            commitment_limit: commitment,
            availability_start: issue,
            availability_end: issue,
            draws: Vec::new(),
            commitment_step_downs: Vec::new(),
            usage_fee_bp: 0,
            commitment_fee_bp: 0,
            fee_base: CommitmentFeeBase::Undrawn,
            oid_policy: None,
        };

        let spec = TermLoanSpec {
            id: InstrumentId::new("TL-DDTL"),
            discount_curve_id: CurveId::new("USD-CREDIT"),
            credit_curve_id: None,
            currency: Currency::USD,
            notional_limit: None,
            issue,
            maturity,
            rate: RateSpec::Fixed { rate_bp: 450 },
            pay_freq: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            amortization: AmortizationSpec::None,
            coupon_type: CouponType::Cash,
            upfront_fee: None,
            ddtl: Some(ddtl),
            covenants: None,
            pricing_overrides: PricingOverrides::default(),
            oid_eir: None,
            call_schedule: None,
            settlement_days: 1,
        };

        let loan: TermLoan = spec.try_into().expect("conversion should succeed");
        assert_eq!(loan.notional_limit, commitment);
    }

    #[test]
    fn test_term_loan_spec_conversion_missing_notional() {
        let issue = Date::from_calendar_date(2024, Month::January, 2).expect("valid date");
        let maturity = Date::from_calendar_date(2026, Month::January, 2).expect("valid date");

        let spec = TermLoanSpec {
            id: InstrumentId::new("TL-MISSING"),
            discount_curve_id: CurveId::new("USD-CREDIT"),
            credit_curve_id: None,
            currency: Currency::USD,
            notional_limit: None,
            issue,
            maturity,
            rate: RateSpec::Fixed { rate_bp: 500 },
            pay_freq: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            amortization: AmortizationSpec::None,
            coupon_type: CouponType::Cash,
            upfront_fee: None,
            ddtl: None,
            covenants: None,
            pricing_overrides: PricingOverrides::default(),
            oid_eir: None,
            call_schedule: None,
            settlement_days: 1,
        };

        let err = TermLoan::try_from(spec).expect_err("missing notional should fail");
        match err {
            finstack_core::Error::Input(InputError::NotFound { .. }) => {}
            _ => panic!("unexpected error: {err:?}"),
        }
    }
}
