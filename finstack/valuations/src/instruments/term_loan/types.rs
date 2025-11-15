//! Term loan types and instrument trait implementation.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::spec::{AmortizationSpec, CovenantSpec, DdtlSpec, LoanCallSchedule};
use crate::cashflow::builder::specs::CouponType;
use crate::cashflow::builder::FloatingRateSpec;
use crate::instruments::common::traits::Attributes;
use crate::instruments::pricing_overrides::PricingOverrides;

/// Rate specification for term loans.
///
/// Defines whether the loan uses fixed or floating rate interest.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RateSpec {
    /// Fixed annual rate in basis points
    Fixed { rate_bp: i32 },

    /// Floating rate using canonical FloatingRateSpec.
    ///
    /// Uses the standard floating rate specification with full support
    /// for floors, caps, and gearing.
    Floating(FloatingRateSpec),
}

/// Term Loan instrument (DDTL features added via later tasks)
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
    pub pay_freq: Frequency,

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
    /// Create a canonical example term loan (fixed rate, quarterly, linear amortization).
    pub fn example() -> Self {
        use finstack_core::dates::BusinessDayConvention;
        use finstack_core::dates::StubKind;
        use time::Month;
        TermLoanBuilder::new()
            .id(InstrumentId::new("TERM-LOAN-USD-5Y"))
            .currency(Currency::USD)
            .notional_limit(Money::new(10_000_000.0, Currency::USD))
            .issue(Date::from_calendar_date(2024, Month::January, 1).unwrap())
            .maturity(Date::from_calendar_date(2029, Month::January, 1).unwrap())
            .rate(RateSpec::Fixed { rate_bp: 600 }) // 6%
            .pay_freq(Frequency::quarterly())
            .day_count(DayCount::Act360)
            .bdc(BusinessDayConvention::ModifiedFollowing)
            .calendar_id_opt(None)
            .stub(StubKind::None)
            .discount_curve_id(CurveId::new("USD-OIS"))
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
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        // Delegate to discounting pricer (deterministic v1)
        crate::instruments::term_loan::pricing::TermLoanDiscountingPricer::price(
            self, curves, as_of,
        )
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::MarketContext,
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
        )
    }
}

impl crate::cashflow::traits::CashflowProvider for TermLoan {
    fn build_schedule(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::cashflow::DatedFlows> {
        let sched =
            crate::instruments::term_loan::cashflows::generate_cashflows(self, curves, as_of)?;
        Ok(crate::instruments::term_loan::cashflows::build_dated_flows(
            &sched,
        ))
    }

    fn build_full_schedule(
        &self,
        curves: &finstack_core::market_data::MarketContext,
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

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for TermLoan {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}
